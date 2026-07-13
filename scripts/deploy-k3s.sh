#!/bin/bash
set -euo pipefail

#===============================================================================
# Boinc portal - K3S Deployment Script (Podman)
#===============================================================================
# Builds and pushes the portal image, then deploys to K3S on the VPS.
# Adapted from the AidBooks deploy script.
#
# Architecture:
#   • boinc-site   nginx + Vite site. NodePort 32087 — Cloudflare Tunnel
#                  forwards https://boinc.hideterms.com here (tunnel config
#                  is managed outside this repo).
#
# Namespace: boinc
# Registry:  beecodersregistry.azurecr.io  (reuses the existing acr-secret
#            ImagePullSecret in the cluster — copied into the namespace
#            below.)
#
# Usage:
#   ./scripts/deploy-k3s.sh [all|build|push|deploy|status|logs]
#
#   all     build → push → deploy → status (default)
#   build   build the site image locally with podman
#   push    podman login + push the image
#   deploy  scp manifests + apply on the VPS, restart the deployment
#   status  kubectl get on the namespace
#   logs    tail site logs
#
# Required tools locally:
#   podman, ssh, scp
#===============================================================================

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

COMMAND="${1:-all}"

REGISTRY="${REGISTRY:-beecodersregistry.azurecr.io}"
NAMESPACE="boinc"
SITE_IMAGE="$REGISTRY/boinc-site"

# VPS target (override via environment variables).
VPS_IP="${VPS_IP:-212.47.77.32}"
VPS_USER="${VPS_USER:-bart}"

VPS_BASE_DIR="${VPS_BASE_DIR:-~/boinc}"
VPS_K8S_DIR="$VPS_BASE_DIR/k8s"

# Single source of truth for the image tag: site/package.json. The script
# also always pushes :latest so the rolling update picks it up regardless.
APP_VERSION=$(grep -m1 '"version"' "$ROOT_DIR/site/package.json" | sed -E 's/.*"([0-9][^"]*)".*/\1/')
if [[ -z "$APP_VERSION" ]]; then
  echo "could not parse version from site/package.json"
  exit 1
fi

ssh_vps() {
  local cmd="$1"
  ssh -o StrictHostKeyChecking=accept-new "$VPS_USER@$VPS_IP" "bash -lc $(printf %q "$cmd")"
}

kubectl_vps() {
  local args="$1"
  ssh_vps "if command -v kubectl >/dev/null 2>&1; then kubectl $args; else sudo k3s kubectl $args; fi"
}

check_build_deps() {
  command -v podman >/dev/null 2>&1 || { echo "podman not found"; exit 1; }
}

check_remote_deps() {
  command -v ssh >/dev/null 2>&1 || { echo "ssh not found"; exit 1; }
  command -v scp >/dev/null 2>&1 || { echo "scp not found"; exit 1; }
}

# -----------------------------------------------------------------------------
# Build
# -----------------------------------------------------------------------------
build_site() {
  echo "==> Building $SITE_IMAGE:$APP_VERSION"
  podman build \
    --pull=newer \
    -t "$SITE_IMAGE:latest" \
    -t "$SITE_IMAGE:$APP_VERSION" \
    -f "$ROOT_DIR/site/Dockerfile" \
    "$ROOT_DIR/site"
}

# -----------------------------------------------------------------------------
# Push
# -----------------------------------------------------------------------------
push_images() {
  echo "==> Logging into $REGISTRY"
  if [[ -n "${REGISTRY_USER:-}" && -n "${REGISTRY_PASSWORD:-}" ]]; then
    podman login -u "$REGISTRY_USER" -p "$REGISTRY_PASSWORD" "$REGISTRY"
  else
    # No-op when already logged in; prompts otherwise.
    podman login --get-login "$REGISTRY" >/dev/null 2>&1 || podman login "$REGISTRY"
  fi

  echo "==> Pushing $SITE_IMAGE"
  podman push "$SITE_IMAGE:latest"
  podman push "$SITE_IMAGE:$APP_VERSION"
}

# -----------------------------------------------------------------------------
# Deploy
# -----------------------------------------------------------------------------
ensure_remote_dirs() {
  ssh_vps "mkdir -p $VPS_K8S_DIR || (command -v sudo >/dev/null 2>&1 && sudo mkdir -p $VPS_K8S_DIR && sudo chown -R $VPS_USER:$VPS_USER $VPS_BASE_DIR)"
}

copy_manifests() {
  ensure_remote_dirs
  scp -o StrictHostKeyChecking=accept-new -r \
    "$ROOT_DIR/k8s/boinc" \
    "$VPS_USER@$VPS_IP:$VPS_K8S_DIR/"
}

deploy_manifests() {
  echo "==> Deploying to $VPS_USER@$VPS_IP (namespace: $NAMESPACE)"
  copy_manifests

  # 1. Namespace must exist before anything else lands in it.
  kubectl_vps "apply -f $VPS_K8S_DIR/boinc/namespace.yaml"

  # 2. ImagePullSecret — copy the existing acr-secret into this namespace.
  ensure_acr_secret

  # 3. Service + workload.
  kubectl_vps "apply -f $VPS_K8S_DIR/boinc/service.yaml"
  kubectl_vps "apply -f $VPS_K8S_DIR/boinc/deployment.yaml"

  # 4. Force a rollout so an unchanged tag still picks up the new image
  # digest (we always push :latest with imagePullPolicy: Always).
  kubectl_vps "-n $NAMESPACE rollout restart deployment boinc-site"
  kubectl_vps "-n $NAMESPACE rollout status deployment boinc-site --timeout=120s"

  echo ""
  echo "Deployed Boinc portal v$APP_VERSION"
  echo "  • NodePort:         http://$VPS_IP:32087   (cloudflare tunnel target)"
  echo "  • Public hostname:  https://boinc.hideterms.com (once the tunnel route exists)"
  echo "  • Health:           curl -s http://$VPS_IP:32087/healthz"
}

# Copy the existing acr-secret from a namespace that has one into boinc.
# Idempotent — does nothing if it's already there. Bails with a clear
# remediation hint if no source can be found.
ensure_acr_secret() {
  if kubectl_vps "-n $NAMESPACE get secret acr-secret >/dev/null 2>&1"; then
    return 0
  fi

  local source_ns
  for source_ns in aidbooks sqail; do
    if kubectl_vps "-n $source_ns get secret acr-secret >/dev/null 2>&1"; then
      echo "==> boinc/acr-secret missing; copying from $source_ns/acr-secret"
      kubectl_vps "-n $source_ns get secret acr-secret -o yaml \
        | sed -e '/namespace:/d' -e '/resourceVersion:/d' -e '/uid:/d' -e '/creationTimestamp:/d' \
        | kubectl -n $NAMESPACE apply -f -"
      return 0
    fi
  done

  echo ""
  echo "ERROR: no source acr-secret found (looked in 'aidbooks' and 'sqail')."
  echo "       Create it manually:"
  echo "         kubectl -n $NAMESPACE create secret docker-registry acr-secret \\"
  echo "           --docker-server=$REGISTRY \\"
  echo "           --docker-username=<acr-user> \\"
  echo "           --docker-password=<acr-token>"
  exit 1
}

# -----------------------------------------------------------------------------
# Status / logs
# -----------------------------------------------------------------------------
status() {
  kubectl_vps "-n $NAMESPACE get pods,svc,deploy"
}

logs() {
  echo "==> boinc-site (last 50 lines)"
  kubectl_vps "-n $NAMESPACE logs deployment/boinc-site --tail=50" || true
}

# -----------------------------------------------------------------------------
# main
# -----------------------------------------------------------------------------
main() {
  case "$COMMAND" in
    all)
      check_build_deps
      check_remote_deps
      echo "Deploying Boinc portal v$APP_VERSION to $VPS_USER@$VPS_IP"
      build_site
      push_images
      deploy_manifests
      status
      ;;
    build)  check_build_deps; build_site ;;
    push)   check_build_deps; push_images ;;
    deploy) check_remote_deps; deploy_manifests ;;
    status) check_remote_deps; status ;;
    logs)   check_remote_deps; logs ;;
    *)
      echo "Unknown command: $COMMAND"
      echo "Usage: $0 [all|build|push|deploy|status|logs]"
      exit 1
      ;;
  esac
}

main

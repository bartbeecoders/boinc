#!/usr/bin/env bash
# Apply the optional in-cluster runner manifests on the Boinc VPS (K3s).
# Prefer install-host.sh for real Boinc CI; this is for a light/isolated agent.
#
# Usage:
#   ./runner/deploy-k3s.sh              # apply namespace + deployment
#   ./runner/deploy-k3s.sh secret       # print secret creation help
#   ./runner/deploy-k3s.sh status
#   ./runner/deploy-k3s.sh logs
#   ./runner/deploy-k3s.sh delete
set -euo pipefail

K8S_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/k8s" && pwd)"
COMMAND="${1:-apply}"

VPS_IP="${VPS_IP:-212.47.77.32}"
VPS_USER="${VPS_USER:-bart}"
NAMESPACE=gha-runner
REMOTE="~/boinc/k8s/runner"

ssh_vps() {
  ssh -o StrictHostKeyChecking=accept-new "$VPS_USER@$VPS_IP" "bash -lc $(printf %q "$1")"
}

kubectl_vps() {
  ssh_vps "if command -v kubectl >/dev/null 2>&1; then kubectl $1; else sudo k3s kubectl $1; fi"
}

case "$COMMAND" in
  secret)
    cat <<'EOF'
Create the PAT secret on the cluster (do this once):

  ssh bart@212.47.77.32
  sudo k3s kubectl create namespace gha-runner --dry-run=client -o yaml | sudo k3s kubectl apply -f -
  sudo k3s kubectl -n gha-runner create secret generic gha-runner \
    --from-literal=github_token='ghp_YOUR_TOKEN' \
    --dry-run=client -o yaml | sudo k3s kubectl apply -f -

Or copy runner/k8s/secret.example.yaml → secret.yaml, edit, then:

  scp runner/k8s/secret.yaml bart@212.47.77.32:~/boinc/k8s/runner/
  ssh bart@212.47.77.32 'sudo k3s kubectl apply -f ~/boinc/k8s/runner/secret.yaml'
EOF
    ;;
  apply | all)
    echo "==> Copying manifests to $VPS_USER@$VPS_IP"
    ssh_vps "mkdir -p $REMOTE"
    scp -o StrictHostKeyChecking=accept-new \
      "$K8S_DIR/namespace.yaml" \
      "$K8S_DIR/deployment.yaml" \
      "${VPS_USER}@${VPS_IP}:${REMOTE}/"
    if [[ -f "$K8S_DIR/secret.yaml" ]]; then
      scp -o StrictHostKeyChecking=accept-new \
        "$K8S_DIR/secret.yaml" \
        "${VPS_USER}@${VPS_IP}:${REMOTE}/secret.yaml"
    fi
    echo "==> Applying"
    kubectl_vps "apply -f $REMOTE/namespace.yaml"
    if kubectl_vps "-n $NAMESPACE get secret gha-runner >/dev/null 2>&1"; then
      echo "secret gha-runner already present"
    elif [[ -f "$K8S_DIR/secret.yaml" ]]; then
      kubectl_vps "apply -f $REMOTE/secret.yaml"
    else
      echo "WARNING: secret gha-runner missing. Run: $0 secret"
      echo "Deployment will be applied but the pod will CrashLoop until the secret exists."
    fi
    kubectl_vps "apply -f $REMOTE/deployment.yaml"
    kubectl_vps "-n $NAMESPACE rollout status deployment/gha-runner --timeout=180s" || true
    kubectl_vps "-n $NAMESPACE get pods -o wide"
    ;;
  status)
    kubectl_vps "-n $NAMESPACE get deploy,pods,secret -o wide"
    ;;
  logs)
    kubectl_vps "-n $NAMESPACE logs deploy/gha-runner -f --tail=100"
    ;;
  delete)
    kubectl_vps "delete deployment gha-runner -n $NAMESPACE --ignore-not-found"
    kubectl_vps "delete namespace $NAMESPACE --ignore-not-found"
    echo "Deleted namespace $NAMESPACE (if it existed)."
    ;;
  *)
    echo "Usage: $0 [apply|secret|status|logs|delete]"
    exit 2
    ;;
esac

#!/usr/bin/env bash
# Install (or reconfigure) a GitHub Actions self-hosted runner as a systemd
# user/system service on the VPS. Run this ON the VPS as a user with sudo.
#
# Usage:
#   ./install-host.sh              # uses ./config.env
#   ./install-host.sh /path/to/config.env
#
# Docs: runner/README.md
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="${1:-$SCRIPT_DIR/config.env}"

if [[ ! -f "$CONFIG_FILE" ]]; then
  echo "Missing config: $CONFIG_FILE"
  echo "Copy config.env.example → config.env and fill in values."
  exit 1
fi
# shellcheck disable=SC1090
source "$CONFIG_FILE"

: "${GITHUB_OWNER:?}"
: "${RUNNER_NAME:?}"
: "${RUNNER_HOME:?}"
: "${RUNNER_USER:?}"
RUNNER_SCOPE="${RUNNER_SCOPE:-repo}"
RUNNER_LABELS="${RUNNER_LABELS:-vps}"

if [[ "$RUNNER_SCOPE" == "repo" ]]; then
  : "${GITHUB_REPO:?GITHUB_REPO required when RUNNER_SCOPE=repo}"
  REPO_URL="https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}"
  API_REG_URL="https://api.github.com/repos/${GITHUB_OWNER}/${GITHUB_REPO}/actions/runners/registration-token"
else
  REPO_URL="https://github.com/${GITHUB_OWNER}"
  API_REG_URL="https://api.github.com/orgs/${GITHUB_OWNER}/actions/runners/registration-token"
fi

if [[ "$(id -un)" != "root" && "$(id -un)" != "$RUNNER_USER" ]]; then
  echo "Run as root or as RUNNER_USER=$RUNNER_USER"
  exit 1
fi

run_as_user() {
  if [[ "$(id -un)" == "$RUNNER_USER" ]]; then
    "$@"
  else
    sudo -u "$RUNNER_USER" -- "$@"
  fi
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "Missing required command: $1"
    exit 1
  }
}

need_cmd curl
need_cmd tar
need_cmd systemctl

# ---------------------------------------------------------------------------
# Registration token
# ---------------------------------------------------------------------------
resolve_token() {
  if [[ -n "${REGISTRATION_TOKEN:-}" ]]; then
    echo "$REGISTRATION_TOKEN"
    return
  fi
  if [[ -z "${GITHUB_TOKEN:-}" ]]; then
    echo "Set REGISTRATION_TOKEN or GITHUB_TOKEN in $CONFIG_FILE" >&2
    exit 1
  fi
  echo "Requesting short-lived registration token via API..." >&2
  local body
  body=$(curl -fsSL -X POST \
    -H "Accept: application/vnd.github+json" \
    -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    "$API_REG_URL")
  # Prefer jq when present; fall back to sed.
  if command -v jq >/dev/null 2>&1; then
    echo "$body" | jq -r .token
  else
    echo "$body" | sed -n 's/.*"token"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1
  fi
}

TOKEN="$(resolve_token)"
if [[ -z "$TOKEN" || "$TOKEN" == "null" ]]; then
  echo "Failed to obtain a registration token."
  exit 1
fi

# ---------------------------------------------------------------------------
# Download latest runner for this arch
# ---------------------------------------------------------------------------
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64 | amd64) RUNNER_ARCH=x64 ;;
  aarch64 | arm64) RUNNER_ARCH=arm64 ;;
  *)
    echo "Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

echo "Resolving latest actions/runner release for linux-${RUNNER_ARCH}..."
RELEASE_JSON=$(curl -fsSL https://api.github.com/repos/actions/runner/releases/latest)
if command -v jq >/dev/null 2>&1; then
  TAG=$(echo "$RELEASE_JSON" | jq -r .tag_name)
else
  TAG=$(echo "$RELEASE_JSON" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
fi
VERSION="${TAG#v}"
TARBALL="actions-runner-linux-${RUNNER_ARCH}-${VERSION}.tar.gz"
URL="https://github.com/actions/runner/releases/download/${TAG}/${TARBALL}"

echo "Installing runner ${TAG} → ${RUNNER_HOME}"
if [[ "$(id -un)" == "root" ]]; then
  mkdir -p "$RUNNER_HOME"
  chown "$RUNNER_USER:$RUNNER_USER" "$RUNNER_HOME"
else
  mkdir -p "$RUNNER_HOME"
fi

# Stop existing service if present so we can overwrite files safely.
SERVICE_NAME="actions.runner.${GITHUB_OWNER}-${GITHUB_REPO:-org}.${RUNNER_NAME}.service"
# Also try the generic pattern after config; for reinstall stop known units.
if systemctl list-unit-files 2>/dev/null | grep -q 'actions.runner'; then
  echo "Stopping any existing actions.runner units for this host..."
  systemctl list-units --type=service --all 'actions.runner.*' --no-legend 2>/dev/null \
    | awk '{print $1}' \
    | while read -r u; do
        [[ -n "$u" ]] && sudo systemctl stop "$u" || true
      done
fi

run_as_user bash -c "
  set -euo pipefail
  cd $(printf %q "$RUNNER_HOME")
  # Keep _work and _diag across reinstalls; wipe only agent bits.
  for f in config.sh run.sh bin externals svc.sh; do
    rm -rf \"\$f\"
  done
  curl -fsSL -o runner.tgz $(printf %q "$URL")
  tar xzf runner.tgz
  rm -f runner.tgz
"

# Dotnet Core runtime deps (libicu, etc.). Required on minimal Debian/Ubuntu
# images before config.sh will run. Official helper from the runner package.
if [[ -x "$RUNNER_HOME/bin/installdependencies.sh" ]]; then
  echo "Installing runner OS dependencies (libicu / .NET)..."
  sudo "$RUNNER_HOME/bin/installdependencies.sh"
fi

# ---------------------------------------------------------------------------
# Configure (non-interactive)
# ---------------------------------------------------------------------------
# Remove previous .runner registration if reconfiguring.
if [[ -f "$RUNNER_HOME/.runner" ]]; then
  echo "Removing previous runner registration..."
  run_as_user bash -c "
    cd $(printf %q "$RUNNER_HOME")
    ./config.sh remove --token $(printf %q "$TOKEN") || true
  " || true
  # If remove failed (token mismatch), force-clean local state.
  run_as_user rm -f "$RUNNER_HOME/.runner" "$RUNNER_HOME/.credentials" \
    "$RUNNER_HOME/.credentials_rsaparams" 2>/dev/null || true
  # Need a fresh token after remove consumed the old one.
  TOKEN="$(resolve_token)"
fi

echo "Configuring runner against ${REPO_URL} as '${RUNNER_NAME}'..."
CONFIG_EXTRA=()
if [[ -n "${RUNNER_WORKDIR:-}" ]]; then
  CONFIG_EXTRA+=(--work "$RUNNER_WORKDIR")
fi
# Build argv as a single safely-quoted remote command.
CONFIG_CMD=(
  ./config.sh --unattended
  --url "$REPO_URL"
  --token "$TOKEN"
  --name "$RUNNER_NAME"
  --labels "$RUNNER_LABELS"
  --replace
  "${CONFIG_EXTRA[@]}"
)
# shellcheck disable=SC2048,SC2086
if ! run_as_user bash -c "
  set -euo pipefail
  cd $(printf %q "$RUNNER_HOME")
  $(printf '%q ' "${CONFIG_CMD[@]}")
"; then
  echo
  echo "config.sh failed. Common fix on Debian/Ubuntu:"
  echo "  sudo $RUNNER_HOME/bin/installdependencies.sh"
  echo "Then re-run this script (with a fresh REGISTRATION_TOKEN if the old one expired)."
  exit 1
fi

if [[ ! -x "$RUNNER_HOME/svc.sh" ]]; then
  echo "config.sh finished but svc.sh is missing under $RUNNER_HOME — aborting service install."
  exit 1
fi

# Drop secrets from the environment of this shell.
unset TOKEN REGISTRATION_TOKEN GITHUB_TOKEN

# ---------------------------------------------------------------------------
# systemd service (root installs a system unit via svc.sh)
# ---------------------------------------------------------------------------
echo "Installing systemd service..."
if [[ "$(id -un)" == "root" ]]; then
  cd "$RUNNER_HOME"
  ./svc.sh install "$RUNNER_USER"
  ./svc.sh start
  ./svc.sh status || true
else
  # Non-root: use svc.sh with sudo for install, then enable lingering so the
  # service starts at boot without a login session.
  cd "$RUNNER_HOME"
  sudo ./svc.sh install "$RUNNER_USER"
  sudo ./svc.sh start
  sudo ./svc.sh status || true
  if command -v loginctl >/dev/null 2>&1; then
    sudo loginctl enable-linger "$RUNNER_USER" || true
  fi
fi

echo
echo "Runner installed."
echo "  URL:     $REPO_URL"
echo "  Name:    $RUNNER_NAME"
echo "  Labels:  self-hosted, Linux, X64, ${RUNNER_LABELS}"
echo "  Home:    $RUNNER_HOME"
echo
echo "Check:  sudo $RUNNER_HOME/svc.sh status"
echo "Logs:   journalctl -u 'actions.runner.*' -f"
echo "GitHub: ${REPO_URL}/settings/actions/runners"
echo
echo "Tip: remove REGISTRATION_TOKEN / GITHUB_TOKEN from $CONFIG_FILE now."

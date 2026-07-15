#!/usr/bin/env bash
# Unregister and remove the host self-hosted runner installed by install-host.sh.
# Run on the VPS.
#
# Usage:
#   ./uninstall-host.sh              # uses ./config.env
#   ./uninstall-host.sh /path/to/config.env
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="${1:-$SCRIPT_DIR/config.env}"

if [[ ! -f "$CONFIG_FILE" ]]; then
  echo "Missing config: $CONFIG_FILE"
  exit 1
fi
# shellcheck disable=SC1090
source "$CONFIG_FILE"

: "${RUNNER_HOME:?}"
: "${RUNNER_USER:?}"
: "${GITHUB_OWNER:?}"
RUNNER_SCOPE="${RUNNER_SCOPE:-repo}"

if [[ "$RUNNER_SCOPE" == "repo" ]]; then
  : "${GITHUB_REPO:?}"
  API_REG_URL="https://api.github.com/repos/${GITHUB_OWNER}/${GITHUB_REPO}/actions/runners/registration-token"
else
  API_REG_URL="https://api.github.com/orgs/${GITHUB_OWNER}/actions/runners/registration-token"
fi

run_as_user() {
  if [[ "$(id -un)" == "$RUNNER_USER" ]]; then
    "$@"
  else
    sudo -u "$RUNNER_USER" -- "$@"
  fi
}

resolve_token() {
  if [[ -n "${REGISTRATION_TOKEN:-}" ]]; then
    echo "$REGISTRATION_TOKEN"
    return
  fi
  if [[ -z "${GITHUB_TOKEN:-}" ]]; then
    echo "Set REGISTRATION_TOKEN or GITHUB_TOKEN to unregister cleanly."
    echo "Continuing with local service removal only..."
    return 1
  fi
  body=$(curl -fsSL -X POST \
    -H "Accept: application/vnd.github+json" \
    -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    "$API_REG_URL")
  if command -v jq >/dev/null 2>&1; then
    echo "$body" | jq -r .token
  else
    echo "$body" | sed -n 's/.*"token"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1
  fi
}

if [[ -d "$RUNNER_HOME" && -x "$RUNNER_HOME/svc.sh" ]]; then
  echo "Stopping and uninstalling systemd service..."
  (cd "$RUNNER_HOME" && sudo ./svc.sh stop || true)
  (cd "$RUNNER_HOME" && sudo ./svc.sh uninstall || true)
fi

if TOKEN=$(resolve_token) && [[ -n "$TOKEN" && "$TOKEN" != "null" && -x "$RUNNER_HOME/config.sh" ]]; then
  echo "Unregistering from GitHub..."
  run_as_user bash -c "
    cd $(printf %q "$RUNNER_HOME")
    ./config.sh remove --token $(printf %q "$TOKEN") || true
  "
else
  echo "Skipped GitHub unregister (no token or no config.sh)."
fi

read -r -p "Delete install directory $RUNNER_HOME ? [y/N] " ans
if [[ "${ans,,}" == "y" || "${ans,,}" == "yes" ]]; then
  sudo rm -rf "$RUNNER_HOME"
  echo "Removed $RUNNER_HOME"
else
  echo "Left $RUNNER_HOME in place."
fi

echo "Done. Confirm in GitHub → Settings → Actions → Runners that the agent is gone."

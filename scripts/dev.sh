#!/usr/bin/env bash
# Start the Boinc tray app together with the portal dev server (Vite, hot
# reload — URL printed below). Extra args go to the app, e.g.
# scripts/dev.sh --some-flag. Quitting the app stops the dev server too.
set -euo pipefail
cd "$(dirname "$0")/.."

(
    cd site
    [ -d node_modules ] || npm install
    exec node_modules/.bin/vite
) &
VITE_PID=$!
cleanup() { kill "$VITE_PID" 2>/dev/null || true; }
trap cleanup EXIT
trap 'cleanup; exit 130' INT TERM

cargo run -p boinc-app -- "$@"

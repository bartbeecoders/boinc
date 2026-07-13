#!/usr/bin/env bash
# Start the Boinc tray app for development. Extra args go to the app,
# e.g. scripts/dev.sh --some-flag
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo run -p boinc-app -- "$@"

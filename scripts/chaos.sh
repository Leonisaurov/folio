#!/usr/bin/env bash
# Suite de caos de datos.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"$SCRIPT_DIR/chaos-save.sh"
"$SCRIPT_DIR/chaos-move.sh"

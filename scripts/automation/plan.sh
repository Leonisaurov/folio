#!/usr/bin/env bash
# Valida la coherencia del plan (sin requerir Rust ni GTK).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

require_cmd python3
cd "$ROOT"
exec python3 scripts/check_plan.py

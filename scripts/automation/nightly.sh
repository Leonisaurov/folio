#!/usr/bin/env bash
# Suite desatendida: pantalla puede apagarse, la máquina no debe suspenderse.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

ARTIFACTS="$(new_artifacts_dir)"
LOG="$ARTIFACTS/nightly.log"

log "Suite desatendida. Log: $LOG"
log "La pantalla puede apagarse; la suspensión queda bloqueada."

set +e
"$SCRIPT_DIR/keep-awake.sh" -- "$SCRIPT_DIR/all.sh" 2>&1 | tee -a "$LOG"
status=${PIPESTATUS[0]}
set -e

printf '\nexit: %s\n' "$status" >>"$LOG"
if [[ $status -ne 0 ]]; then
    warn "La suite falló (salida $status). Revisa $LOG"
fi
exit "$status"

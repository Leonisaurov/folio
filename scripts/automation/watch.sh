#!/usr/bin/env bash
# Re-ejecuta la suite rápida al cambiar archivos.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

if ! have inotifywait; then
    die "Falta inotifywait. En Arch: sudo pacman -S inotify-tools"
fi

cd "$ROOT"
log "Ejecución inicial..."
"$SCRIPT_DIR/all.sh" --fast || true

log "Observando cambios (Ctrl+C para salir)..."
inotifywait -m -r \
    -e close_write,create,delete,move \
    --exclude '(^|/)(\.artifacts|target|\.git|build-dir)(/|$)' \
    --format '%w%f %e' \
    "$ROOT" |
while read -r _path _event; do
    printf '\n==> Cambio detectado; re-ejecutando...\n'
    "$SCRIPT_DIR/all.sh" --fast || true
done

#!/usr/bin/env bash
# Ejecuta un comando sin permitir que la máquina se suspenda.
# La pantalla puede apagarse: no se inhibe "idle".
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

LID=0
if [[ "${1:-}" == "--lid" ]]; then
    LID=1
    shift
fi
if [[ "${1:-}" == "--" ]]; then shift; fi
if [[ $# -eq 0 ]]; then
    cat >&2 <<'EOF'
Uso: keep-awake.sh [--lid] -- <comando> [args...]

  (sin --lid)  bloquea solo la suspensión por inactividad.
  --lid        bloquea también la suspensión al cerrar la tapa.
               Úsalo solo con el portátil ventilado y conectado a corriente.

La pantalla puede apagarse; este script no inhibe el idle.
EOF
    exit 2
fi

if ! have systemd-inhibit; then
    warn "systemd-inhibit no disponible; se ejecuta sin bloqueo de suspensión."
    exec "$@"
fi

what="sleep"
if [[ $LID -eq 1 ]]; then what="sleep:handle-lid-switch"; fi

log "Bloqueando suspensión ($what); la pantalla puede apagarse."
log "Comprobación: systemd-inhibit --list | grep -i folio"

exec systemd-inhibit \
    --what="$what" \
    --who="Folio automation" \
    --why="Ejecución desatendida de tests/build de Folio" \
    --mode=block \
    -- "$@"

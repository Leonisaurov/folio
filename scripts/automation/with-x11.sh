#!/usr/bin/env bash
# Ejecuta un comando en un X11 privado, con D-Bus y XDG temporales.
# Nunca reutiliza el DISPLAY de la sesión del usuario.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

if [[ $# -eq 0 || "$1" == "--" && $# -eq 1 ]]; then
    cat >&2 <<'EOF'
Uso: with-x11.sh [--keep] -- <comando> [args...]

Ejemplos:
  ./scripts/automation/with-x11.sh -- xdpyinfo
  ./scripts/automation/with-x11.sh -- cargo test -p folio-ui
EOF
    exit 2
fi

if [[ "${1:-}" == "--keep" ]]; then
    export FOLIO_TEST_KEEP=1
    shift
fi
if [[ "${1:-}" == "--" ]]; then shift; fi
[[ $# -gt 0 ]] || die "No hay comando que ejecutar."

require_cmd Xvfb
require_cmd xvfb-run
require_cmd dbus-run-session

ARTIFACTS="$(new_artifacts_dir)"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/folio-x11.XXXXXX")"

cleanup() {
    if [[ "${FOLIO_TEST_KEEP:-0}" == "1" ]]; then
        warn "Entorno temporal conservado: $TMP"
    else
        rm -rf "$TMP"
    fi
}
trap cleanup EXIT

mkdir -p \
    "$TMP/home" "$TMP/config" "$TMP/cache" "$TMP/data" "$TMP/state" \
    "$TMP/runtime" "$TMP/tmp" "$TMP/gst" "$TMP/vault" "$ARTIFACTS/screenshots"
chmod 700 "$TMP/runtime"

export HOME="$TMP/home"
export XDG_CONFIG_HOME="$TMP/config"
export XDG_CACHE_HOME="$TMP/cache"
export XDG_DATA_HOME="$TMP/data"
export XDG_STATE_HOME="$TMP/state"
export XDG_RUNTIME_DIR="$TMP/runtime"
export TMPDIR="$TMP/tmp"
export XDG_SESSION_TYPE=x11
export GDK_BACKEND=x11
export LIBGL_ALWAYS_SOFTWARE=1
export GSETTINGS_BACKEND=memory
export GST_REGISTRY="$TMP/gst/registry.bin"
export LANG="C.UTF-8"
export LC_ALL="C.UTF-8"
export TZ="${TZ:-UTC}"
export FOLIO_TEST=1
export FOLIO_TEST_OFFLINE=1
export FOLIO_TEST_ARTIFACTS="$ARTIFACTS"
export FOLIO_TEST_VAULT="$TMP/vault"
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"
if [[ "${FOLIO_TEST_A11Y:-0}" == "1" ]]; then
    export NO_AT_BRIDGE=0
else
    export NO_AT_BRIDGE=1
fi

# Garantiza que no se herede el display ni el bus del usuario.
unset DISPLAY WAYLAND_DISPLAY XAUTHORITY DBUS_SESSION_BUS_ADDRESS

{
    printf 'command:'
    printf ' %q' "$@"
    printf '\n'
    printf 'artifacts: %s\n' "$ARTIFACTS"
    printf 'home: %s\n' "$HOME"
    printf 'runtime: %s\n' "$XDG_RUNTIME_DIR"
    printf 'backend: x11 (Xvfb privado)\n'
    printf 'started: %s\n' "$(date -Is)"
} >"$ARTIFACTS/env.txt"

{
    printf '%q ' "$@"
    printf '\n'
} >"$ARTIFACTS/command.txt"

log "Ejecutando en Xvfb privado (sin usar el display del usuario)"
log "Artefactos: $ARTIFACTS"
log "Comando: $*"

set +e
xvfb-run -a -s "-screen 0 1440x900x24 -nolisten tcp" \
    dbus-run-session -- "$@" 2>&1 | tee "$ARTIFACTS/command.log"
status=${PIPESTATUS[0]}
set -e

printf '\n' >>"$ARTIFACTS/command.log"
printf 'exit: %s\n' "$status" >>"$ARTIFACTS/env.txt"

if [[ $status -ne 0 ]]; then
    warn "Falló el comando (salida $status). Log: $ARTIFACTS/command.log"
fi
exit "$status"

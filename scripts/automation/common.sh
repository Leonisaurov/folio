#!/usr/bin/env bash
# Utilidades comunes de automatización de Folio.
set -euo pipefail

COMMON_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$COMMON_DIR/../.." && pwd)"
ARTIFACTS_ROOT="$ROOT/.artifacts"

log()  { printf '==> %s\n' "$*"; }
warn() { printf 'AVISO: %s\n' "$*" >&2; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

have() { command -v "$1" >/dev/null 2>&1; }

require_cmd() {
    have "$1" || die "Falta '$1'. Instálalo antes de continuar."
}

has_cargo() { [[ -f "$ROOT/Cargo.toml" ]]; }

require_cargo() {
    has_cargo || die "No existe Cargo.toml; este paso aún no aplica (etapa E0)."
    require_cmd cargo
}

new_artifacts_dir() {
    local stamp
    stamp="$(date +%Y%m%d-%H%M%S)-$$"
    mkdir -p "$ARTIFACTS_ROOT/$stamp"
    printf '%s\n' "$ARTIFACTS_ROOT/$stamp"
}

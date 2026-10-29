#!/usr/bin/env bash
# Tests de dominio, store, layout, config y plugins. Sin GTK ni display.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

if ! has_cargo; then
    log "Sin Cargo.toml: tests de unidad omitidos (etapa de planificación)."
    exit 0
fi
require_cmd cargo

cd "$ROOT"

packages=()
for pkg in folio-core folio-store folio-config folio-daily folio-layout folio-render folio-plugins; do
    if [[ -f "crates/$pkg/Cargo.toml" ]]; then
        packages+=(-p "$pkg")
    fi
done

if [[ ${#packages[@]} -eq 0 ]]; then
    log "No hay crates de dominio todavía; nada que probar."
    exit 0
fi

log "cargo test ${packages[*]}"
exec cargo test "${packages[@]}" --all-features

#!/usr/bin/env bash
# Tests de UI/GTK dentro de un Xvfb privado. Nunca usa el display del usuario.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

if ! has_cargo; then
    log "Sin Cargo.toml: tests de UI omitidos (etapa de planificación)."
    exit 0
fi
require_cmd cargo

packages=()
if [[ -f "$ROOT/crates/folio-ui/Cargo.toml" ]]; then packages+=(-p folio-ui); fi
if [[ -f "$ROOT/crates/folio-app/Cargo.toml" ]]; then packages+=(-p folio-app); fi
if [[ ${#packages[@]} -eq 0 ]]; then
    log "No hay crates de UI todavía; nada que probar."
    exit 0
fi

feature_args=()
if [[ -n "${FOLIO_TEST_FEATURES:-}" ]]; then
    read -r -a feature_args <<<"$FOLIO_TEST_FEATURES"
fi

has_separator=0
for arg in "$@"; do
    [[ "$arg" == "--" ]] && has_separator=1
done

run_args=("$@")
if [[ $has_separator -eq 0 ]]; then
    run_args+=(-- --test-threads=1)
fi

cd "$ROOT"
exec "$SCRIPT_DIR/with-x11.sh" -- \
    cargo test "${packages[@]}" "${feature_args[@]}" "${run_args[@]}"

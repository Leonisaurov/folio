#!/usr/bin/env bash
# Formato, lints y auditoría de dependencias.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

if ! has_cargo; then
    log "Sin Cargo.toml: calidad de código omitida (etapa de planificación)."
    exit 0
fi
require_cmd cargo

cd "$ROOT"

log "cargo fmt --check"
cargo fmt --all -- --check

log "cargo clippy"
cargo clippy --workspace --all-targets --all-features -- -D warnings

if [[ -f deny.toml ]] && have cargo-deny; then
    log "cargo deny"
    cargo deny check
else
    warn "cargo-deny no instalado o sin deny.toml; se omite."
fi

if have cargo-audit; then
    log "cargo audit"
    cargo audit
else
    warn "cargo-audit no instalado; se omite."
fi

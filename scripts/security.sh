#!/usr/bin/env bash
# Pruebas de seguridad: sandbox de plugins, rutas, HTML y auditoría.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd cargo

log "Sandbox y permisos de plugins"
cargo test -q -p folio-plugins

log "Rutas de assets y validación de URL"
cargo test -q -p folio-store assets
cargo test -q -p folio-core bad_url_and_mailto

log "Exportación sin scripts"
cargo test -q -p folio-render html_escapes_text_and_links
cargo test -q -p folio-render html_is_self_contained_and_has_no_scripts

if have cargo-audit; then
    log "cargo audit"
    cargo audit
else
    warn "cargo-audit no instalado; se omite la auditoría de advisories."
fi

if have cargo-deny; then
    log "cargo deny"
    cargo deny check
else
    warn "cargo-deny no instalado; se omite."
fi

log "Seguridad OK (revisar avisos omitidos en CI completa)"

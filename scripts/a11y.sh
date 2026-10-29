#!/usr/bin/env bash
# Auditoría automatizada de accesibilidad con escala 200 % y puente a11y.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd cargo
export GDK_DPI_SCALE=2
export GDK_SCALE=1
export FOLIO_TEST_A11Y=1
log "Accesibilidad: foco, etiquetas, tema, escala 200 % y alternativa al lienzo"
"$SCRIPT_DIR/automation/with-x11.sh" -- \
    cargo test -p folio-ui --test a11y -- --test-threads=1 --nocapture

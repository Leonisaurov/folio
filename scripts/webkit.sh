#!/usr/bin/env bash
# Pruebas de aislamiento de la vista web viva (WebKitGTK 6.0).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd cargo
if ! pkg-config --exists webkitgtk-6.0; then
    die "Falta webkitgtk-6.0 (Arch: webkitgtk-6.0; Debian: libwebkitgtk-6.0-dev)."
fi
if [[ "${FOLIO_TEST_NO_WEBKIT_SANDBOX:-0}" == "1" ]]; then
    warn "WebKit sin sandbox para este test (runner sin userns); el sandbox real se prueba en local."
    export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
fi
log "Compilando y probando la vista web efímera"
"$SCRIPT_DIR/automation/with-x11.sh" -- \
    cargo test -p folio-ui --features webkit --test webkit -- --test-threads=1 --nocapture

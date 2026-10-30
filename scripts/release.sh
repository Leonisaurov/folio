#!/usr/bin/env bash
# Release reproducible: build, checksums, tarball y (opcional) firma.
# Uso: release.sh [--dry-run]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd cargo

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
DRY=0
[[ "${1:-}" == "--dry-run" ]] && DRY=1

log "Folio $VERSION"
log "Plan + calidad + tests"
./scripts/automation/all.sh --fast

log "Build release"
FEATURE_ARGS=()
if pkg-config --exists webkitgtk-6.0; then
    log "WebKitGTK 6.0 detectado: se incluye la vista web viva"
    FEATURE_ARGS=(--features webkit)
else
    warn "WebKitGTK 6.0 no disponible: release sin vista web viva."
fi
cargo build --release -p folio-app "${FEATURE_ARGS[@]}"
cargo build --release -p folio-tools

DIST="$ROOT/dist"
NAME="folio-$VERSION-linux-x86_64"
mkdir -p "$DIST/$NAME/bin"
cp target/release/folio "$DIST/$NAME/bin/"
cp target/release/folio-tools "$DIST/$NAME/bin/"
cp README.md LICENSE CHANGELOG.md "$DIST/$NAME/"
cp -r docs "$DIST/$NAME/"
cp -r plugins "$DIST/$NAME/"
tar -C "$DIST" -czf "$DIST/$NAME.tar.gz" "$NAME"
( cd "$DIST" && sha256sum "$NAME.tar.gz" >"$NAME.tar.gz.sha256" )

if [[ -n "${FOLIO_SIGNING_KEY:-}" ]] && have gpg; then
    log "Firmando con $FOLIO_SIGNING_KEY"
    gpg --armor --detach-sign --local-user "$FOLIO_SIGNING_KEY" "$DIST/$NAME.tar.gz"
else
    warn "Sin FOLIO_SIGNING_KEY/gpg: el artefacto queda SIN FIRMA."
fi

log "Artefactos en $DIST"
ls -lh "$DIST" | tail -5
if [[ $DRY -eq 1 ]]; then
    log "Dry-run: no se publica nada."
fi

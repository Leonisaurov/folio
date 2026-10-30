#!/usr/bin/env bash
# Prepara el canal beta local: checks, .deb, artefactos y metadatos sin telemetría.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd cargo
require_cmd sha256sum

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
CHANNEL_DIR="$ROOT/dist/beta/$VERSION"
mkdir -p "$CHANNEL_DIR"

log "Checks de beta"
"$SCRIPT_DIR/beta-check.sh"

log "Construyendo .deb de beta"
"$SCRIPT_DIR/build-deb.sh"
DEB="$(ls -1 "$ROOT/dist"/folio_*_*.deb | tail -1)"
cp "$DEB" "$CHANNEL_DIR/"
cp "$ROOT/dist/beta-check.txt" "$CHANNEL_DIR/REPORT.txt"
cp "$ROOT/docs/20-beta-30-dias.md" "$CHANNEL_DIR/CHECKLIST.md"
cp "$ROOT/docs/23-beta-github.md" "$CHANNEL_DIR/GITHUB-BETA.md"
cp "$ROOT/keys/folio-release-key.asc" "$CHANNEL_DIR/folio-release-key.asc"

log "Empaquetando binarios y checksums"
FEATURE_ARGS=()
if pkg-config --exists webkitgtk-6.0; then FEATURE_ARGS=(--features webkit); fi
cargo build --release -q -p folio-app "${FEATURE_ARGS[@]}"
cargo build --release -q -p folio-tools
tar -C "$ROOT/target/release" -czf "$CHANNEL_DIR/folio-$VERSION-linux-x86_64.tar.gz" folio folio-tools
( cd "$CHANNEL_DIR" && sha256sum ./*.deb ./*.tar.gz > SHA256SUMS )
cat >"$CHANNEL_DIR/CHANNEL.json" <<EOF
{
  "channel": "beta",
  "version": "$VERSION",
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "telemetry": false,
  "schema": 1,
  "checklist": "CHECKLIST.md",
  "report": "REPORT.txt",
  "artifacts": [
    "$(basename "$DEB")",
    "folio-$VERSION-linux-x86_64.tar.gz"
  ]
}
EOF
if [[ -n "${FOLIO_SIGNING_KEY:-}" ]] && have gpg; then
    "$SCRIPT_DIR/sign-artifacts.sh"
else
    warn "Beta SIN FIRMA: define FOLIO_SIGNING_KEY para firmar los artefactos."
fi
log "Canal beta preparado en $CHANNEL_DIR"
ls -lh "$CHANNEL_DIR"

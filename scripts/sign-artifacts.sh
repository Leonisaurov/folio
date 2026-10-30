#!/usr/bin/env bash
# Firma los artefactos de release y genera SHA256SUMS firmado.
# Uso: FOLIO_SIGNING_KEY=<huella|key-id> ./scripts/sign-artifacts.sh
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd gpg
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
KEY="${FOLIO_SIGNING_KEY:-}"
[[ -n "$KEY" ]] || die "Define FOLIO_SIGNING_KEY con la huella de tu clave."
gpg --list-secret-keys --list-options show-only-fpr-mbox "$KEY" >/dev/null 2>&1 \
    || die "No hay clave secreta para '$KEY' en este GNUPGHOME."

FILES=$(find "$ROOT/dist" -maxdepth 2 -type f \
    \( -name "folio-$VERSION-linux-x86_64.tar.gz" \
       -o -name "folio_${VERSION}_*.deb" \
       -o -name "folio-${VERSION}-*.pkg.tar.zst" \
       -o -name "RELEASE-$VERSION.md" \) -print | sort)
[[ -n "$FILES" ]] || die "No hay artefactos; ejecuta release.sh y build-deb.sh."

log "Firmando $VERSION con $KEY"
while IFS= read -r file; do
    [[ -n "$file" ]] || continue
    gpg --yes --armor --detach-sign --local-user "$KEY" "$file"
    gpg --verify "$file.asc" "$file" >/dev/null
    log "OK $(basename "$file").asc"
done <<<"$FILES"

( cd "$ROOT/dist" && sha256sum $(find . -maxdepth 2 -type f \
    \( -name "folio-$VERSION-linux-x86_64.tar.gz" \
       -o -name "folio_${VERSION}_*.deb" \
       -o -name "folio-${VERSION}-*.pkg.tar.zst" \
       -o -name "RELEASE-$VERSION.md" \) -print | sed 's#^\./##' | sort) >SHA256SUMS )
gpg --yes --armor --detach-sign --local-user "$KEY" "$ROOT/dist/SHA256SUMS"
gpg --verify "$ROOT/dist/SHA256SUMS.asc" "$ROOT/dist/SHA256SUMS" >/dev/null
log "Firmas y SHA256SUMS listos en dist/"
log "Clave pública: gpg --armor --export $KEY > folio-release-key.asc"

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
sign_one() {
    local file="$1"
    gpg --yes --armor --detach-sign --local-user "$KEY" "$file"
    gpg --verify "$file.asc" "$file" >/dev/null
    log "OK $(basename "$file").asc"
}
while IFS= read -r file; do
    [[ -n "$file" ]] || continue
    sign_one "$file"
done <<<"$FILES"

( cd "$ROOT/dist" && sha256sum $(find . -maxdepth 2 -type f \
    \( -name "folio-$VERSION-linux-x86_64.tar.gz" \
       -o -name "folio_${VERSION}_*.deb" \
       -o -name "folio-${VERSION}-*.pkg.tar.zst" \
       -o -name "RELEASE-$VERSION.md" \) -print | sed 's#^\./##' | sort) >SHA256SUMS )
sign_one "$ROOT/dist/SHA256SUMS"
# Firmar también el canal beta si existe.
for beta_dir in "$ROOT"/dist/beta/*; do
    [[ -d "$beta_dir" ]] || continue
    for file in "$beta_dir"/*.deb "$beta_dir"/*.tar.gz "$beta_dir"/SHA256SUMS "$beta_dir"/CHANNEL.json; do
        [[ -f "$file" ]] && sign_one "$file"
    done
done
log "Firmas y SHA256SUMS listos en dist/"
log "Clave pública: gpg --armor --export $KEY > folio-release-key.asc"

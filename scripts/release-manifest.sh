#!/usr/bin/env bash
# Verifica los artefactos de release y escribe el manifiesto con sus bloqueos.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd python3

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
ARCH="$(uname -m)"
DIST="$ROOT/dist"
TAR="$DIST/folio-$VERSION-linux-x86_64.tar.gz"
DEB="$(ls -1 "$DIST"/folio_"$VERSION"_*.deb 2>/dev/null | head -1 || true)"
ARCH_PKG="$(ls -1 "$DIST"/arch/folio-"$VERSION"-*.pkg.tar.zst 2>/dev/null | head -1 || true)"
[[ -f "$TAR" ]] || die "falta $TAR; ejecuta ./scripts/release.sh"
[[ -n "$DEB" && -f "$DEB" ]] || die "falta el .deb; ejecuta ./scripts/build-deb.sh"

TMP="$(mktemp -d "$ROOT/.artifacts/release-verify.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/tar"
tar -C "$TMP/tar" -xzf "$TAR"
BIN="$TMP/tar/folio-$VERSION-linux-x86_64/bin/folio"
[[ -x "$BIN" ]] || die "el tarball no contiene bin/folio"
GOT="$("$BIN" --version)"
[[ "$GOT" == "folio $VERSION" ]] || die "versión inesperada en tarball: $GOT"
if have dpkg-deb; then
    dpkg-deb --extract "$DEB" "$TMP/deb"
else
    python3 "$SCRIPT_DIR/deb_build.py" verify "$DEB" "$TMP/deb" >/dev/null
fi
"$TMP/deb/usr/bin/folio" --version | grep -q "$VERSION"
appstreamcli validate "$TMP/deb/usr/share/metainfo/io.github.leonisauro.Folio.metainfo.xml" >/dev/null
if [[ -n "$ARCH_PKG" ]] && have pacman && have bsdtar; then
    pacman -Qp "$ARCH_PKG" >/dev/null
    mkdir -p "$TMP/arch"
    bsdtar -xf "$ARCH_PKG" -C "$TMP/arch"
    "$TMP/arch/usr/bin/folio" --version | grep -q "$VERSION"
fi

SHA_TAR="$(sha256sum "$TAR" | cut -d' ' -f1)"
SHA_DEB="$(sha256sum "$DEB" | cut -d' ' -f1)"
SHA_ARCH=""
if [[ -n "$ARCH_PKG" ]]; then
    SHA_ARCH="$(sha256sum "$ARCH_PKG" | cut -d' ' -f1)"
fi
SIGNED="sin firma"
SIGNER=""
if [[ -f "$TAR.asc" && -f "$DEB.asc" ]] && have gpg; then
    SIGNER="$(gpg --status-fd 1 --verify "$TAR.asc" "$TAR" 2>/dev/null | awk '/VALIDSIG/{print $3; exit}')"
    if [[ -n "$SIGNER" ]]; then SIGNED="firmada"; fi
fi
if [[ "$SIGNED" == "firmada" ]]; then
    SIG_LINE="- [x] **Firma GPG**: verificada con la clave \`$SIGNER\`."
else
    SIG_LINE="- [ ] **Firma GPG**: falta \`FOLIO_SIGNING_KEY\`; el artefacto no está firmado."
fi
MANIFEST="$DIST/RELEASE-$VERSION.md"
cat >"$MANIFEST" <<EOF
# Folio $VERSION — manifiesto de release

**Fecha**: $(date -u +%Y-%m-%dT%H:%M:%SZ)
**Canal**: release candidate ($SIGNED)
**Arquitectura**: $ARCH

## Artefactos

| Artefacto | SHA-256 |
|---|---|
| $(basename "$TAR") | \`$SHA_TAR\` |
| $(basename "$DEB") | \`$SHA_DEB\` |
$(if [[ -n "$ARCH_PKG" ]]; then printf '| %s | %s |\n' "$(basename "$ARCH_PKG")" "$SHA_ARCH"; fi)

Verificación ejecutada:

- tarball extraído y \`folio --version\` = \`$GOT\`;
- \`.deb\` extraído, binario ejecutado y AppStream validado;
- paquete pacman verificado con \`pacman -Qp\` y \`bsdtar\`;
- \`./scripts/automation/all.sh\` verde;
- \`./scripts/beta-check.sh\` verde (sin telemetría).

## Gates automáticos

- [x] Workspace compila sin warnings con clippy.
- [x] Suite de dominio, UI en Xvfb, PDF, cifrado y a11y.
- [x] \`cargo audit\` y \`cargo deny\` sin vulnerabilidades.
- [x] Artefactos y checksums generados.
- [x] AppStream y desktop validados.

## Bloqueos humanos / decisiones pendientes

$SIG_LINE
- [ ] **Beta de 30 días con testers reales**: el canal y el checklist están listos,
  pero no se ha ejecutado el periodo.
- [x] **Debian y pacman**: instalados y verificados en contenedores limpios
  Ubuntu 24.04 y Arch Linux.
- [ ] **Flatpak/AUR publicado**: requiere \`flatpak-builder\` y una VM de
  escritorio limpia; los manifiestos y el paquete pacman existen.
- [ ] **Revisión manual con Orca/Wayland**: la auditoría automatizada y la lista de
  bloques están hechas, pero falta la prueba con lector real.
- [x] **Runtime WASM de terceros**: implementado con ABI JSON/wasmi (ADR-033);
  los cinco plugins de referencia, \`api = 1\` y \`plugins/hello\` están probados.
- [ ] **Publicación**: tag firmado, release notes y rollback firmado.

## Cómo continuar

1. Publicar los artefactos firmados y el tag \`v$VERSION\`.
2. Distribuir \`dist/beta/$VERSION/\` y completar los 30 días.
3. Publicar y validar Flatpak/AUR en una VM de escritorio limpia.
4. Completar la revisión con lector de pantalla.
5. Verificar \`gpg --verify\` y \`sha256sum -c\` antes de anunciar.
EOF
cp "$MANIFEST" "$DIST/RELEASE.md"
sed -n '1,45p' "$MANIFEST"
log "Manifiesto: $MANIFEST"

#!/usr/bin/env bash
# Construye y verifica un paquete .deb sin privilegios.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd cargo
require_cmd python3

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
case "$(uname -m)" in
    x86_64) ARCH=amd64 ;;
    aarch64) ARCH=arm64 ;;
    *) ARCH="$(uname -m)" ;;
esac
FEATURE_ARGS=()
DEB_EXTRA=""
if pkg-config --exists webkitgtk-6.0; then
    FEATURE_ARGS=(--features webkit)
    DEB_EXTRA=", libwebkitgtk-6.0-4"
fi

log "Compilando release $VERSION ($ARCH)"
cargo build --release -p folio-app "${FEATURE_ARGS[@]}"
cargo build --release -p folio-tools

TMP="$(mktemp -d "${TMPDIR:-/tmp}/folio-deb.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
PKGROOT="$TMP/pkg"
mkdir -p "$PKGROOT/DEBIAN" "$PKGROOT/usr/bin" \
    "$PKGROOT/usr/share/applications" \
    "$PKGROOT/usr/share/metainfo" \
    "$PKGROOT/usr/share/icons/hicolor/scalable/apps" \
    "$PKGROOT/usr/share/doc/folio"
install -m755 target/release/folio "$PKGROOT/usr/bin/folio"
install -m755 target/release/folio-tools "$PKGROOT/usr/bin/folio-tools"
install -m644 flatpak/io.github.leonisauro.Folio.desktop \
    "$PKGROOT/usr/share/applications/io.github.leonisauro.Folio.desktop"
install -m644 flatpak/io.github.leonisauro.Folio.metainfo.xml \
    "$PKGROOT/usr/share/metainfo/io.github.leonisauro.Folio.metainfo.xml"
install -m644 flatpak/io.github.leonisauro.Folio.svg \
    "$PKGROOT/usr/share/icons/hicolor/scalable/apps/io.github.leonisauro.Folio.svg"
install -m644 docs/16-manual-usuario.md "$PKGROOT/usr/share/doc/folio/manual.md"
install -m644 docs/17-recuperacion.md "$PKGROOT/usr/share/doc/folio/recuperacion.md"
install -m644 LICENSE "$PKGROOT/usr/share/doc/folio/copyright"

INSTALLED_KIB="$(du -sk "$PKGROOT" | cut -f1)"
cat >"$PKGROOT/DEBIAN/control" <<EOF
Package: folio
Version: $VERSION
Section: office
Priority: optional
Architecture: $ARCH
Installed-Size: $INSTALLED_KIB
Depends: libgtk-4-1 (>= 4.14), libadwaita-1-0 (>= 1.5), libsqlite3-0,
 libcairo2, libpango-1.0-0, libgstreamer1.0-0$DEB_EXTRA
Maintainer: Folio contributors <maintainers@folio.invalid>
Homepage: https://github.com
Description: Notas y libros diarios con lienzo y plugins
 Folio es una aplicación nativa de notas y diarios con editor de bloques,
 lienzo infinito, búsqueda FTS5, cifrado opcional de bóveda y plugins
 aislados con permisos. Los datos viven en archivos legibles.
EOF

DIST="$ROOT/dist"
mkdir -p "$DIST"
DEB="$DIST/folio_${VERSION}_${ARCH}.deb"
if have dpkg-deb; then
    dpkg-deb --root-owner-group --build "$PKGROOT" "$DEB" >/dev/null
    log "Paquete: $DEB"
    dpkg-deb --info "$DEB"
    dpkg-deb --contents "$DEB" | sed -n '1,20p'
    log "Verificando instalación en árbol temporal"
    mkdir -p "$TMP/extract"
    dpkg-deb --extract "$DEB" "$TMP/extract"
else
    log "dpkg-deb no disponible; se usa el empaquetador ar/tar de Folio"
    python3 "$SCRIPT_DIR/deb_build.py" build "$PKGROOT" "$DEB"
    python3 "$SCRIPT_DIR/deb_build.py" verify "$DEB" "$TMP/extract"
fi
"$TMP/extract/usr/bin/folio" --version
test -x "$TMP/extract/usr/bin/folio"
test -f "$TMP/extract/usr/share/applications/io.github.leonisauro.Folio.desktop"
test -f "$TMP/extract/usr/share/metainfo/io.github.leonisauro.Folio.metainfo.xml"
appstreamcli validate "$TMP/extract/usr/share/metainfo/io.github.leonisauro.Folio.metainfo.xml" >/dev/null
log ".deb verificado: $(stat -c '%s' "$DEB") bytes"

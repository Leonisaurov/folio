#!/usr/bin/env bash
# Construye y verifica un paquete Arch (pacman) sin privilegios.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=automation/common.sh
source "$SCRIPT_DIR/automation/common.sh"
cd "$ROOT"
require_cmd makepkg
require_cmd pacman
require_cmd bsdtar
require_cmd tar
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
TMP="$(mktemp -d "$ROOT/.artifacts/arch-XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src" "$ROOT/dist/arch"
# Fuente reproducible del árbol, sin artefactos locales.
tar -C "$ROOT" \
    --exclude='./target' --exclude='./dist' --exclude='./.git' \
    --exclude='./.artifacts' --exclude='./.folio' \
    --transform "s,^\./,folio-$VERSION/," \
    -czf "$TMP/folio-$VERSION.tar.gz" .
cp "$ROOT/PKGBUILD" "$TMP/PKGBUILD"
cd "$TMP"
log "makepkg --nodeps (sin root)"
PATH="/tmp/opencode/bin:$PATH" PKGDEST="$ROOT/dist/arch" SRCDEST="$TMP" BUILDDIR="$TMP/build" \
    makepkg --nodeps --force --cleanbuild --nocheck >"$TMP/makepkg.log" 2>&1 || {
        tail -30 "$TMP/makepkg.log" >&2
        die "makepkg falló"
    }
PKG="$(find "$ROOT/dist/arch" -maxdepth 1 -name "folio-${VERSION}-*.pkg.tar.zst" -print -quit)"
[[ -n "$PKG" ]] || die "no se generó el paquete Arch"
log "Paquete: $PKG"
pacman -Qp "$PKG" | sed -n '1,8p'
bsdtar -tf "$PKG" | sed -n '1,15p'
mkdir -p "$TMP/root"
bsdtar -xf "$PKG" -C "$TMP/root"
"$TMP/root/usr/bin/folio" --version | grep -q "$VERSION"
test -f "$TMP/root/usr/share/applications/io.github.leonisauro.Folio.desktop"
test -d "$TMP/root/usr/share/folio/plugins"
log "Paquete Arch verificado: $(stat -c '%s' "$PKG") bytes"

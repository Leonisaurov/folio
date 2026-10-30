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

SHA_TAR="$(sha256sum "$TAR" | cut -d' ' -f1)"
SHA_DEB="$(sha256sum "$DEB" | cut -d' ' -f1)"
MANIFEST="$DIST/RELEASE-$VERSION.md"
cat >"$MANIFEST" <<EOF
# Folio $VERSION — manifiesto de release

**Fecha**: $(date -u +%Y-%m-%dT%H:%M:%SZ)
**Canal**: release candidate (sin firma)
**Arquitectura**: $ARCH

## Artefactos

| Artefacto | SHA-256 |
|---|---|
| $(basename "$TAR") | \`$SHA_TAR\` |
| $(basename "$DEB") | \`$SHA_DEB\` |

Verificación ejecutada:

- tarball extraído y \`folio --version\` = \`$GOT\`;
- \`.deb\` extraído, binario ejecutado y AppStream validado;
- \`./scripts/automation/all.sh\` verde;
- \`./scripts/beta-check.sh\` verde (sin telemetría).

## Gates automáticos

- [x] Workspace compila sin warnings con clippy.
- [x] Suite de dominio, UI en Xvfb, PDF, cifrado y a11y.
- [x] \`cargo audit\` y \`cargo deny\` sin vulnerabilidades.
- [x] Artefactos y checksums generados.
- [x] AppStream y desktop validados.

## Bloqueos humanos / decisiones pendientes

- [ ] **Firma GPG**: no hay \`FOLIO_SIGNING_KEY\` en este entorno. El artefacto
  queda sin firma; no se declara publicado.
- [ ] **Beta de 30 días con testers reales**: el canal y el checklist están listos,
  pero no se ha ejecutado el periodo.
- [ ] **Flatpak/AUR en VM limpia**: requiere \`flatpak-builder\`/Arch limpio y
  validación manual; los manifiestos existen.
- [ ] **Revisión manual con Orca/Wayland**: la auditoría automatizada y la lista de
  bloques están hechas, pero falta la prueba con lector real.
- [ ] **Runtime WASM de terceros**: decisión de incluirlo en 1.0 o 1.1 (ADR-023).
  Los cinco plugins de referencia y \`api = 1\` están congelados.
- [ ] **Publicación**: tag firmado, release notes y rollback firmado.

## Cómo continuar

1. Aportar la clave de firma y ejecutar
   \`FOLIO_SIGNING_KEY=<huella> ./scripts/release.sh\`.
2. Distribuir \`dist/beta/$VERSION/\` y completar los 30 días.
3. Validar Flatpak/AUR en una VM limpia.
4. Completar la revisión con lector de pantalla.
5. Decidir el runtime WASM y publicar el tag \`v$VERSION\`.
EOF
cp "$MANIFEST" "$DIST/RELEASE.md"
sed -n '1,45p' "$MANIFEST"
log "Manifiesto: $MANIFEST"

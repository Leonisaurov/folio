# 21 — Release 1.0

## 1. Precondiciones

- Beta de 30 días cerrada según `docs/20-beta-30-dias.md`.
- Sin P0/P1 abiertos.
- `./scripts/automation/all.sh`, `security.sh`, `a11y.sh` y `beta-check.sh` verdes.
- Firma y claves disponibles (solo el usuario puede aportarlas).
- VM limpia Arch/GNOME y Debian/Ubuntu para Flatpak/AUR/.deb.

## 2. Congelación

1. Crear rama `release/1.0` y congelar features.
2. Ejecutar la suite completa y guardar `.artifacts/` relevantes.
3. Revisar `docs/09`, `docs/10`, `docs/14` y el changelog.
4. Confirmar `Cargo.toml` versión `1.0.0` y `docs/09` estados.

## 3. Estado de ejecución (2026-10-29)

Ejecutado en el entorno de desarrollo:

```text
./scripts/automation/all.sh   → OK
./scripts/beta-check.sh       → BETA-CHECK OK
./scripts/release.sh          → tarball 1.0.0 + checksum, sin firma
./scripts/build-deb.sh        → .deb 1.0.0 verificado, sin firma
./scripts/build-arch.sh       → paquete pacman 1.0.0 verificado
./scripts/release-manifest.sh → dist/RELEASE-1.0.0.md
./scripts/a11y.sh             → OK
./scripts/webkit.sh           → OK
Ubuntu 24.04 (Docker)         → .deb instalado, folio 1.0.0
Arch Linux (Docker)           → pacman instalado, folio 1.0.0
```

Artefactos en `dist/`; checksums en el manifiesto. El release queda en estado
**RC no publicado** hasta completar los bloqueos humanos de la sección 6.

## 4. Artefactos

```bash
./scripts/automation/all.sh
./scripts/security.sh
./scripts/a11y.sh
./scripts/beta-check.sh
./scripts/build-deb.sh
./scripts/release.sh
```

- `release.sh` genera tarball y checksums; con `FOLIO_SIGNING_KEY` firma GPG.
- `.deb` se construye con `build-deb.sh` y se verifica por extracción.
- `release-manifest.sh` verifica ambos artefactos y escribe `dist/RELEASE-<versión>.md`
  con checksums y bloqueos humanos.
- `FOLIO_SIGNING_KEY=<huella> ./scripts/sign-artifacts.sh` firma tarball, `.deb`,
  paquete pacman, manifiesto y `SHA256SUMS` (ver [docs/24](24-firma.md)).
- Flatpak: `flatpak-builder --force-clean build-dir flatpak/...yml` en VM limpia.
- AUR: `makepkg -si` en VM limpia.

## 5. Publicación

- [ ] Tag `v1.0.0` firmado.
- [ ] Flatpak, AUR y `.deb` publicados.
- [ ] `SHA256SUMS` y firma adjuntas.
- [ ] Release notes con cambios, límites y recuperación.
- [ ] Canal beta cerrado y enlace a 1.0.

## 6. Rollback de release

1. Retirar el artefacto defectuoso y avisar en la release.
2. Republicar el artefacto anterior ya firmado, nunca reempaquetar en caliente.
3. En datos: backup + restore a copia + `doctor`; no tocar la bóveda original.
4. Abrir postmortem y convertir el fallo en test antes de la siguiente release.

## 7. Pendientes que no bloquean el MVP pero sí la narrativa 1.0

- Runtime WASM de terceros: implementado con ABI JSON/wasmi (ADR-033). El WIT
  queda como contrato de migración futura.
- Publicación/validación Flatpak y AUR en una VM de escritorio limpia: el `.deb` y
  el paquete pacman ya se verificaron en contenedores limpios, pero Flatpak y la
  publicación AUR requieren credenciales/entorno del usuario.
- Revisión manual con lector de pantalla/Wayland: requiere un tester humano.
- Beta de 30 días: requiere testers reales; el canal y el checklist están listos.

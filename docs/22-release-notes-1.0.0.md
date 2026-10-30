# Folio 1.0.0 — release candidate

**Fecha**: 2026-10-29
**Canal**: release candidate **firmada** (clave `18615FBFBDFAE2E51866B5AB554741526478CD4A`)
**Arquitectura**: x86_64

## Qué incluye

- Bóveda local con archivos JSON, assets BLAKE3 e índice SQLite/FTS5 reconstruible.
- Libros diarios con hora de corte, DST, formatos y backfill.
- Editor de bloques, lienzo híbrido, anclaje, zoom y recomposición.
- Tablas, imagen, audio, vídeo, archivo, web card, dibujo y plugins.
- Búsqueda global, previsualización, exportación Markdown/HTML/PDF e importación Markdown.
- Cifrado opcional Argon2id + XChaCha20-Poly1305, recuperación y auto-bloqueo.
- Vista web viva opcional WebKitGTK 6.0, efímera y sin cookies persistentes.
- Accesibilidad: teclado, etiquetas, escala 200 %, tema claro/oscuro y alternativa
  al lienzo.
- `.deb`, Flatpak/AUR (manifiestos), AppStream, manual y recuperación.

## Artefactos

```text
dist/folio-1.0.0-linux-x86_64.tar.gz
dist/folio_1.0.0_amd64.deb
dist/arch/folio-1.0.0-1-x86_64.pkg.tar.zst
dist/RELEASE-1.0.0.md
dist/beta/0.1.0/          # canal beta preparado en la semana 19
```

## Estado de la release

**Beta publicada** en `v1.0.0-beta.1`; el release final `v1.0.0` aún no. Faltan:

1. [x] Firma GPG generada y aplicada a todos los artefactos.
2. Beta real de 30 días con testers; el canal y checklist están listos.
3. Publicación Flatpak/AUR en una VM limpia; `.deb` y pacman ya se instalaron y
   verificaron en contenedores limpios.
4. Revisión manual con Orca/Wayland.
5. ~~Decisión sobre el runtime WASM~~: implementado con ABI JSON/wasmi; queda
   validar la guía con un plugin externo.

Los gates automáticos (plan, tests, UI aislada, a11y, PDF, cifrado, auditoría,
AppStream y artefactos) están verdes. Ver `dist/RELEASE-1.0.0.md`.

## Rollback

Instalar la versión anterior firmada, restaurar un backup en una copia y ejecutar
`folio doctor --rebuild-index`. Nunca restaurar in-place sin confirmación.

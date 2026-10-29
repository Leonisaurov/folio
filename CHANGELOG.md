# Changelog

Todas las fechas en formato ISO 8601. Este proyecto sigue [SemVer](https://semver.org/).

## [0.1.0] — 2026-10-29

Primer MVP funcional de Folio. Implementa el plan E0–E13 con las limitaciones
documentadas en `docs/09` y `docs/10`.

### Añadido

- Workspace Rust con `folio-core`, `store`, `config`, `daily`, `layout`, `render`,
  `plugins`, `plugin-sdk`, `ui`, `app` y `tools`.
- Modelo de nota con bloques tipados, campos desconocidos preservados y validación.
- Bóveda con escritura atómica, orden fraccional, índice SQLite/FTS5 reconstruible,
  watcher, lock, papelera, versiones, backups por hardlinks y `doctor`.
- Libros diarios con hora de corte, DST, formatos en español, backfill y renombrado
  sin duplicados.
- Configuración TOML con esquema único, precedencia, recarga, GUI generada y CLI.
- Editor GTK4/Libadwaita con texto rico, toolbar, autosave, recuperación, bloques,
  lienzo con anclaje/zoom/recomposición, previsualización y búsqueda.
- Medios, tarjetas web, tablas, dibujo vectorial y bloques de plugin.
- Exportación Markdown/HTML sin scripts e importación Markdown saneada.
- Plugins de referencia `progress`, `chart`, `diagram`, `stats` y `quickbar`,
  manifiestos, permisos, firma Ed25519, límites y aislamiento de panic.
- Automatización aislada en Xvfb, scripts de caos, seguridad, rendimiento, soak y
  release; baseline de arranque.
- Manifiestos de Flatpak/AUR, AppStream, iconos y documentación de usuario.

### Limitaciones conocidas

- El runtime WASM no ejecuta binarios de terceros en esta compilación; los plugins
  de referencia usan el mismo contrato desde Rust (ADR-021).
- PDF, cifrado, vista web viva, `.deb`, `quickbar`/`stats` como plugins externos y
  auditoría de accesibilidad completa son 1.0.
- El soak de 2 h y la instalación Flatpak/AUR requieren una máquina limpia y se
  documentan como verificación de release.

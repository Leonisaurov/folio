# Changelog

Todas las fechas en formato ISO 8601. Este proyecto sigue [SemVer](https://semver.org/).

## [1.0.0] — en curso

### Semana 19 — beta de 30 días

- Canal beta local `dist/beta/<versión>` con `.deb`, binarios, checksums y
  `CHANNEL.json` sin telemetría.
- `beta-check.sh`: búsqueda, campos desconocidos, backup/restore, MD/HTML/PDF,
  cifrado/descifrado y ausencia de clientes de red/telemetría.
- Checklist de 30 días, triage P0/P1, migraciones y plan de rollback.
- Preparado el release 1.0 de la semana 20.

### Semana 18 — accesibilidad y .deb

- Etiquetas accesibles en botones de icono, foco visible y foco inicial.
- Lista de bloques como alternativa completa al lienzo (`Ctrl+Shift+B`, `Esc`).
- Auditoría automatizada con escala 200 %, puente a11y y tema claro/oscuro.
- `.deb` reproducible con `scripts/build-deb.sh`; verificación por extracción,
  ejecución de `--version` y AppStream, también sin `dpkg-deb`.

### Semana 17 — PDF y plugins

- Exportación PDF A4 con Cairo/Pango: paginación, tablas, imágenes, medios,
  tarjetas web, dibujos y página de lienzo con posiciones.
- Callback de assets para bóvedas cifradas; nunca se ejecuta web viva.
- Golden de texto extraído con `pdftotext` y pruebas de paginación.
- Los cinco plugins de referencia cubren el catálogo 1.0 y el contrato `api = 1`
  está congelado y documentado.

### Semana 16 — cifrado de bóveda

- Argon2id + XChaCha20-Poly1305 con VK envuelta en `.folio/keyring.json`.
- Cifrado de libros, notas, assets, versiones, papelera y recuperación.
- Frase de recuperación BIP39 de 24 palabras verificada por hash.
- Índice efímero en memoria en bóvedas cifradas.
- Auto-bloqueo por inactividad y bloqueo manual en GUI; desbloqueo con contraseña
  o frase; CLI con `--password`/`FOLIO_PASSWORD`.
- `vault encrypt`/`vault decrypt`, pruebas de `strings`, temporales, assets y TOML.

### Semana 15 — vista web viva

- WebKitGTK 6.0 tras feature flag `webkit`.
- `NetworkSession` efímera, cookies `Never`, sin almacenamiento local, base de
  datos, caché de página ni WebGL.
- Permisos denegados, ventanas emergentes bloqueadas y descargas canceladas.
- Confirmación explícita, descarga/terminación del proceso web al salir.
- Pruebas de aislamiento en Xvfb (`scripts/webkit.sh`).

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

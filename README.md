# Folio — notas y libros diarios

**Folio** es una aplicación nativa de Linux para notas y libros diarios: rápida,
local primero, con un editor híbrido entre documento lineal y lienzo infinito, y
plugins aislados con UI declarativa.

> Estado: **1.0.0 release candidate generada** (E0–E13 y semanas 15–20;
> artefactos y checksums creados). La firma, la beta real de 30 días, la VM limpia
> Flatpak/AUR, la revisión con lector de pantalla y el runtime WASM de terceros
> siguen siendo bloqueos humanos/decisiones documentados en
> [docs/09](docs/09-hoja-de-ruta.md#17-estado-de-avance) y [docs/21](docs/21-release-1.0.md).
> Binario: `folio`. ID de aplicación: `io.github.leonisauro.Folio`.
>
> Licencia de la app: GPL-3.0-or-later. SDK de plugins: MIT OR Apache-2.0.

## Qué funciona hoy

- **Bóveda**: archivos JSON legibles + assets direccionados por BLAKE3; SQLite/FTS5
  solo como índice reconstruible. Escritura atómica, lock, watcher, papelera,
  versiones y backups por hardlinks.
- **Libros diarios**: día lógico con hora de corte, zonas IANA, DST, patrones en
  español, backfill y renombrado sin duplicados.
- **Notas**: orden manual fraccional, DnD dentro y entre libros, spring-load de
  650 ms, «Mover a…», búsqueda y papelera.
- **Editor**: texto enriquecido con tags, barra de formato, undo/redo, autosave,
  recuperación tras crash, previsualización de solo lectura y contador.
- **Lienzo**: flujo por defecto, bloques anclados, arrastre, zoom 25–400 %,
  encuadres y «Recomponer».
- **Bloques**: tabla editable, imagen, audio, vídeo, archivo, tarjeta web, dibujo
  vectorial y plugin.
- **Plugins**: manifiestos, permisos, firma Ed25519, límites, aislamiento de panic
  y cinco ejemplos (`progress`, `chart`, `diagram`, `stats`, `quickbar`).
- **Vista web viva** opcional con WebKitGTK 6.0 (`--features webkit`): sesión
  efímera, sin cookies, sin descargas, permisos denegados y descarga al salir.
- **Cifrado opcional** de bóveda con Argon2id + XChaCha20-Poly1305, clave
  envuelta, frase de recuperación de 24 palabras, índice efímero y auto-bloqueo.
- **Datos**: exportación Markdown/HTML sin scripts y PDF A4 paginado con Cairo/Pango;
  importación Markdown saneada.
- **Configuración**: un esquema único para TOML, CLI y GUI; precedencia y recarga.

Limitaciones del MVP: el runtime WASM de terceros y la validación Flatpak/AUR en
una VM limpia siguen pendientes; la auditoría de accesibilidad automatizada y el
`.deb` ya están disponibles (ver [docs/12](docs/12-backlog-post-1.0.md), ADR-023,
ADR-027, ADR-028, ADR-029 y ADR-030/031).

## Compilar y ejecutar

Dependencias del sistema en Arch:

```bash
sudo pacman -S --needed base-devel rustup pkgconf gtk4 libadwaita sqlite \
  xorg-server-xvfb dbus
rustup default stable
rustup component add rustfmt clippy
```

```bash
# Bóveda cifrada (pide contraseña y muestra la frase de recuperación):
./target/release/folio vault init ~/Notas --encrypt
# o cifrar una existente (haz backup antes):
./target/release/folio --vault ~/Notas vault encrypt

cargo build --release
# Vista web viva (requiere webkitgtk-6.0 / libwebkitgtk-6.0-dev):
cargo build --release --features webkit
./target/release/folio --help
./target/release/folio vault init ~/Notas
./target/release/folio --vault ~/Notas
```

Sin argumentos abre la GUI. `folio --vault <ruta>` abre una bóveda concreta.

## Verificación

```bash
./scripts/automation/all.sh          # plan + calidad + dominio + UI en Xvfb
./scripts/automation/all.sh --fast   # sin UI
./scripts/chaos.sh                   # kill -9 durante guardados y movimientos
./scripts/security.sh                # sandbox, rutas, HTML, auditoría
./scripts/perf.sh                    # 10k notas, búsqueda, layout y arranque
./scripts/soak.sh --quick 30         # soak corto; sin --quick son 2 h
./scripts/a11y.sh                    # foco, etiquetas y escala 200 %
./scripts/build-deb.sh               # construye y verifica el .deb
./scripts/beta-check.sh              # pre-beta: datos, backups, cifrado, sin telemetría
./scripts/beta.sh                    # prepara dist/beta/<versión>
./scripts/webkit.sh                  # aislamiento de la vista web viva
cargo test -p folio-store --test encryption  # strings, temporales, recuperación
cargo test -p folio-render --test pdf_golden # PDF, paginación y lienzo
./bench/startup.sh                   # actualiza bench/baseline.json
```

La UI de los tests **nunca** usa tu `DISPLAY`: `all.sh` arranca Xvfb y un D-Bus
privados. Detalle en [docs/15](docs/15-automatizacion.md).

Última medición (release, Xvfb 1440×900, 29/10/2026):

| Métrica | Resultado |
|---|---|
| Generar 10 000 notas | 11,2 s |
| Reconstruir índice de 10 000 notas | 4,1 s |
| Búsqueda FTS con proceso incluido | 66 ms |
| Layout de 500 bloques | 0,19 ms |
| Arranque en caliente (mediana) | 527 ms |
| Arranque en frío (Xvfb/llvmpipe) | 2,56 s |

El arranque en frío bajo Xvfb incluye la compilación de shaders de software; el
presupuesto de 600 ms se mide en sesión real. `bench/baseline.json` guarda la
medición y su entorno.

## Documentación

| Si quieres… | Lee |
|---|---|
| Entender el producto | [docs/01-vision-alcance.md](docs/01-vision-alcance.md) |
| Arquitectura técnica | [docs/02-arquitectura.md](docs/02-arquitectura.md) |
| Modelo y persistencia | [docs/03-modelo-datos.md](docs/03-modelo-datos.md) |
| UI, atajos y accesibilidad | [docs/04-experiencia-ui.md](docs/04-experiencia-ui.md) |
| Configuración TOML/CLI/GUI | [docs/05-configuracion.md](docs/05-configuracion.md) |
| Plugins y SDK | [docs/06-plugins.md](docs/06-plugins.md) y [plugins/README.md](plugins/README.md) |
| Seguridad y rendimiento | [docs/07-seguridad-rendimiento.md](docs/07-seguridad-rendimiento.md) |
| Pruebas y entrega | [docs/08-calidad-pruebas.md](docs/08-calidad-pruebas.md) |
| Etapas, puertas y estado | [docs/09-hoja-de-ruta.md](docs/09-hoja-de-ruta.md) |
| Decisiones y ADR | [docs/10-decisiones.md](docs/10-decisiones.md) |
| Manual de usuario | [docs/16-manual-usuario.md](docs/16-manual-usuario.md) |
| Recuperación y backups | [docs/17-recuperacion.md](docs/17-recuperacion.md) |
| Accesibilidad | [docs/18-accesibilidad.md](docs/18-accesibilidad.md) |
| Empaquetado .deb | [docs/19-empaquetado.md](docs/19-empaquetado.md) |
| Beta de 30 días | [docs/20-beta-30-dias.md](docs/20-beta-30-dias.md) |
| Release 1.0 | [docs/21-release-1.0.md](docs/21-release-1.0.md) y [notas 1.0.0](docs/22-release-notes-1.0.0.md) |
| Automatización aislada | [docs/15-automatizacion.md](docs/15-automatizacion.md) |

Resumen ejecutivo del plan: [PLAN.md](PLAN.md).

## Estructura

```text
crates/
├── folio-core/        # dominio puro (sin GTK, sin IO)
├── folio-store/       # bóveda, archivos, índice, assets, backups
├── folio-config/      # esquema TOML/CLI
├── folio-daily/       # día lógico y patrones
├── folio-layout/      # geometría de flujo y lienzo
├── folio-render/      # Markdown, HTML e importación
├── folio-plugins/     # manifiestos, permisos, runtime de referencia
├── folio-plugin-sdk/  # contrato y UI declarativa
├── folio-ui/          # GTK4/Libadwaita
├── folio-app/         # binario `folio` y CLI
└── folio-tools/       # generadores y benchmarks
```

## Contribuir

Lee [CONTRIBUTING.md](CONTRIBUTING.md) y [AGENTS.md](AGENTS.md). Los cambios de
alcance exigen ADR y actualizar `docs/09`.

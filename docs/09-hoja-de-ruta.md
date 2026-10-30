# 09 — Hoja de ruta ejecutable

**14 semanas para el MVP; 20 para 1.0. Una sola persona. Cada etapa deja la app ejecutable y probada.**

> **Regla de calendario**: los «pasos» son unidades de trabajo, no días naturales. El MVP ocupa **70 días (14 semanas)** y no tiene un buffer separado: si un paso crece, se aplica la lista de recorte; nunca se acumula deuda de datos.
>
> **MVP**: bóveda, libros diarios, notas, editor, lienzo, orden/DnD, medios, web tarjeta, tablas y dibujo básicos, búsqueda, MD/HTML, plugins con 3 ejemplos, backups y Flatpak/AUR.
>
> **1.0 (semanas 15–20)**: cifrado, web viva, PDF, `quickbar`/`stats`, `.deb`, auditoría de accesibilidad y release estable.

## Cómo leer una etapa

Cada etapa tiene:

- **Objetivo**: el resultado que debe existir al final.
- **Depende de**: lo que debe estar verde antes de empezar.
- **Pasos**: tareas de ≤ 1 día, en orden.
- **Entregables**: artefactos concretos.
- **Puerta**: criterios que no se negocian.
- **Verificación**: comandos o pruebas manuales.
- **Demo**: qué se enseña el viernes.
- **Plan B**: qué hacer si el riesgo se materializa.

Regla: no se empieza una etapa si la anterior no pasó su puerta. Si una puerta falla, se arregla o se recorta el alcance **antes** de avanzar.

---

## E0 — Fundamentos del repositorio y la app

| Campo | Valor |
|---|---|
| Días | 3 (semana 1) |
| Depende de | Nada |
| Riesgo | Bajo |

### Objetivo

Tener un repositorio reproducible, una CI que impide romper lo básico, una ventana GTK que arranca y los presupuestos de rendimiento registrados. Sin código de producto todavía.

### Pasos

1. **Paso 1 — Repositorio**: `git init`, `.gitignore` (target, build, flatpak build-dir, `.folio`), `README.md`, `LICENSE` (GPL-3.0-or-later para la app; SDK MIT/Apache-2.0), `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`.
2. **Paso 2 — Toolchain**: `rust-toolchain.toml` con stable + components (`rustfmt`, `clippy`, `llvm-tools`), `.editorconfig`, `rustfmt.toml`, `clippy.toml`.
3. **Paso 3 — Workspace**: `Cargo.toml` con los crates vacíos de la arquitectura; perfiles `release` (LTO thin, codegen-units 1, strip) y `dev` (debug=1).
4. **Paso 4 — Calidad**: `deny.toml`, `justfile` con `fmt`, `lint`, `test`, `check`, `run`, `bench`; hooks locales opcionales.
5. **Paso 5 — CI**: GitHub Actions con fmt, clippy `-D warnings`, test, `cargo deny`, `cargo audit`, `check_plan.py`; cache de cargo.
6. **Paso 6 — Ventana mínima**: `folio-app` con `GtkApplication`, `AdwApplicationWindow`, headerbar, tema del sistema, icono placeholder, `--version`, `--help`.
7. **Paso 7 — Logging y errores**: `tracing`, `FolioError`, panic hook que escribe crash log local.
8. **Paso 8 — Medición**: `folio-tools` con `gen-vault` mínimo; script `bench/startup.sh` con `hyperfine`; guardar `baseline.json`.
9. **Paso 9 — ADRs**: crear `docs/10-decisiones.md` con las decisiones D-01…D-12 y las ADR iniciales.
10. **Paso 10 — Documentación y scripts**: `docs/09` enlazado, `check_plan.py`, plantillas de issue (bug, feature, plugin), política de commits; integrar `scripts/automation/` (plan, quality, unit, ui, all, with-x11, keep-awake, nightly) en CI y en la rutina diaria; stubs ejecutables de `scripts/chaos.sh`, `security.sh`, `perf.sh`, `soak.sh` y `release.sh` que fallan con «no implementado» hasta su etapa.

### Entregables

- Repo compilable en Arch limpio.
- Ventana GTK que abre y cierra.
- CI verde.
- Suite de automatización aislada (`./scripts/automation/all.sh`).
- `baseline.json` con arranque y RAM.
- ADRs iniciales.

### Puerta E0

- [ ] `cargo build --workspace` sin warnings.
- [ ] `./scripts/automation/all.sh` pasa; la UI corre en Xvfb, nunca en el display del usuario.
- [ ] `./scripts/automation/with-x11.sh -- sh -c 'echo $DISPLAY'` devuelve un display distinto de `:0`.
- [ ] `./scripts/automation/keep-awake.sh -- true` bloquea suspensión y permite apagar pantalla.
- [ ] CI verde en el primer push.
- [ ] La ventana arranca en Wayland y X11 (smoke manual).
- [ ] `folio --version` funciona.
- [ ] Presupuesto de arranque registrado.

### Verificación

```bash
./scripts/automation/all.sh
./scripts/automation/with-x11.sh -- sh -c 'echo "DISPLAY=$DISPLAY"'
./scripts/automation/keep-awake.sh -- ./scripts/automation/all.sh --fast
cargo run -p folio-app
hyperfine --warmup 3 './target/release/folio --quit-after-init'
```

### Demo

Ventana vacía con headerbar, tema claro/oscuro, `folio --version`, CI.

### Plan B

Si GTK4/Libadwaita no compila en el entorno, usar Flatpak SDK para desarrollo y documentar la instalación nativa; no cambiar de toolkit.

---

## E1 — Núcleo de dominio, bóveda e índice

| Campo | Valor |
|---|---|
| Días | 7 (semanas 1–2) |
| Depende de | E0 |
| Riesgo | Medio |

### Objetivo

Crear, abrir, guardar y reindexar una bóveda con libros y notas. Todavía sin editor bonito, pero con integridad de datos demostrada.

### Pasos

1. **Paso 1 — IDs y tiempo**: `Uuid::now_v7`, `Clock` inyectable, `jiff` para zonas; tests de orden y DST básico.
2. **Paso 2 — Modelo**: `Vault`, `Book`, `Note`, `Block`, `RichText`, `Placement`; validación de invariantes; sin IO.
3. **Paso 3 — Serialización**: JSON estable, `schema = 1`, campos desconocidos preservados, fixtures de cada tipo de bloque.
4. **Paso 4 — Bóveda**: crear estructura, `vault.toml`, `book.toml`, `notes/`, `assets/`, `.folio/`; permisos 0700/0600.
5. **Paso 5 — Escritura atómica**: helper con temp + fsync + rename + fsync dir; tests de interrupción; `cap-std` para rutas.
6. **Paso 6 — Repositorio**: `create_book`, `rename_book`, `create_note`, `load_note`, `save_note`, `delete_note` (a papelera), `move_note` (rename con order_key).
7. **Paso 7 — Orden**: generador de claves fraccionales; nombres `<order_key>_<id>.json`; tests de inserción y colisión.
8. **Paso 8 — Índice**: SQLite, esquema, writer thread, FTS5; indexar al guardar; `rebuild`.
9. **Paso 9 — Doctor**: validar archivos, reconstruir índice, detectar huérfanos, replay de journal; `folio doctor`.
10. **Paso 10 — Watcher y lock**: `notify` con debounce, lock de instancia, modo solo lectura; pruebas de edición externa.

### Entregables

- `folio vault init`, `book create`, `note create`, `doctor`, `index rebuild`.
- Bóveda de ejemplo en `tests/fixtures/`.
- Suite de integración con 10k notas sintéticas.
- Journal de operaciones y recuperación.

### Puerta E1

- [ ] Roundtrip de todos los tipos de bloque.
- [ ] Borrar `index.sqlite` y reconstruir sin pérdida.
- [ ] `kill -9` durante 20 guardados: ningún JSON corrupto.
- [ ] Mover 100 notas entre libros: sin duplicados ni pérdidas.
- [ ] Watcher detecta edición externa y reindexa.
- [ ] `doctor` repara índice y reporta huérfanos.
- [ ] 10k notas indexadas en < 30 s.
- [ ] Test de propiedades de orden (10k movimientos).

### Verificación

```bash
cargo test -p folio-core -p folio-store --all-features
cargo run -p folio-app -- vault init /tmp/folio-test
cargo run -p folio-app -- note create --book today --title "Prueba"
./scripts/chaos-save.sh
cargo run -p folio-tools -- gen-vault --notes 10000 --out /tmp/folio-10k
```

### Demo

Crear bóveda, libros y notas desde CLI; borrar el índice; reconstruirlo; matar el proceso a mitad de guardado y mostrar recuperación.

### Plan B

Si SQLite/FTS5 bundled da problemas en Flatpak, usar SQLCipher/system SQLite con feature flag; el índice sigue siendo reconstruible. Si el journal es demasiado complejo, limitar operaciones multiarchivo y usar backups antes de mover; no se acepta pérdida.

---

## E2 — Shell nativo y navegación

| Campo | Valor |
|---|---|
| Días | 4 (semana 3) |
| Depende de | E1 (modelo y store) |
| Riesgo | Bajo |

### Objetivo

Una app usable para navegar: abrir bóveda, ver «Hoy», libros, lista de notas, crear/renombrar/borrar, buscar en la paleta y usar el teclado.

### Pasos

1. **Paso 1 — Servicios**: `AppState`, `ConfigService` básico, `StoreService`, `CommandBus`; inyección desde `folio-app`.
2. **Paso 2 — Ventana principal**: `AdwNavigationSplitView` con sidebar, lista y contenido; estados vacíos; headerbar.
3. **Paso 3 — Sidebar**: árbol de libros agrupado por año/mes, Hoy, fijados, archivados; iconos; contador.
4. **Paso 4 — Lista de notas**: filas con título/extracto/fecha/iconos; selección; orden básico; botón `+`.
5. **Paso 5 — Acciones**: crear libro/nota, renombrar, archivar, borrar/restaurar; diálogos de confirmación; undo de borrado.
6. **Paso 6 — Paleta**: `Ctrl+K` con comandos y búsqueda simple por título; navegación por teclado.
7. **Paso 7 — i18n y tema**: Fluent con español; tema claro/oscuro/sistema; atajos; menú de app; instancia única.

### Entregables

- Ventana navegable con datos reales.
- Paleta de comandos.
- i18n base.
- Instancia única y deep link `folio://today`.

### Puerta E2

- [ ] Abrir bóveda y ver libros/notas reales.
- [ ] Crear, renombrar y borrar libros y notas.
- [ ] La paleta encuentra notas por título y ejecuta comandos.
- [ ] Todo se puede hacer con teclado.
- [ ] La lista mantiene 60 fps con 1k notas.
- [ ] Cambio de tema en vivo.
- [ ] La app no arranca dos veces; el segundo lanzamiento activa la ventana.

### Verificación

```bash
./scripts/automation/ui.sh
./scripts/automation/ui.sh --test smoke
cargo run -p folio-app -- --vault /tmp/folio-test
```

### Demo

Navegación completa por teclado, paleta, tema, deep link.

### Plan B

Si `AdwNavigationSplitView` no está en la versión mínima, usar `GtkPaned` con el mismo layout; la UX se mantiene.

---

## E3 — Libros diarios y configuración total

| Campo | Valor |
|---|---|
| Días | 5 (semanas 3–4) |
| Depende de | E1, E2 |
| Riesgo | Medio |

### Objetivo

Que el libro del día aparezca solo, se pueda renombrar y configurar por GUI, TOML y CLI sin duplicados ni sorpresas de zona horaria.

### Pasos

1. **Paso 1 — Motor diario**: `LogicalDay` con zona, `day_start_hour`, DST; tests de 23/25 horas y cambios de zona.
2. **Paso 2 — Políticas**: `first_open`, `first_note`, `manual`; creación perezosa; no rellenar días; `recreate_deleted`.
3. **Paso 3 — Patrones**: `title_format` y `folder_format` con `jiff`/ICU; validación y fallback; ejemplos en vivo.
4. **Paso 4 — Renombrar**: título editable, `daily_date` inmutable, insignia de fecha, «Restaurar título automático».
5. **Paso 5 — Backfill**: diálogo de rango, vista previa, creación idempotente; «Generar libros faltantes».
6. **Paso 6 — Config**: esquema completo de `daily.*`, carga, precedencia, watcher, errores; CLI `config`.
7. **Paso 7 — GUI**: sección «Libros diarios» generada del esquema; editor TOML; prueba de paridad; recarga en vivo.

### Entregables

- Libro de hoy automático y renombrable.
- Backfill manual.
- Configuración diaria por GUI/TOML/CLI.
- Tests de DST y zona.

### Puerta E3

- [ ] Abrir a las 03:59 y 04:00 produce el día correcto.
- [ ] Con la app abierta, cruzar `day_start_hour` crea/reutiliza el libro nuevo sin interrumpir la nota actual.
- [ ] Cambiar de zona con la app abierta no duplica.
- [ ] Renombrar el libro no crea otro al reiniciar.
- [ ] Cambiar `title_format` en GUI se refleja sin reiniciar.
- [ ] Cambiar `folder_format` en TOML se refleja sin reiniciar.
- [ ] TOML inválido conserva la configuración anterior y muestra línea.
- [ ] `config set` y la GUI producen el mismo archivo.
- [ ] Backfill de 10 días crea exactamente 10 libros y no toca los existentes.

### Verificación

```bash
cargo test -p folio-daily -p folio-config
cargo run -p folio-app -- config set daily.title_format "%Y/%m/%d"
cargo run -p folio-app -- config validate
TZ=America/New_York cargo test -p folio-daily dst
```

### Demo

Viaje en el tiempo simulado: abrir 03:59, 04:00, cambiar formato, renombrar, reiniciar, ver que no se duplica.

### Plan B

Si ICU/Jiff no localiza meses, usar `chrono` + `chrono-tz` + tabla de meses en español; el formato `strftime` se mantiene. Si el backfill es riesgoso, dejarlo solo CLI para el MVP.

---

## E4 — Editor de texto y modelo de bloques

| Campo | Valor |
|---|---|
| Días | 8 (semanas 4–6) |
| Depende de | E1, E2 |
| Riesgo | **Alto** |

### Objetivo

Escribir una nota real con formato, listas, checklists, citas, código y enlaces; con undo, autosave y recuperación. Sin canvas todavía.

### Pasos

1. **Paso 1 — Spike S-01**: prototipar opciones A/B/C con 5k palabras; medir tecleo, IME, undo, copiar/pegar; decidir y escribir ADR.
2. **Paso 2 — Modelo de texto**: `TextBlock`, párrafos, spans, marcas, validación, serialización.
3. **Paso 3 — Vista**: `TextBlockView` con GtkTextView + tags; estilos H1–H3, cita, código, listas, checklist.
4. **Paso 4 — Entrada**: IME, selección, portapapeles, pegado saneado, arrastrar texto, atajos Markdown.
5. **Paso 5 — Barra de formato**: toolbar contextual, menú `/`, selector de estilo, enlaces, colores.
6. **Paso 6 — Undo**: `UndoStack` de dominio, coalescing, undo de formato y de bloques; desactivar undo de GTK.
7. **Paso 7 — Autosave**: debounce, guardado al perder foco, estado visual, journal de recuperación.
8. **Paso 8 — Título y outline**: título editable, derivación desde primera línea, esquema de encabezados, contador.
9. **Paso 9 — Previsualización básica**: solo lectura de texto; `Ctrl+Shift+E`.
10. **Paso 10 — Pruebas**: propiedad de edición, golden de serialización, 100 pasos de undo, IME español, 5k palabras.

### Entregables

- Editor de texto usable a diario.
- Undo/redo robusto.
- Autosave y recuperación.
- Previsualización.

### Puerta E4

- [ ] Escribir 5k palabras sin degradación; p95 < 16 ms.
- [ ] IME español funciona (tildes, ñ, composición).
- [ ] Copiar/pegar entre bloques y desde el sistema.
- [ ] Pegar HTML con `<script>` no ejecuta nada.
- [ ] Undo/redo de 100 operaciones, incluyendo formato.
- [ ] `kill -9` antes del autosave ofrece recuperación sin pérdida.
- [ ] El estado «Guardado» es veraz.
- [ ] La previsualización no permite editar.
- [ ] El editor es usable solo con teclado.

### Verificación

```bash
cargo test -p folio-core -p folio-ui editor
./scripts/automation/ui.sh --test editor_e2e
cargo bench -p folio-layout -- text
```

### Demo

Nota de 5k palabras con formato, checklists, código, enlaces; undo/redo; matar la app y recuperar.

### Plan B

Si la opción A falla en selección/IME, adoptar C (GtkTextView único + canvas de previews) para el MVP y mover la edición in situ a 1.0. No intentar B en 14 semanas.

---

## E5 — Lienzo híbrido

| Campo | Valor |
|---|---|
| Días | 8 (semanas 6–7) |
| Depende de | E4 |
| Riesgo | **Alto** |

### Objetivo

Convertir la nota en un lienzo infinito con flujo por defecto: mover bloques, anclarlos, zoom, paneo y recomponer, sin perder la comodidad de escritura.

### Pasos

1. **Paso 1 — Spike S-02**: viewport con zoom/pan y widgets GTK; medir hit-testing y 500 bloques; decidir.
2. **Paso 2 — Geometría**: `WorldPoint`, `Rect`, `Viewport`, conversión, bbox, hit-testing; tests puros.
3. **Paso 3 — Layout de flujo**: medir bloques, apilar, gap, ancho; cache de layouts.
4. **Paso 4 — Bloques libres**: `placement`, z, resize, límites; render de anclados.
5. **Paso 5 — CanvasView**: snapshot, fondo papel/rejilla, paneo, zoom al puntero, scrollbars.
6. **Paso 6 — Arrastrar y anclar**: asa, sombra, cierre de hueco, drop, cancelar, undo.
7. **Paso 7 — Recomponer/volver al flujo**: algoritmos, animación, toast undo.
8. **Paso 8 — Selección múltiple y teclado**: lazo, Shift, flechas, resize, Z-order, duplicar.
9. **Paso 9 — Encuadres**: documento, todo, 100 %; LOD; barra de posición (minimapa en 1.1).
10. **Paso 10 — Rendimiento y pruebas**: 500 bloques, memoria, golden de posiciones, accesibilidad del canvas.

### Entregables

- Lienzo funcional con flujo y anclaje.
- Comandos de encuadre y recomposición.
- Pruebas de geometría y rendimiento.
- Alternativa de teclado.

### Puerta E5

- [ ] 500 bloques con paneo/zoom a 60 fps.
- [ ] Arrastrar un bloque lo ancla exactamente donde se suelta.
- [ ] El flujo cierra el hueco sin saltos.
- [ ] «Recomponer» ordena por posición visual y es undoable.
- [ ] `Ctrl+1` enfoca el documento; `Ctrl+2` muestra todo.
- [ ] Hit-testing correcto a 25 %, 100 % y 400 %.
- [ ] Edición de texto funciona tras mover un bloque.
- [ ] Todas las acciones tienen equivalente por teclado.
- [ ] No hay fugas de widgets al hacer scroll.

### Verificación

```bash
cargo test -p folio-layout
./scripts/automation/ui.sh --test canvas_e2e
heaptrack ./target/release/folio --open tests/fixtures/canvas-500
```

### Demo

Anclar texto, imagen y tabla; mover en grupo; recomponer; encuadres; zoom; undo.

### Plan B

Si el hit-testing con widgets escalados no es fiable, renderizar previews en canvas y abrir un editor superpuesto al hacer doble clic (opción C). El modelo y los comandos no cambian.

---

## E6 — Orden libre de notas y drag & drop

| Campo | Valor |
|---|---|
| Días | 4 (semana 8) |
| Depende de | E5 |
| Riesgo | Medio |

### Objetivo

Que mover notas sea tan natural como mover bloques: dentro del libro, entre libros, con spring-load y selector de posición.

### Pasos

1. **Paso 1 — Orden manual**: claves fraccionales, renombrado de archivo, rebalanceo, tests.
2. **Paso 2 — DnD interno**: asa, línea de inserción, auto-scroll, drop, undo.
3. **Paso 3 — DnD entre libros**: drag a sidebar, resaltado, spring-load 650 ms, abrir libro.
4. **Paso 4 — Selector de posición**: popover con líneas, «principio/final», teclado, `Esc`.
5. **Paso 5 — Alternativa y robustez**: «Mover a…» por teclado, journal, crash, permisos, 100 movimientos.

### Entregables

- Orden manual completo.
- Spring-load y selector.
- Alternativa por teclado.
- Pruebas de crash y undo.

### Puerta E6

- [ ] Mover dentro del libro actualiza el orden y persiste.
- [ ] Mover entre libros no duplica ni pierde.
- [ ] Spring-load se dispara a los 650 ms y se cancela con `Esc`.
- [ ] El selector permite insertar en cualquier posición.
- [ ] `Ctrl+Shift+M` hace lo mismo por teclado.
- [ ] Cambiar a orden por título y volver conserva el orden manual.
- [ ] 100 movimientos aleatorios: invariantes intactos.
- [ ] Crash a mitad de move: recuperación correcta.

### Verificación

```bash
cargo test -p folio-core ordering
cargo test -p folio-store move_note
./scripts/automation/ui.sh --test dnd
./scripts/chaos-move.sh
```

### Demo

Arrastrar una nota de «Hoy» a «Proyectos», abrir el libro con hover, elegir posición 2, soltar, deshacer.

### Plan B

Si el DnD de GTK entre widgets es inestable, usar un `GtkDragSource` con datos propios y un overlay de posiciones; el selector siempre existe como fallback.

## E7 — Medios, archivos y web

| Campo | Valor |
|---|---|
| Días | 5 (semanas 8–9) |
| Depende de | E5 |
| Riesgo | Medio |

### Objetivo

Que una nota pueda contener imagen, audio, vídeo, archivo y tarjeta web sin que la app se vuelva pesada o insegura. La vista web viva queda para 1.0.

### Pasos

1. **Paso 1 — Assets**: importar con BLAKE3, copiar/vincular, deduplicar, límites, miniaturas; pruebas de path traversal.
2. **Paso 2 — Imagen**: bloque, pegado/arrastre, redimensionado, visor, EXIF, exportación.
3. **Paso 3 — Audio/vídeo**: GStreamer, reproducción, onda, póster, pausa offscreen; límites de memoria.
4. **Paso 4 — Archivo**: tarjeta, MIME, apertura por portal con confirmación, reemplazo.
5. **Paso 5 — Web tarjeta**: metadatos bajo demanda, caché, favicon, validación de URL, fallback offline.

### Entregables

- Bloques de imagen, audio, vídeo, archivo y web tarjeta.
- Assets deduplicados y cacheados.
- Pruebas de memoria y seguridad.

### Puerta E7

- [ ] Arrastrar una imagen de 20 MB la copia y muestra thumbnail.
- [ ] El mismo archivo dos veces ocupa un solo asset.
- [ ] Vídeo de 100 MB se reproduce sin cargar todo en RAM.
- [ ] El audio se pausa al salir de pantalla.
- [ ] Abrir un archivo pide confirmación y usa el portal.
- [ ] Una URL `javascript:` se rechaza.
- [ ] Sin red, la tarjeta muestra los metadatos cacheados o un fallback.
- [ ] `strings` de la bóveda no revela contenido de web.

### Verificación

```bash
cargo test -p folio-store assets
./scripts/automation/ui.sh --test web_card
./scripts/automation/ui.sh --test media
```

### Demo

Nota con imagen, audio, vídeo, PDF adjunto y tarjeta web; reproducir; comprobar fallback offline.

### Plan B

Si GStreamer falta, degradar a «Abrir externo» y documentarlo. La vista web viva se implementa en 1.0 con WebKitGTK 6.0; no bloquea el MVP.

---

## E8 — Dibujo y tablas

| Campo | Valor |
|---|---|
| Días | 5 (semanas 9–10) |
| Depende de | E5 |
| Riesgo | Medio |

### Objetivo

Añadir los dos bloques que más piden libertad visual y estructura: dibujo vectorial y tablas editables.

### Pasos

1. **Paso 1 — Modelo de dibujo**: trazos, puntos, presión, herramientas, serialización.
2. **Paso 2 — Lienzo de dibujo**: GtkGestureStylus/ratón, render GSK/Cairo, suavizado, borrador.
3. **Paso 3 — Integración**: bloque en flujo/libre, tamaño, zoom, undo, export SVG/PNG.
4. **Paso 4 — Tabla**: modelo, render, edición de celdas, filas/columnas, cabecera, anchos.
5. **Paso 5 — Tabla avanzada**: pegar CSV/TSV, navegación con Tab, selección, eliminar, exportar.
6. **Paso 6 — Pruebas**: 10k trazos, tabla 100×20, golden de export, accesibilidad.

### Entregables

- Bloque de dibujo con presión y export.
- Tabla editable con CSV.
- Pruebas de rendimiento y export.

### Puerta E8

- [ ] Dibujar con stylus y ratón; la presión cambia el grosor si el dispositivo la reporta.
- [ ] Deshacer un trazo es instantáneo.
- [ ] Exportar SVG conserva vectores; PNG respeta zoom.
- [ ] Tabla 100×20 se desplaza fluida.
- [ ] Pegar CSV de 20×10 crea la tabla correcta.
- [ ] Tab entre celdas funciona; `Esc` sale.
- [ ] Exportación de tabla a Markdown/HTML es fiel (PDF en 1.0).
- [ ] El bloque de dibujo se mueve y ancla como cualquier otro.

### Verificación

```bash
cargo test -p folio-core drawing table
./scripts/automation/ui.sh --test drawing
cargo run -p folio-tools -- gen-table --rows 100 --cols 20
```

### Demo

Dibujar un plano, anclarlo junto al texto, insertar una tabla, pegar CSV, exportar SVG y PNG.

### Plan B

Si la presión no está disponible en Wayland, usar velocidad como proxy o ancho fijo; documentarlo. Si la tabla 100×20 no rinde, virtualizar filas y limitar a 50×20 visible.

---

## E9 — Búsqueda, previsualización y exportación

| Campo | Valor |
|---|---|
| Días | 4 (semanas 10–11) |
| Depende de | E1, E4, E7, E8 |
| Riesgo | Medio |

### Objetivo

Encontrar cualquier cosa y sacar el contenido en formatos que sobrevivan a la app.

### Pasos

1. **Paso 1 — FTS**: indexar título, texto, adjuntos y etiquetas; snippets; ranking; filtros.
2. **Paso 2 — UI de búsqueda**: paleta, filtros por libro/fecha/tipo, resaltado, teclado, cancelación.
3. **Paso 3 — Previsualización completa**: texto, medios, tablas, dibujos, plugins en lectura; sin edición.
4. **Paso 4 — Markdown**: exportar con assets, tablas GFM, aviso de canvas; importar con sanitización.
5. **Paso 5 — HTML y pruebas**: exportar autocontenido, posiciones absolutas, CSS, sin scripts; golden MD/HTML; 10k notas < 80 ms; importar un vault Markdown de prueba.

> PDF con canvas se implementa en 1.0 (semana 17); el MVP exporta HTML imprimible y Markdown.

### Entregables

- Búsqueda global y por libro.
- Previsualización de solo lectura.
- Exportadores MD/HTML e importador MD.
- Golden files.

### Puerta E9

- [ ] Buscar una palabra en 10k notas tarda < 80 ms.
- [ ] Los resultados muestran libro, fecha y fragmento.
- [ ] La búsqueda respeta mayúsculas/acentos según configuración.
- [ ] La previsualización reproduce medios y no edita.
- [ ] Markdown exporta texto, listas, tablas, imágenes y enlaces.
- [ ] HTML se abre sin scripts y conserva posiciones del canvas.
- [ ] Importar Markdown crea una nota válida y copia assets.
- [ ] Golden files pasan en CI.

### Verificación

```bash
cargo test -p folio-store fts
cargo test -p folio-render --all-features
cargo run -p folio-app -- export --format html --note <id> --out /tmp/nota.html
cargo run -p folio-app -- search "jardín" --json
```

### Demo

Buscar, abrir resultado, previsualizar, exportar a HTML/Markdown, importar un Markdown.

### Plan B

Si HTML con canvas es inestable, exportar Markdown y HTML en flujo; PDF queda para 1.0. Markdown siempre disponible; es el mínimo de portabilidad.

---

## E10 — Plugins

| Campo | Valor |
|---|---|
| Días | 6 (semanas 11–12) |
| Depende de | E1, E5, E9 |
| Riesgo | **Alto** |

### Objetivo

Un ecosistema real: host WASM, SDK, UI nativa, permisos, gestor y tres plugins que demuestren bloques, gráficas y diagramas. `quickbar` y `stats` se añaden en 1.0.

### Pasos

1. **Paso 1 — Spike S-04**: Wasmtime Component Model + WIT; cargar un componente trivial; medir overhead; ADR.
2. **Paso 2 — Manifiesto y paquete**: parser TOML, validación, zip, assets, firma Ed25519, instalación.
3. **Paso 3 — Host**: Store, límites, fuel/epoch, host functions, logs, errores, aislamiento.
4. **Paso 4 — UI schema**: tipos, validación, renderizador GTK, diff por ID, tema, accesibilidad.
5. **Paso 5 — Eventos y estado**: on-event, acciones, update-state, debounce, undo, permisos.
6. **Paso 6 — Bloques/comandos/SDK**: registro, inserción, paleta, `folio-plugin-sdk`, `cargo folio new/dev/build/test/pack`.
7. **Paso 7 — Ejemplos MVP**: `progress`, `chart` y `diagram`; pruebas de conformidad, permisos y rendimiento.
8. **Paso 8 — Gestor y seguridad**: UI de instalación/permisos, revocación, logs, pruebas de sandbox, congelar API 1.0.

### Entregables

- Host de plugins funcional.
- SDK Rust + plantilla.
- 3 plugins de ejemplo (`progress`, `chart`, `diagram`).
- Gestor con permisos.
- Suite de seguridad y conformidad.

### Puerta E10

- [x] Los 3 plugins del MVP funcionan en una nota real y en CI.
- [x] Instalar/desinstalar/deshabilitar desde la UI.
- [x] Un plugin sin `network` no puede hacer HTTP.
- [x] Un bucle infinito se corta sin congelar la UI.
- [x] Un panic no cierra la app.
- [x] El estado de un bloque sobrevive a desinstalar/reinstalar.
- [x] La UI del plugin respeta tema, escala y teclado.
- [x] La API 1.0 está congelada y documentada.
- [ ] Un tercero crea un plugin siguiendo la guía (prueba manual con usuario).

### Verificación

```bash
cargo test -p folio-plugins --all-features
cargo run -p folio-app -- plugin install plugins/chart/dist/chart.folio-plugin
cargo run -p folio-app -- plugin test plugins/chart
cargo test -p folio-plugins sandbox
```

### Demo

Insertar barra de progreso, gráfica que lee una tabla y diagrama DSL interactivo; revocar permiso de red y ver el fallo; matar un plugin y seguir escribiendo.

### Plan B

Si Component Model/WIT retrasa, usar ABI JSON con wasmtime core (más simple) y mantener el contrato; el SDK se adapta. Si falta tiempo, priorizar `progress` y `chart`, y mover `diagram` a 1.0; `quickbar`/`stats` ya están en 1.0.

---

## E11 — Backups, papelera y robustez

| Campo | Valor |
|---|---|
| Días | 4 (semana 13) |
| Depende de | E1, E10 |
| Riesgo | Alto |

### Objetivo

Que la bóveda sea recuperable y que ningún fallo de disco destruya datos. La bóveda cifrada se implementa en 1.0; aquí se dejan backups, papelera, versiones y reparación probados.

### Pasos

1. **Paso 1 — Amenazas y auditoría**: revisar modelo, `cargo deny`, sanitización, permisos, plugins y diseño de web viva; informe.
2. **Paso 2 — Backups**: hardlinks, retención diaria/semanal/mensual, verificación, restore a copia; CLI y UI.
3. **Paso 3 — Papelera/versiones**: retención, restauración, GC de assets, pruebas de referencia.
4. **Paso 4 — Doctor y caos**: corrupción, journal, FS lleno, solo lectura, cuarentena, `kill -9`, fuzz, restore completo.

### Entregables

- Backups verificables y restauración.
- Papelera e historial de versiones.
- `doctor` robusto.
- Informe de seguridad y caos.

### Puerta E11

- [ ] Restaurar un backup produce una bóveda idéntica (hash).
- [ ] Restaurar una nota borrada funciona.
- [ ] El historial restaura una versión sin borrar la actual.
- [ ] `kill -9` en 20 rondas no corrompe.
- [ ] Disco lleno y FS solo lectura se manejan sin pérdida.
- [ ] `cargo audit`/`deny` sin críticos.
- [ ] Las pruebas de sandbox de plugins pasan (web viva en 1.0).
- [ ] `doctor` repara índice y journal, y pone corruptos en cuarentena.

### Verificación

```bash
cargo test -p folio-store backup trash versions
./scripts/chaos.sh
./scripts/security.sh
cargo run -p folio-app -- backup verify --vault /tmp/folio-test
```

### Demo

Restaurar backup, recuperar nota borrada, restaurar versión, matar proceso, reparar bóveda.

### Plan B

Si los hardlinks no están disponibles en el FS, usar copias comprimidas con retención más corta; el backup sigue siendo verificable. El cifrado se pospone a 1.0 sin afectar a la integridad del MVP.

---

## E12 — Pulido, accesibilidad y rendimiento

| Campo | Valor |
|---|---|
| Días | 3 (semanas 13–14) |
| Depende de | Todas |
| Riesgo | Medio |

### Objetivo

Que la app se sienta terminada: rápida, accesible, coherente, sin rough edges, con onboarding y estados de error útiles.

### Pasos

1. **Paso 1 — Onboarding**: bienvenida, crear/abrir bóveda, tema, nota de bienvenida (cifrado se ofrece en 1.0).
2. **Paso 2 — Accesibilidad e i18n**: recorrido con Tab, nombres, foco, contraste, escala 200 %, lector de pantalla, canvas alternativo; revisar español, fechas, plurales y atajos.
3. **Paso 3 — Rendimiento y memoria**: perfiles de arranque, RAM, búsqueda, canvas, medios; heaptrack, fugas de widgets/texturas, LRU, cierre de WebViews.
4. **Paso 4 — Visual y errores**: tokens, espaciado, iconos, sombras, animaciones, modo oscuro, densidad; todos los estados vacíos y toasts accionables.
5. **Paso 5 — Soak test**: 2 h de uso, 1k operaciones, sin fugas ni crashes; checklist final.

### Entregables

- Onboarding.
- Checklist de accesibilidad.
- Presupuestos medidos y corregidos.
- UI pulida.
- Informe de soak.

### Puerta E12

- [ ] Arranque < 600 ms; RAM < 150 MB; búsqueda < 80 ms.
- [ ] 500 bloques a 60 fps.
- [ ] Todas las acciones por teclado; foco visible.
- [ ] Escala 200 % sin cortes.
- [ ] Soak 2 h sin fugas ni crashes.
- [ ] No quedan textos hardcodeados.
- [ ] Todos los estados de error tienen acción.
- [ ] Checklist visual revisado en claro/oscuro.

### Verificación

```bash
./scripts/perf.sh
./scripts/soak.sh
./scripts/automation/ui.sh --test a11y
```

### Demo

Recorrido completo con teclado y lector de pantalla; perfil de memoria; app en claro/oscuro.

### Plan B

Si el rendimiento no llega, recortar LOD/minimapa antes que funciones; si la accesibilidad del canvas no llega, ofrecer vista de lista de bloques como alternativa oficial.

---

## E13 — Beta, empaquetado y entrega

| Campo | Valor |
|---|---|
| Días | 4 (semana 14) |
| Depende de | E12 |
| Riesgo | Medio |

### Objetivo

Instalar Folio en una máquina limpia, iniciar la beta de 30 días y publicar una release candidate firmada del MVP.

### Pasos

1. **Paso 1 — Flatpak**: manifiesto, runtime, permisos mínimos, iconos, AppStream, build reproducible.
2. **Paso 2 — AUR**: PKGBUILD, dependencias, pruebas de instalación. `.deb` se añade en 1.0.
3. **Paso 3 — Documentación**: manual de usuario, guía de plugins, FAQ, recuperación, atajos.
4. **Paso 4 — Release CI**: tag, build, firmas, checksums, changelog, artefactos.
5. **Paso 5 — Beta inicial**: invitar 5–10 usuarios, canal beta; la beta de 30 días continúa durante las semanas 15–19.
6. **Paso 6 — Feedback**: triage P0/P1, corregir, re-test.
7. **Paso 7 — RC del MVP**: congelar, verificar checklist, publicar; 1.0 se construye sobre esta base.

### Entregables

- Flatpak y AUR.
- Documentación completa.
- Release candidate firmada.
- Informe de beta.

### Puerta E13

- [ ] Instalación limpia en Arch/GNOME sin dependencias manuales.
- [ ] Flatpak respeta sandbox y abre bóveda por portal.
- [ ] AUR instala y arranca.
- [ ] `folio --version` y `--help` correctos.
- [ ] AppStream valida.
- [ ] Backup/restore probado en una bóveda real.
- [ ] Beta inicial sin P0/P1; la beta de 30 días continúa hasta 1.0.
- [ ] Release del MVP firmada y checksums publicados.
- [ ] Guía de plugins probada por un tercero.

### Verificación

```bash
flatpak-builder --force-clean build-dir flatpak/io.github.leonisauro.Folio.yml
flatpak run io.github.leonisauro.Folio --version
appstreamcli validate flatpak/io.github.leonisauro.Folio.metainfo.xml
./scripts/release.sh --dry-run
```

### Demo

Instalación desde cero en una VM, uso diario, exportación, backup/restore, plugin instalado.

### Plan B

Si Flatpak falla, publicar AUR como canal principal y Flatpak en 1.0; no bloquear la beta. `.deb` no es requisito del MVP.

---

## 1.0 — Semanas 15–20

| Semana | Foco | Resultado |
|---|---|---|
| 15 | Estabilización + web viva | P0/P1 de beta; WebKitGTK aislado, sin cookies, bajo demanda |
| 16 | Cifrado | VK/Argon2id/XChaCha20, índice cifrado, auto-lock, recuperación |
| 17 | PDF + plugins 4–5 | Export Cairo/Pango, `quickbar`, `stats` |
| 18 | Accesibilidad + `.deb` | Auditoría, presupuestos, paquete Debian |
| 19 | Beta 1.0 | 30 días de uso, correcciones, docs, restore drill |
| 20 | Release 1.0 | Flatpak/AUR/deb, firmas, anuncio, rollback |

### Pasos de 1.0

1. **Semana 15 — Web viva**: feature flag WebKitGTK 6.0, perfil efímero, sin cookies/descargas/permisos, descarga al salir; pruebas de aislamiento y memoria.
2. **Semana 16 — Cifrado**: VK, Argon2id, XChaCha20, `zeroize`, bóveda cifrada, índice SQLCipher/FTS, auto-lock, logind, keyring, clave de recuperación; pruebas `strings` y temporales.
3. **Semana 17 — PDF y plugins**: exportador Cairo/Pango con canvas paginado; `quickbar` y `stats`; golden PDF; permisos y rendimiento.
4. **Semana 18 — Accesibilidad y deb**: auditoría con lector de pantalla y teclado, escala 200 %, contraste, movimiento reducido; `cargo-deb` y pruebas de instalación.
5. **Semana 19 — Cierre de beta**: completar los 30 días de uso real iniciados en E13, triage, migraciones, documentación de recuperación, backup/restore completo.
6. **Semana 20 — Release**: congelar, firmar, publicar Flatpak/AUR/deb, checksums, notas y plan de rollback.

Criterios de 1.0:

- [ ] Cifrado opcional estable y documentado.
- [ ] Web viva aislada, sin cookies persistentes.
- [ ] PDF con canvas y medios.
- [ ] SDK de plugins 1.0 congelado y con guía.
- [ ] 5 plugins de ejemplo publicados.
- [ ] `.deb` instalable.
- [ ] Accesibilidad auditada.
- [ ] 30 días de uso sin pérdida de datos.
- [ ] Documentación de usuario y recuperación.
- [ ] Release reproducible y firmada.

---

## 15. Ritmo semanal

| Día | Actividad |
|---|---|
| Lunes | Revisar puerta anterior; elegir etapa; desglosar en tareas ≤ 1 día; actualizar tablero |
| Martes–jueves | Implementar, probar, commit pequeño; actualizar docs |
| Viernes | Demo, checklist manual, benchmarks, retrospectiva; actualizar estado y ADRs |
| Viernes | Si la puerta no pasa: no avanzar; recortar o arreglar |

## 16. Reglas de recorte

Si una semana se retrasa:

1. Recortar `Should`, nunca `Must`.
2. Mover la función completa a 1.0, no dejar a medias.
3. Actualizar el plan y comunicarlo.
4. No acumular deuda de datos: integridad y migraciones siempre se terminan.

Orden de recorte del MVP: tablas avanzadas → presión de stylus → paneles de plugin → AUR → backfill automático → dibujo vectorial avanzado. La web viva, el PDF, el cifrado, `quickbar`/`stats` y `.deb` ya están en 1.0.

## 17. Estado de avance

Fecha de cierre del MVP: **2026-10-29**. Estado verificado con
`./scripts/automation/all.sh`, `./scripts/chaos.sh` y `./scripts/security.sh`.

| Etapa | Estado | Fecha | Notas |
|---|---|---|---|
| E0 | Implementado | 2026-10-29 | Workspace, CI, ventana GTK, Xvfb, ADR, `bench/baseline.json` |
| E1 | Implementado | 2026-10-29 | JSON atómico, orden, SQLite/FTS5, doctor, lock, watcher, trash/versiones |
| E2 | Implementado | 2026-10-29 | Sidebar, notas, paleta, búsqueda, tema, instancia única GTK |
| E3 | Implementado | 2026-10-29 | Día lógico/DST, formatos, backfill, GUI/TOML/CLI con paridad |
| E4 | Implementado | 2026-10-29 | Editor por bloques, tags, undo 100, autosave y recuperación |
| E5 | Implementado | 2026-10-29 | Flujo, anclaje por arrastre, zoom, encuadres, recomposición |
| E6 | Implementado | 2026-10-29 | Orden fraccional, DnD intra/interlibro, spring-load 650 ms, «Mover a…» |
| E7 | Implementado | 2026-10-29 | Assets BLAKE3, imagen/audio/vídeo/archivo, web card offline |
| E8 | Implementado | 2026-10-29 | Tabla editable/CSV y dibujo vectorial con export SVG en HTML |
| E9 | Implementado | 2026-10-29 | FTS5, previsualización, golden de render, importación MD |
| E10 | Implementado | 2026-10-29 | Manifiestos, permisos, firma, límites, 5 ejemplos y runtime WASM de terceros (ABI JSON, wasmi); WIT queda como contrato documental |
| E11 | Implementado | 2026-10-29 | Backups verificables, restore, papelera, versiones, `doctor`, caos |
| E12 | Implementado (auditoría automatizada) | 2026-10-29 | Foco, etiquetas, escala 200 %, contraste, alternativa al lienzo; revisión manual con lector de pantalla pendiente |
| E13 | Implementado (parcial) | 2026-10-29 | Manifiestos Flatpak/AUR, AppStream, `.deb`, pacman, canal beta y artefactos 1.0.0; Flatpak/AUR publicado y firma pendientes |

### 17.1 1.0 en curso

| Semana | Foco | Estado | Evidencia |
|---|---|---|---|
| 15 | Vista web viva aislada | Implementado con feature flag | `cargo test -p folio-ui --features webkit --test webkit`; `scripts/webkit.sh` |
| 16 | Cifrado de bóveda | Implementado | `crates/folio-store/tests/encryption.rs`; `folio vault init --encrypt`; bloqueo/desbloqueo GUI |
| 17 | PDF y plugins 4–5 | Implementado | `folio export --format pdf`; `crates/folio-render/tests/pdf_golden.rs`; 5 plugins de referencia |
| 18 | Accesibilidad y `.deb` | Implementado | `scripts/a11y.sh`; `scripts/build-deb.sh`; `.deb` verificado en árbol temporal |
| 19 | Cierre de beta | Canal preparado; invitación GitHub | `scripts/beta.sh`; `.github/workflows/beta.yml`; `docs/20` y `docs/23` |
| 20 | Release 1.0 | Artefactos RC generados | `release.sh`, `build-deb.sh`, `dist/RELEASE-1.0.0.md`; firma/beta/VM pendientes |

La vista viva usa `NetworkSession` efímera, deniega todos los permisos, cancela
descargas, no persiste cookies ni almacenamiento local y termina el proceso web al
desmontar el bloque. Se compila solo con `--features webkit`; sin la feature la app
sigue funcionando y muestra la tarjeta.

### 17.2 Desviaciones explícitas respecto al plan

Estas desviaciones están registradas como ADR en `docs/10` y no cambian el
formato de datos:

1. **Runtime de plugins**: el contrato (`Manifest` + `Request`/`Response` +
   `UiNode`) se ejecuta tanto en los cinco plugins de referencia como en módulos
   WASM de terceros mediante el ABI JSON sobre **wasmi**, con fuel, límites de
   memoria, permisos y aislamiento (ADR-033). El WIT/Component Model queda como
   contrato documental para una futura migración, no como requisito del ABI actual.
2. **Zoom del lienzo**: las posiciones y tamaños escalan y la edición funciona en
   el lienzo, pero la tipografía de widgets GTK no se re-rasteriza con el zoom en
   el MVP. El refinado visual del zoom queda para 1.0.
3. **Metadatos web**: la tarjeta web funciona con metadatos cacheados y fallback
   offline; la descarga automática de metadatos queda para 1.0. La vista viva ya
   está implementada como feature opcional (ADR-027).
4. **Cifrado**: las bóvedas nuevas usan carpetas de libro opacas; al cifrar una
   bóveda existente se conservan los nombres de carpeta para no romper enlaces
   (la fecha puede deducirse del nombre). El índice no se persiste: se reconstruye
   en memoria tras desbloquear. Los backups antiguos en claro no se convierten
   automáticamente; hay que cifrarlos o eliminarlos. `logind` y la migración
   completa quedan para el cierre de 1.0.
4. **Accesibilidad**: hay navegación por teclado, etiquetas accesibles, foco
   visible, escala 200 % probada y lista de bloques como alternativa al lienzo.
   La revisión manual con lector de pantalla sigue pendiente para el cierre.
5. **Flatpak/AUR**: los manifiestos y metadatos están; falta validar el build
   reproducible en una VM limpia (requiere `flatpak-builder` y `appstreamcli`).
6. **PDF**: el flujo se pagina y los bloques libres van a una página de lienzo
   escalada; los plugins y la web se dibujan como tarjetas estáticas, no como
   capturas interactivas. La rasterización exacta de plugins queda como refinamiento
   de 1.0.
7. **Runtime de plugins**: el ABI JSON/wasmi es el contrato ejecutable de 1.0; el
   WIT/Component Model queda documentado para una migración futura, no se exige
   para publicar la 1.0.

Actualizar esta tabla al cerrar cada puerta. Es la única fuente de verdad del progreso.


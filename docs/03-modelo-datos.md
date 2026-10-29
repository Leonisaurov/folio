# 03 — Modelo de datos y persistencia

## 1. Principios

1. **La bóveda es una carpeta normal.** Se puede copiar, sincronizar con `rsync`, inspeccionar y respaldar sin la app.
2. **El índice es desechable.** SQLite acelera; los archivos mandan.
3. **Cada archivo es atómico.** Nunca se escribe in situ.
4. **El esquema se versiona.** Toda migración es explícita, probada y reversible mediante copia.
5. **Lo desconocido se conserva.** Campos y bloques no reconocidos se mantienen al guardar; nunca se borran por ignorancia.
6. **Los IDs son estables.** Rutas, títulos y fechas cambian; el ID nunca.

## 2. Estructura de la bóveda

```text
vault/
├── vault.toml
├── .folio/
│   ├── index.sqlite
│   ├── index.sqlite-wal
│   ├── lock
│   ├── journal/
│   │   └── <op-id>.json
│   ├── backups/
│   │   └── 2026-10-29T04-00-00Z/
│   ├── trash/
│   │   └── 2026-10-29T10-12-33Z-<note-id>/
│   │       ├── meta.json
│   │       └── payload...
│   ├── quarantine/
│   ├── cache/
│   │   ├── thumbs/
│   │   └── waveforms/
│   └── plugins-data/
│       └── <plugin-id>/
├── books/
│   └── 2026-10-29_a1b2c3/
│       ├── book.toml
│       ├── assets/              # opcional, solo assets exclusivos
│       └── notes/
│           ├── 0V3k_01J9Z6...json
│           └── 0V3m_01J9Z7...json
├── assets/
│   └── ab/
│       └── cd/
│           └── <blake3>.<ext>
└── plugins/
    └── <plugin-id>/
        ├── manifest.toml
        ├── plugin.wasm
        └── signature.ed25519
```

Decisiones:

- Los assets son **globales a la bóveda** y direccionados por contenido; se comparten entre notas y libros. No se borran al borrar una nota: se recolectan por refcount cuando nadie los referencia y ha pasado la retención.
- Las notas viven **dentro del libro**. Mover una nota es mover un archivo.
- El nombre del archivo de nota contiene el orden: `<order_key>_<uuid>.json`.
- Las carpetas de libros son estables: renombrar el título no renombra la carpeta. Una acción futura «Reorganizar carpetas» puede hacerlo, pero no es automática.

## 3. Identificadores

| Entidad | Formato | Ejemplo |
|---|---|---|
| Bóveda | UUIDv7 | `018f...` |
| Libro | UUIDv7 + slug corto en carpeta | `2026-10-29_a1b2c3` |
| Nota | UUIDv7 | `01J9Z6Q3K8...` |
| Bloque | UUIDv7 | `01J9Z6R1...` |
| Asset | BLAKE3 hex (64) | `9f86d0...` |
| Operación | UUIDv7 | `01J9...` |
| Plugin | ID inverso estable | `io.github.folio.chart` |

- UUIDv7 ordena por tiempo y evita colisiones; se genera en el cliente.
- Los IDs se muestran en «Propiedades» y en enlaces `folio://note/<id>`.
- Nunca se reutiliza un ID, ni siquiera tras borrar.

## 4. `vault.toml`

```toml
schema = 1
id = "018f3c2a-7b1e-7c3a-9d2f-6a1b2c3d4e5f"
name = "Notas de León"
created_at = "2026-10-29T09:00:00Z"

[storage]
format = "json"            # json | cbor (cbor en 1.1)
fsync = "always"           # always | batched
assets = "copy"            # copy | link
max_asset_mb = 2048

[encryption]
mode = "none"              # none | password
# Los parámetros viven en .folio/keyring.json, nunca aquí.

[daily]
enabled = true
# El resto hereda de config.toml; se puede sobrescribir por bóveda.
```

## 5. `book.toml`

```toml
schema = 1
id = "01J9Z5A2..."
title = "29/10/2026"
kind = "daily"             # daily | manual
daily_date = "2026-10-29"  # solo daily; inmutable aunque se renombre
daily_timezone = "Europe/Madrid"
created_at = "2026-10-29T04:00:00+02:00"
updated_at = "2026-10-29T09:12:00+02:00"
icon = "calendar"
color = "#3584e4"
pinned = false
archived = false
sort = "manual"            # manual | title | created | modified
template = ""              # id/ruta de plantilla, opcional
```

Invariantes:

- `kind = "daily"` exige `daily_date` válido.
- `daily_date` es la clave de unicidad del libro diario; no se puede cambiar desde la UI.
- `title` puede repetirse entre libros; el `id` distingue.
- `sort` es la vista por defecto; el orden manual vive en los nombres de archivo.
- `archived = true` lo oculta de la vista principal, no lo borra.

## 6. Nota: esquema JSON

```json
{
  "schema": 1,
  "id": "01J9Z6Q3K8V5N2M1P0R7T4W6X9",
  "title": "Ideas para el jardín",
  "created_at": "2026-10-29T09:10:00+02:00",
  "updated_at": "2026-10-29T09:42:11+02:00",
  "layout": {
    "mode": "flow",
    "flow_width": 720,
    "background": "paper",
    "show_grid": false
  },
  "blocks": [
    {
      "id": "01J9Z6R1A2B3C4D5E6F7G8H9J0",
      "type": "text",
      "placement": { "kind": "flow" },
      "content": {
        "paragraphs": [
          {
            "style": "body",
            "align": "left",
            "spans": [
              { "text": "Primero, ", "marks": [] },
              { "text": "observar", "marks": ["italic"] },
              { "text": " el jardín.", "marks": [] }
            ]
          }
        ]
      }
    },
    {
      "id": "01J9Z6R2K3L4M5N6P7Q8R9S0T1",
      "type": "image",
      "placement": { "kind": "free", "x": 780, "y": 40, "w": 360, "h": 240, "z": 10 },
      "content": {
        "asset": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        "caption": "Plano del jardín",
        "alt": "Croquis a lápiz"
      }
    }
  ],
  "meta": {
    "tags": [],
    "word_count": 4,
    "properties": {}
  }
}
```

### 6.1 Reglas de serialización

- JSON UTF-8, 2 espacios, orden de claves estable (serde + `BTreeMap` para mapas).
- Sin `null` redundante; campos opcionales se omiten.
- `schema` obligatorio en raíz y en bloques de plugin.
- Los campos desconocidos se conservan con `#[serde(flatten)] extra: Map<String, Value>`.
- Los bloques desconocidos se conservan completos y se muestran como «Bloque no compatible».
- Las fechas son RFC 3339 con offset; se guarda también UTC para ordenar.
- Los textos no se escapan a HTML; se guardan como spans estructurados.

### 6.2 Límites

| Límite | Valor | Motivo |
|---|---|---|
| Tamaño de nota JSON | 50 MB (aviso > 10 MB) | Evitar bloqueos; medios van fuera |
| Bloques por nota | 5,000 (aviso > 2,000) | Rendimiento |
| Profundidad de UI de plugin | 32 | Sandbox |
| Longitud de título | 500 caracteres | UI y nombres |
| Adjunto | 2 GB configurable | Disco y memoria |

Si se supera un límite, la app avisa y sugiere dividir la nota o vincular el archivo en lugar de copiarlo.

## 7. Tipos de bloque

### 7.1 Texto

```json
{
  "type": "text",
  "content": {
    "paragraphs": [
      {
        "style": "body|h1|h2|h3|quote|code|list_item|checklist_item",
        "align": "left|center|right|justify",
        "indent": 0,
        "checked": null,
        "lang": null,
        "spans": [
          { "text": "Hola", "marks": ["bold"], "link": null, "color": null }
        ]
      }
    ]
  }
}
```

- `marks`: `bold`, `italic`, `underline`, `strikethrough`, `code`, `highlight`, `superscript`, `subscript`.
- `link` es una URL absoluta validada (`http`, `https`, `mailto`, `folio`).
- Los saltos de línea dentro de un span se representan con `\n`; los párrafos son la unidad de bloque.
- El texto vacío es válido (placeholder en UI).

### 7.2 Tabla

```json
{
  "type": "table",
  "content": {
    "rows": 3,
    "cols": 3,
    "header": true,
    "widths": [180, 180, 180],
    "cells": [
      { "row": 0, "col": 0, "paragraphs": [{ "spans": [{ "text": "Nombre", "marks": ["bold"] }] }] }
    ]
  }
}
```

- Celdas con rich text; sin fusión en MVP.
- Pegar CSV/TSV crea/expande una tabla.
- Exportación a Markdown GFM, HTML y PDF.

### 7.3 Imagen, archivo, audio, vídeo

```json
{
  "type": "image",
  "content": { "asset": "<blake3>", "caption": "", "alt": "", "width": 640, "height": null }
}
```

```json
{
  "type": "file",
  "content": { "asset": "<blake3>", "name": "contrato.pdf", "mime": "application/pdf", "size": 123456 }
}
```

```json
{
  "type": "audio",
  "content": { "asset": "<blake3>", "title": "Nota de voz", "duration_ms": 18300, "peaks": "cache:<hash>" }
}
```

```json
{
  "type": "video",
  "content": { "asset": "<blake3>", "title": "Demostración", "duration_ms": 92000, "poster": "<blake3>" }
}
```

- `asset` es obligatorio salvo modo «vincular», que guarda `external_path` y muestra un aviso de portabilidad.
- `peaks` y `poster` son cachés; si faltan, se regeneran.
- `width`/`height` son sugerencias de presentación; el aspecto se conserva.

### 7.4 Web

```json
{
  "type": "web",
  "content": {
    "url": "https://example.com/articulo",
    "mode": "card",              // card | live
    "title": "Artículo",
    "description": "Resumen…",
    "favicon": "<blake3>",
    "image": "<blake3>",
    "fetched_at": "2026-10-29T09:00:00Z",
    "live": { "allowed": false, "last_loaded_at": null }
  }
}
```

- `mode = "live"` requiere confirmación explícita y `web.live_enabled`.
- La tarjeta funciona sin red si ya hay metadatos cacheados.
- La URL se valida; se rechazan `file:`, `javascript:`, `data:`.

### 7.5 Dibujo

```json
{
  "type": "drawing",
  "content": {
    "version": 1,
    "width": 1200,
    "height": 800,
    "background": "transparent",
    "strokes": [
      {
        "tool": "pen",
        "color": "#1a1a1a",
        "width": 2.5,
        "points": [[10.0, 20.0, 0.5, 0], [11.2, 21.4, 0.6, 8]]
      }
    ]
  }
}
```

- `points`: `[x, y, presión (0..1), tiempo_ms]`; la presión es opcional.
- Herramientas: `pen`, `highlighter`, `eraser`.
- El borrador elimina trazos completos en MVP; borrado por punto en 1.0.
- Exportación SVG conserva vectores; PNG rasteriza.

### 7.6 Plugin

```json
{
  "type": "plugin",
  "content": {
    "plugin": "io.github.folio.chart",
    "block": "chart",
    "schema": 1,
    "state": { "kind": "bar", "source": "table:01J9...", "title": "Ventas" },
    "height": 280
  }
}
```

- `state` es opaco para el host, pero validado por tamaño (≤ 1 MB) y JSON.
- Si el plugin no está instalado, se muestra un marcador con el nombre y un botón «Buscar plugin»; el estado se conserva.
- Si el plugin falla, se muestra el último render exitoso en caché si existe, o un error con «Desactivar plugin».

### 7.7 Bloques reservados para el futuro

`callout`, `toggle`, `columns`, `embed`, `reference`, `pdf_annotation`. El cargador los trata como desconocidos y los preserva. No se implementan en MVP.

## 8. Orden de notas

### 8.1 Clave fraccional

- `order_key` es una cadena base62 generada entre dos claves existentes; permite insertar sin renumerar.
- El nombre del archivo es `<order_key>_<uuid>.json`; ordenar el directorio por nombre da el orden manual.
- Al mover una nota:
  - dentro del libro: se calcula una clave entre sus vecinos y se renombra el archivo;
  - entre libros: se calcula la clave en destino y se hace `rename` al directorio destino con el nuevo nombre.
- El `rename` es atómico en el mismo sistema de archivos. La bóveda debe vivir en un solo FS; si no, se detecta y se usa copia + journal.

### 8.2 Invariantes

- Dos notas nunca comparten `order_key` en el mismo libro.
- El orden manual es independiente de `title`, `created_at` y `modified_at`.
- Cambiar a orden por título no renombra archivos.
- Volver a manual conserva el orden previo.
- Si el usuario edita nombres de archivo a mano, el índice detecta duplicados/órdenes inválidos y `doctor` repara.

### 8.3 Casos límite

| Caso | Comportamiento |
|---|---|
| Libro vacío | `order_key = "a0"` |
| Insertar al principio | clave anterior a la primera |
| Insertar al final | clave posterior a la última |
| Clave agotada | rebalanceo del libro (renombrar todas) con journal y backup |
| Colisión | sufijo aleatorio y rebalanceo |
| Nota movida mientras está abierta | se actualiza ruta en el editor; no se pierde el buffer |
| Crash a mitad de move | journal permite completar o revertir; nunca duplica |

## 9. Escritura atómica y journal

### 9.1 Archivo individual

```rust
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap();
    let tmp = dir.join(format!(".tmp-{}", Uuid::now_v7()));
    let mut f = File::create(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    fs::rename(&tmp, path)?;
    File::open(dir)?.sync_all()?; // fsync del directorio
    Ok(())
}
```

- En bóvedas cifradas se cifra antes de escribir; el temporal también está cifrado.
- El nombre temporal nunca termina en `.json` para que el índice no lo confunda.

### 9.2 Journal de operaciones

```json
{
  "id": "01J9...",
  "op": "move_note",
  "state": "prepared",
  "created_at": "...",
  "steps": [
    { "kind": "rename", "from": "books/A/notes/0V3k_id.json", "to": "books/B/notes/0V3m_id.json" },
    { "kind": "index", "note": "id" }
  ]
}
```

- Se escribe antes de empezar.
- Cada paso es idempotente: si el destino ya existe y el origen no, se considera hecho.
- Al arrancar, `doctor` completa o revierte según el estado y la existencia de archivos.
- Los journals completados se archivan 7 días para diagnóstico.

## 10. Índice SQLite

### 10.1 Esquema

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

CREATE TABLE meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE books (
  id TEXT PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  kind TEXT NOT NULL,
  daily_date TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  pinned INTEGER NOT NULL DEFAULT 0,
  archived INTEGER NOT NULL DEFAULT 0,
  sort TEXT NOT NULL DEFAULT 'manual',
  icon TEXT,
  color TEXT,
  mtime_ns INTEGER NOT NULL,
  size INTEGER NOT NULL
);
CREATE UNIQUE INDEX books_daily ON books(daily_date) WHERE kind = 'daily';

CREATE TABLE notes (
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
  path TEXT NOT NULL UNIQUE,
  order_key TEXT NOT NULL,
  title TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  word_count INTEGER NOT NULL DEFAULT 0,
  block_count INTEGER NOT NULL DEFAULT 0,
  has_media INTEGER NOT NULL DEFAULT 0,
  pinned INTEGER NOT NULL DEFAULT 0,
  archived INTEGER NOT NULL DEFAULT 0,
  mtime_ns INTEGER NOT NULL,
  size INTEGER NOT NULL
);
CREATE INDEX notes_book_order ON notes(book_id, order_key);
CREATE INDEX notes_updated ON notes(updated_at DESC);

CREATE TABLE blocks (
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  block_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  text TEXT,
  asset TEXT,
  plugin TEXT,
  PRIMARY KEY (note_id, block_id)
);

CREATE TABLE assets (
  hash TEXT PRIMARY KEY,
  path TEXT NOT NULL,
  size INTEGER NOT NULL,
  mime TEXT,
  refcount INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE note_assets (
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  hash TEXT NOT NULL,
  PRIMARY KEY (note_id, hash)
);

CREATE TABLE plugins (
  id TEXT PRIMARY KEY,
  version TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  permissions TEXT NOT NULL,
  installed_at TEXT NOT NULL
);

CREATE VIRTUAL TABLE notes_fts USING fts5(
  note_id UNINDEXED,
  title,
  body,
  tags,
  tokenize = 'unicode61 remove_diacritics 2'
);
```

### 10.2 Consultas típicas

```sql
-- Notas de un libro en orden manual
SELECT * FROM notes WHERE book_id = ? ORDER BY order_key;

-- Búsqueda
SELECT n.id, n.title, snippet(notes_fts, 2, '<mark>', '</mark>', '…', 24)
FROM notes_fts f JOIN notes n ON n.id = f.note_id
WHERE notes_fts MATCH ? ORDER BY rank LIMIT 50;

-- Libro del día
SELECT * FROM books WHERE kind = 'daily' AND daily_date = ?;
```

### 10.3 Reconstrucción

`folio index rebuild`:

1. Marca `meta.index_state = rebuilding`.
2. Recorre `books/*/book.toml` y `notes/*.json`.
3. Valida y extrae texto.
4. Inserta en una base temporal `index.rebuild.sqlite`.
5. Al terminar, `rename` sobre `index.sqlite` y `fsync`.
6. Si falla, conserva el índice anterior y reporta.

La app puede seguir abriendo notas durante la reconstrucción; la búsqueda usa el índice anterior hasta el swap.

## 11. Assets y recolección

- Importar: calcular BLAKE3, comprobar existencia, copiar si falta, registrar refcount.
- Borrar nota: decrementar refcount; si llega a 0, el asset va a una cola de recolección con retención de 30 días.
- `folio doctor --gc` elimina assets sin referencias y con retención cumplida; pide confirmación.
- Assets huérfanos por crash se detectan comparando `assets` con `note_assets`.
- Nunca se borra un asset referenciado por una versión histórica activa.

## 12. Papelera y versiones

### 12.1 Papelera

- Borrar nota/libro mueve a `.folio/trash/<timestamp>-<id>/` con `meta.json` (ruta original, tipo, título, fecha).
- Restaurar devuelve a la ruta original; si el libro ya no existe, se recrea o se ofrece elegir destino.
- Retención por defecto 30 días; purga al arrancar y desde ajustes.
- Vaciar papelera pide confirmación y no toca assets hasta que no haya referencias.

### 12.2 Historial de versiones

- Snapshot completo comprimido (zstd) en `.folio/versions/<note-id>/<timestamp>.json.zst`.
- Se crea al guardar si pasaron ≥ 5 minutos desde el último snapshot o al cerrar la nota.
- Se conservan 50 versiones o 30 días, lo que sea mayor; configurable.
- Restaurar crea una versión nueva (no borra la actual) y es undoable.
- El historial no se indexa; se puede buscar por fecha.

### 12.3 Backups

- Snapshot por **hardlinks** en `.folio/backups/<timestamp>/`; solo los archivos nuevos o modificados ocupan espacio.
- Frecuencia por defecto: diaria al abrir y manual desde menú/CLI.
- Retención: 7 diarios + 4 semanales + 12 mensuales (configurable).
- En bóvedas cifradas, los backups permanecen cifrados; no se guarda la clave.
- Restaurar un backup se hace sobre una copia, nunca in-place sin confirmación.
- `folio backup verify` comprueba hashes y que el árbol sea restaurable.

## 13. Cifrado (diseño)

> Implementación en 1.0 (semana 16); el formato se define aquí para no romper compatibilidad.

### 13.1 Claves

- Clave de bóveda `VK`: 32 bytes aleatorios.
- `KEK = Argon2id(password, salt, m=64 MiB, t=3, p=4)`.
- `VK` se guarda envuelta con XChaCha20-Poly1305 en `.folio/keyring.json`:

```json
{
  "version": 1,
  "kdf": "argon2id",
  "salt": "base64",
  "m": 65536, "t": 3, "p": 4,
  "nonce": "base64",
  "wrapped_key": "base64"
}
```

- La contraseña nunca se guarda. Opcionalmente, `VK` se guarda en el keyring del sistema (Secret Service) para «recordar hasta cerrar sesión».

### 13.2 Contenido

- Cada archivo se cifra con XChaCha20-Poly1305 usando una subclave derivada de `VK` y un nonce aleatorio; el formato incluye `version`, `nonce`, `ciphertext`.
- Los nombres de archivo y carpetas de libros son opacos en bóvedas cifradas (`books/<id>/notes/<id>.enc`), para no filtrar fechas ni títulos.
- El índice se guarda con SQLCipher o con un contenedor cifrado; FTS5 se usa tras desbloquear.
- Los assets se cifran por bloques de 1 MiB con nonce por bloque; el hash BLAKE3 se calcula sobre el texto plano y se usa como ID, pero no se revela en el nombre.
- La configuración no secreta puede quedar en claro; los títulos y fechas se consideran sensibles y se cifran.

### 13.3 Bloqueo

- Auto-bloqueo tras N minutos de inactividad (15 por defecto) y al recibir `Lock` de logind.
- Al bloquear se vacían buffers, se cierran WebViews y se descartan claves con `zeroize`.
- Al desbloquear se reindexa en memoria si hace falta.
- Recuperación: clave de recuperación de 24 palabras generada al crear la bóveda; se muestra una vez y se verifica. Sin ella, una contraseña perdida implica datos perdidos.

## 14. Migraciones

- `schema` en cada archivo y en el índice.
- Migración al abrir: si `schema < actual`, se migra en memoria; al guardar se escribe la versión nueva.
- Migración masiva de bóveda: comando explícito, backup previo, journal y verificación.
- Nunca se abre una bóveda con `schema` mayor al soportado; se muestra «Actualiza Folio».
- Cada migración tiene test de fixture antiguo → nuevo y roundtrip.
- Los campos desconocidos se conservan; una migración no puede descartarlos sin decisión explícita.

## 15. Conflictos y edición externa

| Escenario | Comportamiento |
|---|---|
| Archivo editado fuera, nota cerrada | Watcher reindexa; la lista se actualiza |
| Archivo editado fuera, nota abierta sin cambios | Recarga silenciosa con toast |
| Archivo editado fuera, nota con cambios | Diálogo: recargar, conservar y guardar copia, comparar |
| Dos instancias | La segunda abre en solo lectura o activa la ventana existente |
| Archivo borrado fuera | La nota abierta pasa a «huérfana»; se ofrece guardar como nueva |
| Asset borrado fuera | Marcador «archivo no encontrado»; opción de reemplazar |
| Reloj cambia de zona | Se recalcula el día lógico; no se crean duplicados |

## 16. Invariantes y pruebas de datos

1. Un libro diario por `daily_date`.
2. Una nota pertenece a exactamente un libro.
3. Dos notas no comparten `order_key` en un libro.
4. Todo bloque tiene ID único dentro de la nota.
5. Todo asset referenciado existe o se reporta.
6. `updated_at` ≥ `created_at`.
7. El índice se puede borrar y reconstruir sin pérdida.
8. Un guardado interrumpido no deja el archivo final a medias.
9. Las versiones históricas referencian assets existentes.
10. Un plugin ausente no elimina su bloque.

Cada invariante tiene una prueba en `folio-core` o `folio-store` y, cuando aplica, una prueba de caos.

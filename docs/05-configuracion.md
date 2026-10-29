# 05 — Configuración por texto y GUI

## 1. Objetivo

Que **todo** lo configurable se pueda cambiar de tres formas equivalentes:

1. Editando un archivo TOML comentado.
2. Usando la GUI de ajustes, generada del mismo esquema.
3. Usando la CLI (`folio config set`).

Sin duplicar definiciones, sin perder comentarios y sin reiniciar salvo que el campo lo exija.

## 2. Archivos y precedencia

| Nivel | Ruta | Uso |
|---|---|---|
| Flags CLI | `--daily.rollover-hour 4` | Pruebas, scripts, override puntual |
| Entorno | `FOLIO_DAILY_ROLLOVER_HOUR=4` | Sesiones y CI |
| Bóveda | `<vault>/vault.toml` | Ajustes propios de esa bóveda |
| Global | `$XDG_CONFIG_HOME/folio/config.toml` | Preferencias del usuario |
| Defectos | Compilados | Siempre válidos |

Precedencia: **flags > entorno > bóveda > global > defectos**.

- El archivo global se crea comentado en el primer arranque.
- `vault.toml` solo contiene lo que difiere; no se duplica todo.
- Los secretos (contraseñas, tokens de sync) van al keyring, nunca a TOML.
- `FOLIO_CONFIG` permite usar otro archivo global; `FOLIO_VAULT` abre una bóveda concreta.

## 3. Esquema único

`folio-config` define un `ConfigSchema` con, por campo:

```rust
struct Field {
    path: &'static str,          // "daily.rollover_hour"
    ty: FieldType,               // bool, int, float, string, enum, path, duration, color
    default: Value,
    label: &'static str,         // "Hora de corte del día"
    help: &'static str,
    group: &'static str,         // "Libros diarios"
    range: Option<(Value, Value)>,
    choices: &'static [&'static str],
    advanced: bool,
    restart: bool,
    scope: Scope,                // global | vault | both
}
```

De este esquema se generan:

- Validación tipada al cargar.
- Formulario GTK (widget según tipo, búsqueda por etiqueta/ruta).
- `config.schema.json` (JSON Schema) para editores y CI.
- Ayuda de CLI y `--help` por campo.
- Página de documentación de referencia.
- Prueba de paridad: ninguna clave puede existir sin metadatos de GUI.

## 4. Carga, validación y recarga

1. Leer global; si no existe, crear con comentarios.
2. Leer `vault.toml`; aplicar overrides.
3. Aplicar entorno y flags.
4. Validar tipos, rangos, rutas, patrones y combinaciones.
5. Si hay error: **no aplicar**; conservar la última configuración válida; mostrar banner con archivo, línea y clave.
6. Si es válida: `ArcSwap` y emitir `ConfigChanged`; la UI se actualiza.
7. Watcher con debounce de 250 ms; ignorar escrituras propias.
8. Al guardar desde GUI/CLI: leer el TOML actual con `toml_edit`, modificar solo las claves cambiadas, preservar comentarios/orden, escribir atómico, validar y recargar.

### 4.1 Ejemplo de error

```text
vault.toml: 14:3
daily.rollover_hour = 25
Error: debe estar entre 0 y 23.
La configuración anterior sigue activa.
[abrir archivo] [restaurar valor]
```

### 4.2 Cambios que requieren reinicio

Solo cuatro: ruta de la bóveda, backend de índice, modo de cifrado y tema de iconos del sistema. El resto se aplica en vivo.

## 5. Interfaz de ajustes

- Buscador superior: filtra por etiqueta, ruta y descripción.
- Secciones colapsables; los campos avanzados ocultos tras «Mostrar avanzado».
- Cada campo muestra su ruta TOML al hover y un botón «Copiar ruta».
- Botón «Ver en TOML» abre el editor integrado en la línea correspondiente.
- Editor TOML integrado con resaltado, números de línea, validación al vuelo y botón «Guardar».
- «Abrir en editor externo» usa `$EDITOR` (o `xdg-open`).
- «Restaurar sección» y «Restaurar todo» con confirmación.
- Exportar/importar configuración sin secretos.
- Indicador «Global» / «Esta bóveda» junto a cada campo.
- Los cambios se aplican al confirmar; los sliders en vivo solo para zoom/tema.

## 6. Referencia de claves

> Los valores mostrados son los **defectos**. `scope`: G = global, V = bóveda, GV = ambos.

### 6.1 General

| Clave | Tipo | Defecto | Scope | Descripción |
|---|---|---|---|---|
| `app.language` | enum `es`,`en` | `es` | G | Idioma de la interfaz |
| `app.start_view` | enum `today`,`last`,`library` | `today` | G | Vista al abrir |
| `app.confirm_quit` | bool | `false` | G | Confirmar al salir con cambios |
| `app.single_instance` | bool | `true` | G | Reutilizar la ventana existente |
| `app.restore_window` | bool | `true` | G | Restaurar tamaño/posición |
| `app.check_updates` | bool | `true` | G | Avisar de versiones (sin telemetría) |

### 6.2 Apariencia

| Clave | Tipo | Defecto | Scope | Descripción |
|---|---|---|---|---|
| `ui.theme` | enum `system`,`light`,`dark` | `system` | G | Tema |
| `ui.accent` | enum `blue`,`teal`,`green`,`yellow`,`orange`,`red`,`pink`,`purple` | `blue` | G | Color de acento |
| `ui.density` | enum `comfortable`,`compact` | `comfortable` | G | Densidad de listas |
| `ui.sidebar_width` | int 200–400 | `280` | G | Ancho de barra lateral |
| `ui.show_word_count` | bool | `true` | G | Contador de palabras |
| `ui.reduce_motion` | bool | `false` | G | Desactivar animaciones |
| `ui.font_ui` | string | `""` | G | Fuente de UI (vacío = sistema) |
| `ui.font_editor` | string | `""` | G | Fuente del editor (vacío = sistema) |
| `ui.editor_font_size` | int 12–28 | `16` | G | Tamaño base del editor |
| `ui.line_height` | float 1.2–2.0 | `1.5` | G | Interlineado |
| `ui.editor_width` | int 560–960 | `720` | V | Ancho de la columna de flujo |
| `ui.paper_texture` | bool | `false` | G | Textura de papel sutil |

### 6.3 Libros diarios

| Clave | Tipo | Defecto | Scope | Descripción |
|---|---|---|---|---|
| `daily.enabled` | bool | `true` | V | Crear libros diarios |
| `daily.create` | enum `first_open`,`first_note`,`manual` | `first_open` | V | Cuándo crear el libro del día |
| `daily.title_format` | string | `"%d/%m/%Y"` | V | Título inicial; acepta `strftime` |
| `daily.folder_format` | string | `"%Y-%m-%d"` | V | Carpeta; debe ser segura para FS |
| `daily.timezone` | string | `"local"` | V | Zona IANA o `local` |
| `daily.day_start_hour` | int 0–23 | `4` | V | Hora de corte del día lógico |
| `daily.backfill` | enum `manual`,`ask`,`auto`,`never` | `manual` | V | Relleno de días faltantes |
| `daily.recreate_deleted` | bool | `false` | V | Recrear si el libro del día fue borrado |
| `daily.template` | string (ruta) | `""` | V | Nota plantilla opcional |
| `daily.auto_archive_days` | int 0–3650 | `0` | V | Archivar tras N días (0 = nunca) |
| `daily.start_of_week` | enum `monday`,`sunday` | `monday` | G | Calendario |
| `daily.locale` | string | `"es-ES"` | G | Nombres de mes/día |

Ejemplos de formato:

```toml
# Título: 29/10/2026, carpeta: 2026-10-29
title_format = "%d/%m/%Y"
folder_format = "%Y-%m-%d"

# Título: jueves, 29 de octubre de 2026
title_format = "%A, %d de %B de %Y"

# Título: 2026-10-29 · Semana 44
title_format = "%Y-%m-%d · Semana %V"
```

`jiff` aporta zonas y DST; los nombres localizados usan ICU/`daily.locale`. Si el formato produce un título vacío o una carpeta inválida, se usa el fallback seguro y se avisa.

### 6.4 Notas y orden

| Clave | Tipo | Defecto | Scope | Descripción |
|---|---|---|---|---|
| `notes.default_sort` | enum `manual`,`title`,`created`,`modified` | `manual` | V | Orden por defecto del libro |
| `notes.sort_direction` | enum `asc`,`desc` | `asc` | V | Dirección |
| `notes.confirm_delete` | bool | `true` | G | Confirmar borrado |
| `notes.trash_days` | int 0–365 | `30` | V | Retención de papelera |
| `notes.version_days` | int 0–3650 | `30` | V | Retención de versiones |
| `notes.version_max` | int 0–500 | `50` | V | Máximo de versiones por nota |
| `notes.autosave_ms` | int 200–5000 | `800` | G | Debounce de autosave |
| `notes.recovery_ms` | int 500–10000 | `2000` | G | Snapshot de recuperación |
| `notes.title_from_first_line` | bool | `true` | G | Derivar título si está vacío |
| `notes.max_note_mb` | int 1–200 | `50` | V | Aviso de nota grande |
| `notes.default_template` | string | `""` | V | Plantilla de nota nueva |

### 6.5 Orden y arrastre

| Clave | Tipo | Defecto | Scope | Descripción |
|---|---|---|---|---|
| `drag.spring_delay_ms` | int 200–2000 | `650` | G | Retardo para abrir libro |
| `drag.autoscroll` | bool | `true` | G | Auto-scroll al arrastrar |
| `drag.position_picker` | bool | `true` | G | Selector de posición |
| `drag.haptic_feedback` | bool | `false` | G | Vibración si el hardware lo soporta |
| `drag.show_drop_preview` | bool | `true` | G | Vista previa de la nota |

### 6.6 Editor y lienzo

| Clave | Tipo | Defecto | Scope | Descripción |
|---|---|---|---|---|
| `editor.spellcheck` | bool | `false` | G | Corrector ortográfico |
| `editor.spellcheck_lang` | string | `"es"` | G | Diccionario |
| `editor.markdown_shortcuts` | bool | `true` | G | Atajos tipo Markdown |
| `editor.slash_menu` | bool | `true` | G | Menú `/` |
| `editor.paste_web_as_card` | bool | `true` | G | URL pegada → tarjeta |
| `editor.typewriter_scroll` | bool | `false` | G | Mantener cursor centrado |
| `canvas.snap` | bool | `true` | G | Ajuste a guías |
| `canvas.grid` | bool | `false` | G | Rejilla visible |
| `canvas.grid_size` | int 4–64 | `8` | G | Tamaño de rejilla |
| `canvas.zoom_min` | float 0.1–1.0 | `0.25` | G | Zoom mínimo |
| `canvas.zoom_max` | float 1.0–8.0 | `4.0` | G | Zoom máximo |
| `canvas.show_anchored_in_document` | bool | `true` | G | Ver anclados en encuadre documento |
| `canvas.default_mode` | enum `document`,`canvas` | `document` | V | Vista inicial |
| `canvas.max_blocks` | int 100–20000 | `5000` | V | Aviso de nota enorme |

### 6.7 Medios

| Clave | Tipo | Defecto | Scope | Descripción |
|---|---|---|---|---|
| `media.copy_into_vault` | bool | `true` | V | Copiar o vincular |
| `media.max_asset_mb` | int 1–20480 | `2048` | V | Tamaño máximo |
| `media.image_max_px` | int 512–16384 | `4096` | G | Redimensionado de imágenes |
| `media.thumbnail_cache_mb` | int 32–2048 | `256` | G | Caché de miniaturas |
| `media.autoplay` | bool | `false` | G | Reproducir al abrir |
| `media.pause_offscreen` | bool | `true` | G | Pausar fuera de pantalla |
| `media.video_poster` | bool | `true` | G | Generar póster |
| `media.waveform` | bool | `true` | G | Generar onda |

### 6.8 Web

| Clave | Tipo | Defecto | Scope | Descripción |
|---|---|---|---|---|
| `web.enabled` | bool | `true` | V | Permite bloques web |
| `web.default_mode` | enum `card`,`live` | `card` | V | Modo por defecto |
| `web.live_enabled` | bool | `true` | V | Permitir vista viva |
| `web.fetch_metadata` | enum `never`,`ask`,`always` | `ask` | V | Metadatos de enlaces |
| `web.persist_cookies` | bool | `false` | V | Cookies persistentes (no recomendado) |
| `web.allow_downloads` | bool | `false` | V | Descargas desde la vista viva |
| `web.timeout_s` | int 3–60 | `15` | G | Timeout de metadatos |
| `web.max_metadata_kb` | int 32–2048 | `512` | G | Tamaño máximo de respuesta |

### 6.9 Plugins

| Clave | Tipo | Defecto | Scope | Descripción |
|---|---|---|---|---|
| `plugins.enabled` | bool | `true` | V | Activar plugins |
| `plugins.dev_mode` | bool | `false` | G | Recarga en vivo y logs |
| `plugins.allow_unsigned` | bool | `false` | V | Permitir plugins sin firma |
| `plugins.max_memory_mb` | int 8–512 | `64` | V | Memoria por plugin |
| `plugins.max_fuel` | int | `10000000` | V | Instrucciones por llamada |
| `plugins.timeout_ms` | int 10–5000 | `2000` | V | Timeout de comando |
| `plugins.network_default` | bool | `false` | V | Red por defecto |
| `plugins.auto_update` | bool | `false` | V | Actualizar desde archivo (1.0) |

### 6.10 Bóveda, copias y seguridad

| Clave | Tipo | Defecto | Scope | Descripción |
|---|---|---|---|---|
| `vault.name` | string | — | V | Nombre visible |
| `vault.encryption` | enum `none`,`password` | `none` | V | Cifrado (reinicio) |
| `vault.auto_lock_min` | int 1–120 | `15` | V | Bloqueo por inactividad |
| `vault.lock_on_suspend` | bool | `true` | V | Bloquear al suspender |
| `vault.backup_enabled` | bool | `true` | V | Backups automáticos |
| `vault.backup_daily` | bool | `true` | V | Backup al abrir una vez al día |
| `vault.backup_keep_daily` | int 0–60 | `7` | V | Retención diaria |
| `vault.backup_keep_weekly` | int 0–52 | `4` | V | Retención semanal |
| `vault.backup_keep_monthly` | int 0–120 | `12` | V | Retención mensual |
| `vault.read_only_on_lock` | bool | `true` | V | Solo lectura si otra instancia |

### 6.11 Atajos

```toml
[keybindings]
new_note = "<Control>n"
new_book = "<Control><Shift>n"
palette = "<Control>k"
search = "<Control>p"
today = "<Control><Shift>h"
preview = "<Control><Shift>e"
focus = "<Control><Shift>f"
move_note = "<Control><Shift>m"
fit_document = "<Control>1"
fit_all = "<Control>2"
```

- Formato GTK accelerator.
- Se validan conflictos; si dos acciones comparten atajo, la UI muestra cuál gana y permite resolver.
- `config set` valida el formato.

### 6.12 Avanzado

| Clave | Tipo | Defecto | Descripción |
|---|---|---|---|
| `advanced.log_level` | enum `error`,`warn`,`info`,`debug`,`trace` | `info` | Nivel de log |
| `advanced.log_retention_days` | int 1–90 | `14` | Retención |
| `advanced.log_content` | bool | `false` | Incluir texto de notas en logs (no recomendado) |
| `advanced.index_batch` | int 10–1000 | `100` | Tamaño de lote |
| `advanced.watch_debounce_ms` | int 50–5000 | `300` | Debounce del watcher |
| `advanced.telemetry` | bool | `false` | Sin telemetría en MVP |
| `advanced.experimental` | bool | `false` | Funciones inestables |

## 7. CLI

```bash
folio config path
folio config get daily.rollover_hour
folio config get daily --json
folio config set daily.rollover_hour 4
folio config set daily.title_format "%Y/%m/%d"
folio config unset daily.template
folio config edit                 # $EDITOR
folio config validate
folio config schema > folio.schema.json
folio config diff                 # compara con defaults
folio --daily.rollover-hour 4      # override de sesión
FOLIO_UI_THEME=dark folio
```

- `get` sin argumentos imprime toda la configuración efectiva con su origen (global/bóveda/env/defecto).
- `set` preserva comentarios, valida y aplica en vivo si la app está abierta (D-Bus).
- `edit` bloquea hasta que el editor cierre y valida al volver.
- Códigos de salida: 0 éxito, 2 error de validación, 3 bóveda bloqueada, 4 no encontrada.

## 8. Paridad y pruebas

1. **Prueba de esquema**: recorre todos los campos y exige etiqueta, ayuda, grupo y tipo.
2. **Prueba de GUI**: para cada campo, el formulario genera un widget y `set`/`get` hace roundtrip.
3. **Prueba de TOML**: escribir desde GUI preserva comentarios y claves desconocidas.
4. **Prueba de recarga**: archivo inválido no cambia la configuración activa.
5. **Prueba de precedencia**: flags > env > bóveda > global > defecto.
6. **Prueba de documentación**: toda clave aparece en esta tabla.
7. **Prueba de migración**: config de versión anterior se acepta y se reescribe.

## 9. Decisiones de configuración

- No hay configuración en una base de datos opaca: el TOML es la fuente.
- La GUI nunca reescribe el archivo entero: solo las claves modificadas.
- Los valores por defecto se documentan y se muestran en la UI.
- Los patrones de fecha se validan con un ejemplo en vivo («Hoy sería: 29/10/2026»).
- Las rutas se expanden (`~`, variables de entorno) y se canonicalizan; se rechazan rutas relativas fuera de la bóveda.
- Los cambios peligrosos (cifrado, borrar backups) piden confirmación y explican consecuencias.

## 10. Ejemplo completo mínimo

```toml
# ~/.config/folio/config.toml
[app]
language = "es"
start_view = "today"

[ui]
theme = "dark"
accent = "teal"
editor_font_size = 17
editor_width = 760

[daily]
enabled = true
create = "first_open"
title_format = "%d/%m/%Y"
folder_format = "%Y-%m-%d"
day_start_hour = 4
timezone = "Europe/Madrid"

[notes]
default_sort = "manual"
autosave_ms = 800

[drag]
spring_delay_ms = 650

[plugins]
allow_unsigned = false

[keybindings]
palette = "<Control>k"
```

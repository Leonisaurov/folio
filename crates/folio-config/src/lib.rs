//! Configuración única de Folio: esquema, TOML, precedencia y validación.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};
use toml_edit::{DocumentMut, Item, Table};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Bool,
    Int,
    Float,
    String,
    Enum,
    Path,
    Color,
    Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Global,
    Vault,
    Both,
}

#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub path: String,
    pub ty: FieldType,
    pub default: Value,
    pub label: String,
    pub help: String,
    pub group: String,
    pub choices: Vec<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub advanced: bool,
    pub restart: bool,
    pub scope: Scope,
}

fn base(path: &str, ty: FieldType, default: Value, label: &str, group: &str) -> Field {
    Field {
        path: path.into(),
        ty,
        default,
        label: label.into(),
        help: label.into(),
        group: group.into(),
        choices: Vec::new(),
        min: None,
        max: None,
        advanced: false,
        restart: false,
        scope: Scope::Global,
    }
}

fn ranged(mut field: Field, min: f64, max: f64) -> Field {
    field.min = Some(min);
    field.max = Some(max);
    field
}

fn vault(mut field: Field) -> Field {
    field.scope = Scope::Vault;
    field
}

fn both(mut field: Field) -> Field {
    field.scope = Scope::Both;
    field
}

fn advanced(mut field: Field) -> Field {
    field.advanced = true;
    field
}

fn choices(mut field: Field, values: &[&str]) -> Field {
    field.choices = values.iter().map(|value| (*value).to_string()).collect();
    field
}

/// Esquema único del que se generan GUI, CLI, validación y documentación.
pub fn schema() -> Vec<Field> {
    use FieldType::*;
    vec![
        // General
        base(
            "app.language",
            Enum,
            json!("es"),
            "Idioma de la interfaz",
            "General",
        )
        .pipe(|f| choices(f, &["es", "en"])),
        base(
            "app.start_view",
            Enum,
            json!("today"),
            "Vista al abrir",
            "General",
        )
        .pipe(|f| choices(f, &["today", "last", "library"])),
        base(
            "app.confirm_quit",
            Bool,
            json!(false),
            "Confirmar al salir con cambios",
            "General",
        ),
        base(
            "app.single_instance",
            Bool,
            json!(true),
            "Reutilizar la ventana existente",
            "General",
        ),
        base(
            "app.restore_window",
            Bool,
            json!(true),
            "Restaurar tamaño y posición",
            "General",
        ),
        base(
            "app.check_updates",
            Bool,
            json!(true),
            "Avisar de versiones sin telemetría",
            "General",
        ),
        // Apariencia
        base("ui.theme", Enum, json!("system"), "Tema", "Apariencia")
            .pipe(|f| choices(f, &["system", "light", "dark"])),
        base(
            "ui.accent",
            Enum,
            json!("blue"),
            "Color de acento",
            "Apariencia",
        )
        .pipe(|f| {
            choices(
                f,
                &[
                    "blue", "teal", "green", "yellow", "orange", "red", "pink", "purple",
                ],
            )
        }),
        base(
            "ui.density",
            Enum,
            json!("comfortable"),
            "Densidad de listas",
            "Apariencia",
        )
        .pipe(|f| choices(f, &["comfortable", "compact"])),
        base(
            "ui.sidebar_width",
            Int,
            json!(280),
            "Ancho de la barra lateral",
            "Apariencia",
        )
        .pipe(|f| ranged(f, 200.0, 400.0)),
        base(
            "ui.show_word_count",
            Bool,
            json!(true),
            "Mostrar contador de palabras",
            "Apariencia",
        ),
        base(
            "ui.reduce_motion",
            Bool,
            json!(false),
            "Reducir animaciones",
            "Apariencia",
        ),
        base(
            "ui.font_ui",
            String,
            json!(""),
            "Fuente de la interfaz",
            "Apariencia",
        ),
        base(
            "ui.font_editor",
            String,
            json!(""),
            "Fuente del editor",
            "Apariencia",
        ),
        base(
            "ui.editor_font_size",
            Int,
            json!(16),
            "Tamaño base del editor",
            "Apariencia",
        )
        .pipe(|f| ranged(f, 12.0, 28.0)),
        base(
            "ui.line_height",
            Float,
            json!(1.5),
            "Interlineado",
            "Apariencia",
        )
        .pipe(|f| ranged(f, 1.2, 2.0)),
        vault(base(
            "ui.editor_width",
            Int,
            json!(720),
            "Ancho de la columna de flujo",
            "Apariencia",
        ))
        .pipe(|f| ranged(f, 560.0, 960.0)),
        base(
            "ui.paper_texture",
            Bool,
            json!(false),
            "Textura de papel sutil",
            "Apariencia",
        ),
        // Libros diarios
        vault(base(
            "daily.enabled",
            Bool,
            json!(true),
            "Crear libros diarios",
            "Libros diarios",
        )),
        vault(base(
            "daily.create",
            Enum,
            json!("first_open"),
            "Cuándo crear el libro del día",
            "Libros diarios",
        ))
        .pipe(|f| choices(f, &["first_open", "first_note", "manual"])),
        vault(base(
            "daily.title_format",
            String,
            json!("%d/%m/%Y"),
            "Formato del título diario",
            "Libros diarios",
        )),
        vault(base(
            "daily.folder_format",
            String,
            json!("%Y-%m-%d"),
            "Formato de la carpeta",
            "Libros diarios",
        )),
        vault(base(
            "daily.timezone",
            String,
            json!("local"),
            "Zona horaria IANA o local",
            "Libros diarios",
        )),
        vault(ranged(
            base(
                "daily.day_start_hour",
                Int,
                json!(4),
                "Hora de corte del día",
                "Libros diarios",
            ),
            0.0,
            23.0,
        )),
        vault(base(
            "daily.backfill",
            Enum,
            json!("manual"),
            "Relleno de días faltantes",
            "Libros diarios",
        ))
        .pipe(|f| choices(f, &["manual", "ask", "auto", "never"])),
        vault(base(
            "daily.recreate_deleted",
            Bool,
            json!(false),
            "Recrear el libro del día borrado",
            "Libros diarios",
        )),
        vault(base(
            "daily.template",
            Path,
            json!(""),
            "Nota plantilla opcional",
            "Libros diarios",
        )),
        vault(ranged(
            base(
                "daily.auto_archive_days",
                Int,
                json!(0),
                "Archivar tras N días (0 = nunca)",
                "Libros diarios",
            ),
            0.0,
            3650.0,
        )),
        base(
            "daily.start_of_week",
            Enum,
            json!("monday"),
            "Inicio de semana",
            "Libros diarios",
        )
        .pipe(|f| choices(f, &["monday", "sunday"])),
        base(
            "daily.locale",
            String,
            json!("es-ES"),
            "Locale de fechas",
            "Libros diarios",
        ),
        // Notas
        vault(base(
            "notes.default_sort",
            Enum,
            json!("manual"),
            "Orden por defecto",
            "Notas",
        ))
        .pipe(|f| choices(f, &["manual", "title", "created", "modified"])),
        vault(base(
            "notes.sort_direction",
            Enum,
            json!("asc"),
            "Dirección del orden",
            "Notas",
        ))
        .pipe(|f| choices(f, &["asc", "desc"])),
        base(
            "notes.confirm_delete",
            Bool,
            json!(true),
            "Confirmar borrado",
            "Notas",
        ),
        vault(ranged(
            base(
                "notes.trash_days",
                Int,
                json!(30),
                "Retención de papelera (días)",
                "Notas",
            ),
            0.0,
            365.0,
        )),
        vault(ranged(
            base(
                "notes.version_days",
                Int,
                json!(30),
                "Retención de versiones (días)",
                "Notas",
            ),
            0.0,
            3650.0,
        )),
        vault(ranged(
            base(
                "notes.version_max",
                Int,
                json!(50),
                "Máximo de versiones por nota",
                "Notas",
            ),
            0.0,
            500.0,
        )),
        ranged(
            base(
                "notes.autosave_ms",
                Int,
                json!(800),
                "Debounce de autosave",
                "Notas",
            ),
            200.0,
            5000.0,
        ),
        ranged(
            base(
                "notes.recovery_ms",
                Int,
                json!(2000),
                "Intervalo de snapshot de recuperación",
                "Notas",
            ),
            500.0,
            10000.0,
        ),
        base(
            "notes.title_from_first_line",
            Bool,
            json!(true),
            "Derivar título de la primera línea",
            "Notas",
        ),
        vault(ranged(
            base(
                "notes.max_note_mb",
                Int,
                json!(50),
                "Aviso de nota grande (MB)",
                "Notas",
            ),
            1.0,
            200.0,
        )),
        vault(base(
            "notes.default_template",
            String,
            json!(""),
            "Plantilla de nota nueva",
            "Notas",
        )),
        // Orden y arrastre
        ranged(
            base(
                "drag.spring_delay_ms",
                Int,
                json!(650),
                "Retardo de spring-load",
                "Arrastrar",
            ),
            200.0,
            2000.0,
        ),
        base(
            "drag.autoscroll",
            Bool,
            json!(true),
            "Auto-scroll al arrastrar",
            "Arrastrar",
        ),
        base(
            "drag.position_picker",
            Bool,
            json!(true),
            "Mostrar selector de posición",
            "Arrastrar",
        ),
        base(
            "drag.haptic_feedback",
            Bool,
            json!(false),
            "Vibración si el hardware la soporta",
            "Arrastrar",
        ),
        base(
            "drag.show_drop_preview",
            Bool,
            json!(true),
            "Vista previa al soltar",
            "Arrastrar",
        ),
        // Editor y lienzo
        base(
            "editor.spellcheck",
            Bool,
            json!(false),
            "Corrector ortográfico",
            "Editor",
        ),
        base(
            "editor.spellcheck_lang",
            String,
            json!("es"),
            "Diccionario del corrector",
            "Editor",
        ),
        base(
            "editor.markdown_shortcuts",
            Bool,
            json!(true),
            "Atajos Markdown",
            "Editor",
        ),
        base(
            "editor.slash_menu",
            Bool,
            json!(true),
            "Menú / de inserción",
            "Editor",
        ),
        base(
            "editor.paste_web_as_card",
            Bool,
            json!(true),
            "Convertir URL pegada en tarjeta",
            "Editor",
        ),
        base(
            "editor.typewriter_scroll",
            Bool,
            json!(false),
            "Mantener el cursor centrado",
            "Editor",
        ),
        base("canvas.snap", Bool, json!(true), "Ajuste a guías", "Lienzo"),
        base(
            "canvas.grid",
            Bool,
            json!(false),
            "Rejilla visible",
            "Lienzo",
        ),
        ranged(
            base(
                "canvas.grid_size",
                Int,
                json!(8),
                "Tamaño de rejilla",
                "Lienzo",
            ),
            4.0,
            64.0,
        ),
        ranged(
            base(
                "canvas.zoom_min",
                Float,
                json!(0.25),
                "Zoom mínimo",
                "Lienzo",
            ),
            0.1,
            1.0,
        ),
        ranged(
            base(
                "canvas.zoom_max",
                Float,
                json!(4.0),
                "Zoom máximo",
                "Lienzo",
            ),
            1.0,
            8.0,
        ),
        base(
            "canvas.show_anchored_in_document",
            Bool,
            json!(true),
            "Ver anclados en encuadre documento",
            "Lienzo",
        ),
        vault(base(
            "canvas.default_mode",
            Enum,
            json!("document"),
            "Vista inicial del lienzo",
            "Lienzo",
        ))
        .pipe(|f| choices(f, &["document", "canvas"])),
        vault(ranged(
            base(
                "canvas.max_blocks",
                Int,
                json!(5000),
                "Aviso de nota enorme",
                "Lienzo",
            ),
            100.0,
            20000.0,
        )),
        // Medios
        vault(base(
            "media.copy_into_vault",
            Bool,
            json!(true),
            "Copiar medios a la bóveda",
            "Medios",
        )),
        vault(ranged(
            base(
                "media.max_asset_mb",
                Int,
                json!(2048),
                "Tamaño máximo de adjunto (MB)",
                "Medios",
            ),
            1.0,
            20480.0,
        )),
        ranged(
            base(
                "media.image_max_px",
                Int,
                json!(4096),
                "Píxeles máximos de imagen",
                "Medios",
            ),
            512.0,
            16384.0,
        ),
        ranged(
            base(
                "media.thumbnail_cache_mb",
                Int,
                json!(256),
                "Caché de miniaturas (MB)",
                "Medios",
            ),
            32.0,
            2048.0,
        ),
        base(
            "media.autoplay",
            Bool,
            json!(false),
            "Reproducir al abrir",
            "Medios",
        ),
        base(
            "media.pause_offscreen",
            Bool,
            json!(true),
            "Pausar fuera de pantalla",
            "Medios",
        ),
        base(
            "media.video_poster",
            Bool,
            json!(true),
            "Generar póster de vídeo",
            "Medios",
        ),
        base(
            "media.waveform",
            Bool,
            json!(true),
            "Generar onda de audio",
            "Medios",
        ),
        // Web
        vault(base(
            "web.enabled",
            Bool,
            json!(true),
            "Permitir bloques web",
            "Web",
        )),
        vault(base(
            "web.default_mode",
            Enum,
            json!("card"),
            "Modo por defecto",
            "Web",
        ))
        .pipe(|f| choices(f, &["card", "live"])),
        vault(base(
            "web.live_enabled",
            Bool,
            json!(false),
            "Permitir vista web viva (1.0)",
            "Web",
        )),
        vault(base(
            "web.fetch_metadata",
            Enum,
            json!("ask"),
            "Obtener metadatos de enlaces",
            "Web",
        ))
        .pipe(|f| choices(f, &["never", "ask", "always"])),
        vault(base(
            "web.persist_cookies",
            Bool,
            json!(false),
            "Cookies persistentes",
            "Web",
        )),
        vault(base(
            "web.allow_downloads",
            Bool,
            json!(false),
            "Permitir descargas",
            "Web",
        )),
        ranged(
            base(
                "web.timeout_s",
                Int,
                json!(15),
                "Timeout de metadatos (s)",
                "Web",
            ),
            3.0,
            60.0,
        ),
        ranged(
            base(
                "web.max_metadata_kb",
                Int,
                json!(512),
                "Tamaño máximo de respuesta (KB)",
                "Web",
            ),
            32.0,
            2048.0,
        ),
        // Plugins
        vault(base(
            "plugins.enabled",
            Bool,
            json!(true),
            "Activar plugins",
            "Plugins",
        )),
        base(
            "plugins.dev_mode",
            Bool,
            json!(false),
            "Recarga en vivo y logs",
            "Plugins",
        ),
        vault(base(
            "plugins.allow_unsigned",
            Bool,
            json!(false),
            "Permitir plugins sin firma",
            "Plugins",
        )),
        vault(ranged(
            base(
                "plugins.max_memory_mb",
                Int,
                json!(64),
                "Memoria por plugin (MB)",
                "Plugins",
            ),
            8.0,
            512.0,
        )),
        vault(base(
            "plugins.max_fuel",
            Int,
            json!(10_000_000),
            "Instrucciones por llamada",
            "Plugins",
        )),
        vault(ranged(
            base(
                "plugins.timeout_ms",
                Int,
                json!(2000),
                "Timeout de comando (ms)",
                "Plugins",
            ),
            10.0,
            5000.0,
        )),
        vault(base(
            "plugins.network_default",
            Bool,
            json!(false),
            "Red por defecto",
            "Plugins",
        )),
        vault(base(
            "plugins.auto_update",
            Bool,
            json!(false),
            "Actualizar desde archivo (1.0)",
            "Plugins",
        )),
        // Bóveda
        vault(base(
            "vault.name",
            String,
            json!(""),
            "Nombre visible de la bóveda",
            "Bóveda",
        )),
        vault(base(
            "vault.encryption",
            Enum,
            json!("none"),
            "Cifrado (1.0)",
            "Bóveda",
        ))
        .pipe(|f| choices(f, &["none", "password"])),
        vault(ranged(
            base(
                "vault.auto_lock_min",
                Int,
                json!(15),
                "Bloqueo por inactividad (min)",
                "Bóveda",
            ),
            1.0,
            120.0,
        )),
        vault(base(
            "vault.lock_on_suspend",
            Bool,
            json!(true),
            "Bloquear al suspender",
            "Bóveda",
        )),
        vault(base(
            "vault.backup_enabled",
            Bool,
            json!(true),
            "Backups automáticos",
            "Bóveda",
        )),
        vault(base(
            "vault.backup_daily",
            Bool,
            json!(true),
            "Backup diario al abrir",
            "Bóveda",
        )),
        vault(ranged(
            base(
                "vault.backup_keep_daily",
                Int,
                json!(7),
                "Retención diaria",
                "Bóveda",
            ),
            0.0,
            60.0,
        )),
        vault(ranged(
            base(
                "vault.backup_keep_weekly",
                Int,
                json!(4),
                "Retención semanal",
                "Bóveda",
            ),
            0.0,
            52.0,
        )),
        vault(ranged(
            base(
                "vault.backup_keep_monthly",
                Int,
                json!(12),
                "Retención mensual",
                "Bóveda",
            ),
            0.0,
            120.0,
        )),
        vault(base(
            "vault.read_only_on_lock",
            Bool,
            json!(true),
            "Solo lectura si otra instancia",
            "Bóveda",
        )),
        // Atajos
        both(base(
            "keybindings.new_note",
            String,
            json!("<Control>n"),
            "Nueva nota",
            "Atajos",
        )),
        both(base(
            "keybindings.new_book",
            String,
            json!("<Control><Shift>n"),
            "Nuevo libro",
            "Atajos",
        )),
        both(base(
            "keybindings.palette",
            String,
            json!("<Control>k"),
            "Paleta de comandos",
            "Atajos",
        )),
        both(base(
            "keybindings.search",
            String,
            json!("<Control>p"),
            "Buscar",
            "Atajos",
        )),
        both(base(
            "keybindings.today",
            String,
            json!("<Control><Shift>h"),
            "Ir a hoy",
            "Atajos",
        )),
        both(base(
            "keybindings.preview",
            String,
            json!("<Control><Shift>e"),
            "Previsualizar",
            "Atajos",
        )),
        both(base(
            "keybindings.move_note",
            String,
            json!("<Control><Shift>m"),
            "Mover nota",
            "Atajos",
        )),
        both(base(
            "keybindings.fit_document",
            String,
            json!("<Control>1"),
            "Encuadre documento",
            "Atajos",
        )),
        both(base(
            "keybindings.fit_all",
            String,
            json!("<Control>2"),
            "Encuadre todo",
            "Atajos",
        )),
        // Avanzado
        advanced(base(
            "advanced.log_level",
            Enum,
            json!("info"),
            "Nivel de log",
            "Avanzado",
        ))
        .pipe(|f| choices(f, &["error", "warn", "info", "debug", "trace"])),
        advanced(ranged(
            base(
                "advanced.log_retention_days",
                Int,
                json!(14),
                "Retención de logs",
                "Avanzado",
            ),
            1.0,
            90.0,
        )),
        advanced(base(
            "advanced.log_content",
            Bool,
            json!(false),
            "Incluir texto de notas en logs",
            "Avanzado",
        )),
        advanced(ranged(
            base(
                "advanced.index_batch",
                Int,
                json!(100),
                "Lote de indexación",
                "Avanzado",
            ),
            10.0,
            1000.0,
        )),
        advanced(ranged(
            base(
                "advanced.watch_debounce_ms",
                Int,
                json!(300),
                "Debounce del watcher",
                "Avanzado",
            ),
            50.0,
            5000.0,
        )),
        advanced(base(
            "advanced.telemetry",
            Bool,
            json!(false),
            "Telemetría (siempre off en MVP)",
            "Avanzado",
        )),
        advanced(base(
            "advanced.experimental",
            Bool,
            json!(false),
            "Funciones inestables",
            "Avanzado",
        )),
    ]
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigError {
    pub file: String,
    pub line: Option<usize>,
    pub key: String,
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(
                formatter,
                "{}: {}:{}: {}",
                self.message, self.file, line, self.key
            ),
            None => write!(formatter, "{}: {}: {}", self.message, self.file, self.key),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone)]
pub struct Config {
    values: BTreeMap<String, Value>,
    sources: BTreeMap<String, String>,
}

impl Config {
    pub fn defaults() -> Self {
        let mut values = BTreeMap::new();
        let mut sources = BTreeMap::new();
        for field in schema() {
            values.insert(field.path.clone(), field.default);
            sources.insert(field.path, "defecto".into());
        }
        Self { values, sources }
    }

    /// Precedencia: global < bóveda < entorno. Los flags se aplican después.
    pub fn load(
        global: Option<&Path>,
        vault: Option<&Path>,
        env: &HashMap<String, String>,
    ) -> Result<Self, ConfigError> {
        let mut config = Self::defaults();
        if let Some(path) = global {
            config.merge_file(path, "global")?;
        }
        if let Some(path) = vault {
            config.merge_file(path, "bóveda")?;
        }
        config.apply_env(env)?;
        config.validate()?;
        Ok(config)
    }

    pub fn merge_file(&mut self, path: &Path, source: &str) -> Result<(), ConfigError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(ConfigError {
                    file: path.display().to_string(),
                    line: None,
                    key: String::new(),
                    message: error.to_string(),
                });
            }
        };
        let table: toml::Table = text.parse().map_err(|error: toml::de::Error| {
            let line = error
                .span()
                .map(|span| text[..span.start].lines().count().max(1));
            ConfigError {
                file: path.display().to_string(),
                line,
                key: String::new(),
                message: error.to_string(),
            }
        })?;
        let mut flat = BTreeMap::new();
        flatten_toml("", &toml::Value::Table(table), &mut flat);
        for (key, value) in flat {
            let value = toml_to_json(value);
            if let Some(field) = schema().into_iter().find(|field| field.path == key) {
                validate_value(&field, &value).map_err(|message| ConfigError {
                    file: path.display().to_string(),
                    line: find_line(&text, &key),
                    key: key.clone(),
                    message,
                })?;
            }
            self.values.insert(key.clone(), value);
            self.sources.insert(key, source.to_string());
        }
        Ok(())
    }

    pub fn apply_env(&mut self, env: &HashMap<String, String>) -> Result<(), ConfigError> {
        for field in schema() {
            let name = format!("FOLIO_{}", field.path.to_uppercase().replace('.', "_"));
            if let Some(raw) = env.get(&name) {
                let value = parse_scalar(&field, raw).map_err(|message| ConfigError {
                    file: "entorno".into(),
                    line: None,
                    key: field.path.clone(),
                    message,
                })?;
                self.values.insert(field.path.clone(), value);
                self.sources.insert(field.path.clone(), "entorno".into());
            }
        }
        Ok(())
    }

    pub fn apply_override(&mut self, key: &str, raw: &str) -> Result<(), ConfigError> {
        let field = schema()
            .into_iter()
            .find(|field| field.path == key)
            .ok_or_else(|| ConfigError {
                file: "CLI".into(),
                line: None,
                key: key.into(),
                message: "clave desconocida".into(),
            })?;
        let value = parse_scalar(&field, raw).map_err(|message| ConfigError {
            file: "CLI".into(),
            line: None,
            key: key.into(),
            message,
        })?;
        self.values.insert(key.into(), value);
        self.sources.insert(key.into(), "CLI".into());
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        for field in schema() {
            if let Some(value) = self.values.get(&field.path) {
                validate_value(&field, value).map_err(|message| ConfigError {
                    file: "config".into(),
                    line: None,
                    key: field.path.clone(),
                    message,
                })?;
            }
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub fn origin(&self, key: &str) -> Option<&str> {
        self.sources.get(key).map(String::as_str)
    }

    pub fn as_map(&self) -> &BTreeMap<String, Value> {
        &self.values
    }

    pub fn string(&self, key: &str) -> String {
        self.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    pub fn bool(&self, key: &str) -> bool {
        self.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    pub fn int(&self, key: &str) -> i64 {
        self.get(key).and_then(Value::as_i64).unwrap_or(0)
    }

    pub fn float(&self, key: &str) -> f64 {
        self.get(key).and_then(Value::as_f64).unwrap_or(0.0)
    }

    pub fn set_memory(&mut self, key: &str, value: Value) {
        self.values.insert(key.into(), value);
        self.sources.insert(key.into(), "memoria".into());
    }

    pub fn schema_json(&self) -> Value {
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "Folio",
            "type": "object",
            "properties": schema().into_iter().map(|field| {
                let mut schema = json!({
                    "type": match field.ty {
                        FieldType::Bool => "boolean",
                        FieldType::Int => "integer",
                        FieldType::Float => "number",
                        _ => "string",
                    },
                    "title": field.label,
                    "description": field.help,
                    "default": field.default,
                    "x-folio-group": field.group,
                    "x-folio-scope": field.scope,
                });
                if let Some(min) = field.min {
                    schema["minimum"] = json!(min);
                }
                if let Some(max) = field.max {
                    schema["maximum"] = json!(max);
                }
                if !field.choices.is_empty() {
                    schema["enum"] = json!(field.choices);
                }
                (field.path, schema)
            }).collect::<serde_json::Map<String, Value>>()
        })
    }
}

fn flatten_toml(prefix: &str, value: &toml::Value, out: &mut BTreeMap<String, toml::Value>) {
    match value {
        toml::Value::Table(table) => {
            for (key, value) in table {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_toml(&path, value, out);
            }
        }
        other => {
            if !prefix.is_empty() {
                out.insert(prefix.to_string(), other.clone());
            }
        }
    }
}

fn toml_to_json(value: toml::Value) -> Value {
    match value {
        toml::Value::String(value) => json!(value),
        toml::Value::Integer(value) => json!(value),
        toml::Value::Float(value) => json!(value),
        toml::Value::Boolean(value) => json!(value),
        toml::Value::Datetime(value) => json!(value.to_string()),
        toml::Value::Array(values) => {
            json!(values.into_iter().map(toml_to_json).collect::<Vec<_>>())
        }
        toml::Value::Table(table) => json!(
            table
                .into_iter()
                .map(|(key, value)| (key, toml_to_json(value)))
                .collect::<serde_json::Map<_, _>>()
        ),
    }
}

fn json_to_toml(value: &Value) -> toml_edit::Value {
    match value {
        Value::Bool(value) => (*value).into(),
        Value::Number(number) => {
            if let Some(int) = number.as_i64() {
                int.into()
            } else {
                number.as_f64().unwrap_or(0.0).into()
            }
        }
        Value::String(value) => value.as_str().into(),
        other => other.to_string().as_str().into(),
    }
}

fn validate_value(field: &Field, value: &Value) -> Result<(), String> {
    match field.ty {
        FieldType::Bool => {
            if !value.is_boolean() {
                return Err(format!("{} debe ser booleano", field.path));
            }
        }
        FieldType::Int => {
            let Some(number) = value.as_i64() else {
                return Err(format!("{} debe ser entero", field.path));
            };
            if let Some(min) = field.min
                && (number as f64) < min
            {
                return Err(format!("{} debe ser >= {min}", field.path));
            }
            if let Some(max) = field.max
                && (number as f64) > max
            {
                return Err(format!("{} debe ser <= {max}", field.path));
            }
        }
        FieldType::Float => {
            let Some(number) = value.as_f64() else {
                return Err(format!("{} debe ser numérico", field.path));
            };
            if let Some(min) = field.min
                && number < min
            {
                return Err(format!("{} debe ser >= {min}", field.path));
            }
            if let Some(max) = field.max
                && number > max
            {
                return Err(format!("{} debe ser <= {max}", field.path));
            }
        }
        FieldType::Enum => {
            let Some(text) = value.as_str() else {
                return Err(format!("{} debe ser texto", field.path));
            };
            if !field.choices.iter().any(|choice| choice == text) {
                return Err(format!(
                    "{} debe ser uno de: {}",
                    field.path,
                    field.choices.join(", ")
                ));
            }
        }
        FieldType::String | FieldType::Path | FieldType::Color | FieldType::Duration => {
            if !value.is_string() {
                return Err(format!("{} debe ser texto", field.path));
            }
        }
    }
    Ok(())
}

fn parse_scalar(field: &Field, raw: &str) -> Result<Value, String> {
    let value = match field.ty {
        FieldType::Bool => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "sí" | "si" => json!(true),
            "false" | "0" | "no" => json!(false),
            _ => return Err(format!("valor booleano inválido: {raw}")),
        },
        FieldType::Int => json!(
            raw.parse::<i64>()
                .map_err(|_| format!("entero inválido: {raw}"))?
        ),
        FieldType::Float => json!(
            raw.parse::<f64>()
                .map_err(|_| format!("número inválido: {raw}"))?
        ),
        _ => json!(raw),
    };
    validate_value(field, &value)?;
    Ok(value)
}

fn find_line(text: &str, key: &str) -> Option<usize> {
    let leaf = key.rsplit('.').next().unwrap_or(key);
    text.lines()
        .position(|line| {
            line.trim_start().starts_with(&format!("{leaf} "))
                || line.trim_start().starts_with(&format!("{leaf}="))
        })
        .map(|index| index + 1)
}

/// Escribe una clave preservando comentarios y orden.
pub fn write_value(path: &Path, key: &str, value: &Value) -> Result<(), ConfigError> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    let mut document: DocumentMut =
        existing
            .parse()
            .map_err(|error: toml_edit::TomlError| ConfigError {
                file: path.display().to_string(),
                line: None,
                key: key.into(),
                message: error.to_string(),
            })?;
    let parts: Vec<&str> = key.split('.').collect();
    let mut table = document.as_table_mut();
    for part in &parts[..parts.len().saturating_sub(1)] {
        if !table.contains_key(part) {
            table.insert(part, Item::Table(Table::new()));
        }
        table = table
            .get_mut(part)
            .and_then(Item::as_table_mut)
            .ok_or_else(|| ConfigError {
                file: path.display().to_string(),
                line: None,
                key: key.into(),
                message: "la clave padre no es una tabla".into(),
            })?;
    }
    let leaf = parts.last().unwrap_or(&key);
    let mut new_value = toml_edit::value(json_to_toml(value));
    if let Some(Item::Value(old)) = table.get(leaf) {
        let prefix = old.decor().prefix().cloned();
        let suffix = old.decor().suffix().cloned();
        if let Some(item) = new_value.as_value_mut() {
            if let Some(prefix) = prefix {
                item.decor_mut().set_prefix(prefix);
            }
            if let Some(suffix) = suffix {
                item.decor_mut().set_suffix(suffix);
            }
        }
    }
    table.insert(leaf, new_value);
    write_atomic_config(path, document.to_string().as_bytes())
}

pub fn unset_value(path: &Path, key: &str) -> Result<bool, ConfigError> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    let mut document: DocumentMut =
        existing
            .parse()
            .map_err(|error: toml_edit::TomlError| ConfigError {
                file: path.display().to_string(),
                line: None,
                key: key.into(),
                message: error.to_string(),
            })?;
    let parts: Vec<&str> = key.split('.').collect();
    let mut table = document.as_table_mut();
    for part in &parts[..parts.len().saturating_sub(1)] {
        let Some(next) = table.get_mut(part).and_then(Item::as_table_mut) else {
            return Ok(false);
        };
        table = next;
    }
    Ok(table.remove(parts.last().unwrap_or(&key)).is_some())
}

fn write_atomic_config(path: &Path, bytes: &[u8]) -> Result<(), ConfigError> {
    use std::io::Write as _;
    let map = |error: std::io::Error| ConfigError {
        file: path.display().to_string(),
        line: None,
        key: String::new(),
        message: error.to_string(),
    };
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(map)?;
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    {
        let mut file = fs::File::create(&tmp).map_err(map)?;
        file.write_all(bytes).map_err(map)?;
        file.flush().map_err(map)?;
        file.sync_all().map_err(map)?;
    }
    fs::rename(&tmp, path).map_err(map)?;
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).map_err(map)?.permissions();
        permissions.set_mode(0o600);
        let _ = fs::set_permissions(path, permissions);
    }
    Ok(())
}

/// Crea el archivo global comentado si no existe.
pub fn ensure_global_file(path: &Path) -> Result<(), ConfigError> {
    if path.exists() {
        return Ok(());
    }
    let mut out = String::from(
        "# Configuración global de Folio.\n# Editable a mano o desde la GUI/CLI. Los valores por defecto están comentados.\n\n",
    );
    let mut current_group = String::new();
    for field in schema() {
        if field.scope == Scope::Vault {
            continue;
        }
        if field.group != current_group {
            if !current_group.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("# {}\n", field.group));
            current_group = field.group.clone();
        }
        out.push_str(&format!(
            "# {} ({}): {}\n# {} = {}\n",
            field.label,
            field.path,
            field.help,
            field.path.rsplit('.').next().unwrap_or(&field.path),
            toml_literal(&field.default)
        ));
    }
    write_atomic_config(path, out.as_bytes())
}

fn toml_literal(value: &Value) -> String {
    match value {
        Value::String(value) => format!("\"{value}\""),
        other => other.to_string(),
    }
}

pub fn global_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("FOLIO_CONFIG") {
        return PathBuf::from(path);
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".config")
        });
    base.join("folio").join("config.toml")
}

pub fn format_effective(config: &Config) -> String {
    let mut out = String::new();
    for field in schema() {
        if let Some(value) = config.get(&field.path) {
            out.push_str(&format!(
                "{:<34} = {:<18}  # {} ({})\n",
                field.path,
                toml_literal(value),
                field.label,
                config.origin(&field.path).unwrap_or("?")
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn every_field_has_metadata_and_valid_default() {
        let mut seen = std::collections::HashSet::new();
        for field in schema() {
            assert!(seen.insert(field.path.clone()), "duplicado {}", field.path);
            assert!(!field.label.is_empty(), "{} sin etiqueta", field.path);
            assert!(!field.help.is_empty(), "{} sin ayuda", field.path);
            assert!(!field.group.is_empty(), "{} sin grupo", field.path);
            validate_value(&field, &field.default).unwrap();
        }
        assert!(schema().len() > 80);
    }

    #[test]
    fn precedence_flags_env_vault_global_default() {
        let dir = tempfile::tempdir().unwrap();
        let global = dir.path().join("global.toml");
        let vault = dir.path().join("vault.toml");
        fs::write(&global, "[ui]\ntheme = \"dark\"\neditor_font_size = 18\n").unwrap();
        fs::write(&vault, "[ui]\ntheme = \"light\"\n").unwrap();
        let mut config = Config::load(
            Some(&global),
            Some(&vault),
            &env(&[("FOLIO_UI_THEME", "system")]),
        )
        .unwrap();
        assert_eq!(config.string("ui.theme"), "system");
        assert_eq!(config.int("ui.editor_font_size"), 18);
        config.apply_override("ui.theme", "dark").unwrap();
        assert_eq!(config.string("ui.theme"), "dark");
        assert_eq!(config.origin("ui.theme"), Some("CLI"));
    }

    #[test]
    fn invalid_toml_reports_line_and_does_not_apply() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.toml");
        fs::write(&path, "[daily]\nday_start_hour = 25\n").unwrap();
        let error = Config::load(None, Some(&path), &HashMap::new()).unwrap_err();
        assert_eq!(error.line, Some(2));
        assert!(error.to_string().contains("<= 23"));
        let bad_syntax = dir.path().join("bad.toml");
        fs::write(&bad_syntax, "[daily\nx = 1\n").unwrap();
        let error = Config::load(None, Some(&bad_syntax), &HashMap::new()).unwrap_err();
        assert!(error.line.is_some());
    }

    #[test]
    fn writing_preserves_comments_and_unknown_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            "# comentario importante\n[ui]\ntheme = \"dark\" # al lado\nfuturo = 42\n",
        )
        .unwrap();
        write_value(&path, "ui.theme", &json!("light")).unwrap();
        write_value(&path, "daily.title_format", &json!("%Y/%m/%d")).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("# comentario importante"));
        assert!(text.contains("# al lado"));
        assert!(text.contains("futuro = 42"));
        assert!(text.contains("theme = \"light\""));
        assert!(text.contains("[daily]"));
        let config = Config::load(None, Some(&path), &HashMap::new()).unwrap();
        assert_eq!(config.string("ui.theme"), "light");
        assert_eq!(config.string("daily.title_format"), "%Y/%m/%d");
    }

    #[test]
    fn gui_and_cli_write_the_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.toml");
        let b = dir.path().join("b.toml");
        write_value(&a, "daily.title_format", &json!("%Y/%m/%d")).unwrap();
        write_value(&b, "daily.title_format", &json!("%Y/%m/%d")).unwrap();
        assert_eq!(
            fs::read_to_string(a).unwrap(),
            fs::read_to_string(b).unwrap()
        );
    }

    #[test]
    fn schema_json_contains_all_fields() {
        let config = Config::defaults();
        let schema_json = config.schema_json();
        for field in schema() {
            assert!(schema_json["properties"].get(&field.path).is_some());
        }
    }

    #[test]
    fn commented_defaults_are_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        ensure_global_file(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("# Configuración global"));
        // Todo está comentado: el TOML es vacío y válido.
        let parsed: toml::Table = text.parse().unwrap();
        assert!(parsed.is_empty());
    }
}

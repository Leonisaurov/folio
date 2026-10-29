use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use folio_config::Config;
use folio_core::{Clock, FolioError, Note};
use folio_daily::{DailyConfig, logical_day};
use folio_render::{ExportOptions, import_markdown};
use folio_store::Store;

pub fn daily_config(config: &Config) -> DailyConfig {
    DailyConfig {
        enabled: config.bool("daily.enabled"),
        create: config.string("daily.create"),
        title_format: config.string("daily.title_format"),
        folder_format: config.string("daily.folder_format"),
        timezone: config.string("daily.timezone"),
        day_start_hour: config.int("daily.day_start_hour").clamp(0, 23) as u8,
        backfill: config.string("daily.backfill"),
        recreate_deleted: config.bool("daily.recreate_deleted"),
        template: config.string("daily.template"),
        auto_archive_days: config.int("daily.auto_archive_days").max(0) as u32,
    }
}

pub fn load_config(vault: Option<&Path>) -> Result<Config, String> {
    let global = folio_config::global_config_path();
    let vault_config = vault.map(|root| root.join("vault.toml"));
    let env: HashMap<String, String> = std::env::vars().collect();
    Config::load(Some(&global), vault_config.as_deref(), &env).map_err(|error| error.to_string())
}

pub fn ensure_today(
    store: &Store,
    config: &Config,
    clock: &dyn Clock,
    force_create: bool,
) -> Result<folio_store::Book, FolioError> {
    let daily = daily_config(config);
    let day = logical_day(clock.now(), &daily.timezone, daily.day_start_hour)?;
    if let Some(book) = store.daily_book(&day.date_string())? {
        return Ok(book);
    }
    if !daily.enabled {
        return Err(FolioError::NotFound(
            "los libros diarios están desactivados".into(),
        ));
    }
    let should_create =
        force_create || folio_daily::should_create_on_open(&daily) || daily.create == "first_note";
    if !should_create {
        return Err(FolioError::NotFound(format!(
            "no hay libro para {} y la política es '{}'",
            day.date_string(),
            daily.create
        )));
    }
    let title = day.title(&daily);
    let folder = day.folder(&daily);
    let timezone = if daily.timezone == "local" {
        "local".to_string()
    } else {
        daily.timezone.clone()
    };
    store.create_book(
        &title,
        "daily",
        Some(&day.date_string()),
        Some(&timezone),
        &folder,
        clock,
    )
}

pub fn export_note(
    store: &Store,
    note_id: &str,
    format: &str,
    out: &Path,
) -> Result<PathBuf, FolioError> {
    let (_book, note) = store.find_note(note_id)?;
    let options = ExportOptions::default();
    let path = match (format, out.extension()) {
        ("md" | "markdown", None) => out.with_extension("md"),
        ("html", None) => out.with_extension("html"),
        _ => out.to_path_buf(),
    };
    match format {
        "md" | "markdown" => folio_render::write_markdown(&path, &note, &options)?,
        "html" => folio_render::write_html(&path, &note, &options)?,
        other => return Err(FolioError::Unsupported(format!("formato {other}"))),
    }
    Ok(path)
}

pub fn import_markdown_file(
    store: &Store,
    book: &folio_store::Book,
    path: &Path,
    clock: &dyn Clock,
) -> Result<Note, FolioError> {
    let text = fs::read_to_string(path)?;
    let title = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "Importada".into());
    let mut note = import_markdown(&text, &title, clock);
    store.save_note(book, &mut note, clock)?;
    Ok(note)
}

pub fn set_config_value(file: &Path, key: &str, raw: &str) -> Result<(), String> {
    use folio_config::FieldType;
    let field = folio_config::schema()
        .into_iter()
        .find(|field| field.path == key)
        .ok_or_else(|| format!("clave desconocida: {key}"))?;
    let value = match field.ty {
        FieldType::Bool => serde_json::json!(match raw {
            "true" | "1" | "yes" | "sí" => true,
            "false" | "0" | "no" => false,
            _ => return Err(format!("booleano inválido: {raw}")),
        }),
        FieldType::Int => serde_json::json!(
            raw.parse::<i64>()
                .map_err(|_| format!("entero inválido: {raw}"))?
        ),
        FieldType::Float => serde_json::json!(
            raw.parse::<f64>()
                .map_err(|_| format!("número inválido: {raw}"))?
        ),
        _ => serde_json::json!(raw),
    };
    // Validar sobre una copia antes de tocar el archivo real. Un valor fuera de
    // rango no debe dejar el TOML inválido aunque la UI lo haya permitido.
    let tmp = file.with_extension(format!("validate-{}", std::process::id()));
    if file.exists() {
        fs::copy(file, &tmp).map_err(|error| error.to_string())?;
    }
    folio_config::write_value(&tmp, key, &value).map_err(|error| error.to_string())?;
    let env: HashMap<String, String> = HashMap::new();
    let result = Config::load(Some(&tmp), None, &env).map_err(|error| error.to_string());
    let _ = fs::remove_file(&tmp);
    result?;
    folio_config::write_value(file, key, &value).map_err(|error| error.to_string())
}

pub fn remember_vault(path: &Path) -> Result<(), FolioError> {
    let dir = std::env::var("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
                .join(".local/state")
        })
        .join("folio");
    std::fs::create_dir_all(&dir)?;
    folio_store::atomic::write_atomic(&dir.join("last_vault"), path.to_string_lossy().as_bytes())
}

pub fn today_date(config: &Config, clock: &dyn Clock) -> Result<String, FolioError> {
    let daily = daily_config(config);
    Ok(logical_day(clock.now(), &daily.timezone, daily.day_start_hour)?.date_string())
}

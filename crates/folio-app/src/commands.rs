use std::collections::HashMap;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use folio_config::{Config, FieldType, schema};
use folio_core::{Clock, FolioError, Note, text::text_block};
use folio_daily::{DailyConfig, backfill_dates, logical_day};
use folio_render::{ExportOptions, import_markdown};
use folio_store::{Index, MovePosition, Store, backup, doctor, trash, versions};

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

pub fn load_config(vault: Option<&Path>, config_path: Option<&Path>) -> Result<Config, String> {
    let global = config_path
        .map(PathBuf::from)
        .unwrap_or_else(folio_config::global_config_path);
    let vault_config = vault.map(|root| root.join("vault.toml"));
    let env: HashMap<String, String> = std::env::vars().collect();
    Config::load(Some(&global), vault_config.as_deref(), &env).map_err(|error| error.to_string())
}

pub fn resolve_vault(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return Some(path.to_path_buf());
    }
    if let Some(path) = std::env::var_os("FOLIO_VAULT") {
        return Some(PathBuf::from(path));
    }
    recent_vault_path().filter(|path| path.join("vault.toml").exists())
}

pub fn state_dir() -> PathBuf {
    std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".local/state")
        })
        .join("folio")
}

pub fn recent_vault_path() -> Option<PathBuf> {
    let path = state_dir().join("last_vault");
    let text = fs::read_to_string(path).ok()?;
    let path = PathBuf::from(text.trim());
    (!path.as_os_str().is_empty()).then_some(path)
}

pub fn remember_vault(path: &Path) -> Result<(), FolioError> {
    let dir = state_dir();
    fs::create_dir_all(&dir)?;
    folio_store::atomic::write_atomic(&dir.join("last_vault"), path.to_string_lossy().as_bytes())
}

/// Devuelve el libro del día, creándolo según la política configurada.
pub fn ensure_today(
    store: &Store,
    config: &Config,
    clock: &dyn Clock,
    force_create: bool,
) -> Result<folio_store::Book, FolioError> {
    let daily = daily_config(config);
    let now = clock.now();
    let day = logical_day(now, &daily.timezone, daily.day_start_hour)?;
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
        jiff::tz::TimeZone::system()
            .iana_name()
            .unwrap_or("local")
            .to_string()
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

pub fn resolve_book(
    store: &Store,
    config: &Config,
    clock: &dyn Clock,
    selector: &str,
) -> Result<folio_store::Book, FolioError> {
    if selector == "today" {
        ensure_today(store, config, clock, true)
    } else {
        store.book(selector).or_else(|_| {
            store
                .list_books()?
                .into_iter()
                .find(|book| book.meta.title == selector)
                .ok_or_else(|| FolioError::NotFound(format!("libro {selector}")))
        })
    }
}

pub fn backfill(
    store: &Store,
    config: &Config,
    clock: &dyn Clock,
    start: jiff::civil::Date,
    end: jiff::civil::Date,
) -> Result<Vec<folio_store::Book>, FolioError> {
    let daily = daily_config(config);
    daily.validate()?;
    let mut created = Vec::new();
    for date in backfill_dates(start, end)? {
        if store.daily_book(&date.to_string())?.is_some() {
            continue;
        }
        let day = logical_day(
            date.at(12, 0, 0, 0)
                .to_zoned(daily.time_zone()?)
                .map_err(|error| FolioError::Invalid(error.to_string()))?
                .timestamp(),
            &daily.timezone,
            daily.day_start_hour,
        )?;
        let title = day.title(&daily);
        let folder = day.folder(&daily);
        let book = store.create_book(
            &title,
            "daily",
            Some(&day.date_string()),
            Some(daily.timezone.as_str()),
            &folder,
            clock,
        )?;
        created.push(book);
    }
    Ok(created)
}

pub struct ExportRequest<'a> {
    pub store: &'a Store,
    pub note_id: &'a str,
    pub format: &'a str,
    pub out: &'a Path,
}

pub fn export_note(request: ExportRequest<'_>) -> Result<PathBuf, FolioError> {
    let (_book, note) = request.store.find_note(request.note_id)?;
    let options = ExportOptions::default();
    match request.format {
        "md" | "markdown" => {
            let path = if request.out.extension().is_none() {
                request.out.with_extension("md")
            } else {
                request.out.to_path_buf()
            };
            folio_render::write_markdown(&path, &note, &options)?;
            Ok(path)
        }
        "html" => {
            let path = if request.out.extension().is_none() {
                request.out.with_extension("html")
            } else {
                request.out.to_path_buf()
            };
            folio_render::write_html(&path, &note, &options)?;
            Ok(path)
        }
        "pdf" => {
            let path = if request.out.extension().is_none() {
                request.out.with_extension("pdf")
            } else {
                request.out.to_path_buf()
            };
            let resolver = |hash: &str| request.store.asset_display_path(hash).ok().flatten();
            folio_render::pdf::write_pdf_with_assets(
                &path,
                &note,
                &folio_render::pdf::PdfOptions::default(),
                &resolver,
            )?;
            Ok(path)
        }
        other => Err(FolioError::Unsupported(format!("formato {other}"))),
    }
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

pub enum ConfigScope {
    Global,
    Vault,
}

pub fn config_file(
    scope: ConfigScope,
    vault: Option<&Path>,
    explicit: Option<&Path>,
) -> Result<PathBuf, FolioError> {
    match scope {
        ConfigScope::Global => Ok(explicit
            .map(PathBuf::from)
            .unwrap_or_else(folio_config::global_config_path)),
        ConfigScope::Vault => {
            let vault = vault.ok_or_else(|| FolioError::NotFound("bóveda".into()))?;
            Ok(vault.join("vault.toml"))
        }
    }
}

pub fn set_config_value(file: &Path, key: &str, raw: &str) -> Result<(), String> {
    let field = schema()
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
    // Validar con una carga real del archivo modificado.
    let tmp = file.with_extension(format!("validate-{}", std::process::id()));
    if file.exists() {
        fs::copy(file, &tmp).map_err(|error| error.to_string())?;
    }
    folio_config::write_value(&tmp, key, &value).map_err(|error| error.to_string())?;
    let env = HashMap::new();
    Config::load(Some(&tmp), None, &env).map_err(|error| error.to_string())?;
    let _ = fs::remove_file(&tmp);
    folio_config::write_value(file, key, &value).map_err(|error| error.to_string())?;
    Ok(())
}

pub fn open_index(store: &Store) -> Result<Index, FolioError> {
    if store.is_encrypted() {
        let mut index = Index::open_in_memory()?;
        index.rebuild(store)?;
        Ok(index)
    } else {
        Index::open(&store.vault.index_path()).map_err(FolioError::from)
    }
}

pub fn rebuild_index(store: &Store) -> Result<usize, FolioError> {
    let mut index = if store.is_encrypted() {
        Index::open_in_memory()?
    } else {
        Index::open(&store.vault.index_path())?
    };
    index.rebuild(store).map_err(FolioError::from)
}

pub fn print_doctor(report: &doctor::DoctorReport, json: bool) -> Result<(), FolioError> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("Libros: {}", report.books);
        println!("Notas: {}", report.notes);
        println!("Corruptas: {}", report.corrupt.len());
        for item in &report.corrupt {
            println!("  - {item}");
        }
        println!("En cuarentena: {}", report.quarantined.len());
        println!("Assets huérfanos: {}", report.orphan_assets.len());
        println!("Assets ausentes: {}", report.missing_assets.len());
        println!("Reindexadas: {}", report.reindexed);
        println!("Operaciones pendientes: {}", report.pending_ops);
    }
    Ok(())
}

pub fn list_trash(store: &Store) -> Result<Vec<trash::TrashEntry>, FolioError> {
    trash::list(&store.vault, store.crypto())
}

pub fn list_backups(store: &Store) -> Result<Vec<backup::BackupEntry>, FolioError> {
    backup::list(&store.vault)
}

pub fn list_versions(
    store: &Store,
    note_id: &str,
) -> Result<Vec<versions::VersionEntry>, FolioError> {
    versions::list(&store.vault, note_id)
}

pub fn apply_markdown_block(note: &mut Note, text: &str) {
    note.blocks.push(text_block(text));
}

pub fn move_note(
    store: &Store,
    note_id: &str,
    from: &folio_store::Book,
    to: &folio_store::Book,
    before: Option<&str>,
) -> Result<PathBuf, FolioError> {
    let position = match before {
        Some(id) => MovePosition::Before(id),
        None => MovePosition::End,
    };
    store.move_note(note_id, from, to, position)
}

pub fn today_date(config: &Config, clock: &dyn Clock) -> Result<String, FolioError> {
    let daily = daily_config(config);
    Ok(logical_day(clock.now(), &daily.timezone, daily.day_start_hour)?.date_string())
}

/// Abre una bóveda, pidiendo contraseña o frase de recuperación si hace falta.
pub fn open_store(
    path: &Path,
    password: Option<&str>,
    recovery: Option<&str>,
) -> Result<Store, FolioError> {
    let mode = Store::encryption_mode(path)?;
    if mode != "password" {
        return Store::open(path);
    }
    if let Some(phrase) = recovery {
        return Store::open_with_recovery(path, phrase);
    }
    let password = match password {
        Some(password) => password.to_string(),
        None => match std::env::var("FOLIO_PASSWORD") {
            Ok(password) if !password.is_empty() => password,
            _ => prompt_secret("Contraseña de la bóveda: ")?,
        },
    };
    Store::open_with_password(path, &password)
}

pub fn prompt_secret(label: &str) -> Result<String, FolioError> {
    if std::io::stdin().is_terminal() {
        rpassword::prompt_password(label)
            .map_err(|error| FolioError::Invalid(format!("no se pudo leer la contraseña: {error}")))
    } else {
        Err(FolioError::Locked(
            "bóveda cifrada: usa --password o FOLIO_PASSWORD".into(),
        ))
    }
}

pub fn prompt_new_password() -> Result<String, FolioError> {
    if let Ok(password) = std::env::var("FOLIO_PASSWORD")
        && !password.is_empty()
    {
        return Ok(password);
    }
    if !std::io::stdin().is_terminal() {
        return Err(FolioError::Invalid(
            "bóveda cifrada: usa --password o FOLIO_PASSWORD".into(),
        ));
    }
    let first = rpassword::prompt_password("Nueva contraseña: ")
        .map_err(|error| FolioError::Invalid(error.to_string()))?;
    let second = rpassword::prompt_password("Repite la contraseña: ")
        .map_err(|error| FolioError::Invalid(error.to_string()))?;
    if first != second {
        return Err(FolioError::Invalid("las contraseñas no coinciden".into()));
    }
    if first.len() < 8 {
        return Err(FolioError::Invalid(
            "la contraseña debe tener al menos 8 caracteres".into(),
        ));
    }
    Ok(first)
}

mod commands;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use clap::{Parser, Subcommand};
use folio_config::Config;
use folio_core::{Clock, FolioError, SystemClock};
use folio_plugins::runtime::PluginRuntime;
use folio_store::Store;

#[derive(Debug, Parser)]
#[command(
    name = "folio",
    version,
    about = "Folio: notas y libros diarios, local primero",
    long_about = "Folio es una aplicación nativa de notas con libros diarios, lienzo infinito y plugins.\nSin subcomando abre la interfaz gráfica."
)]
struct Cli {
    /// Ruta de la bóveda.
    #[arg(long, global = true, env = "FOLIO_VAULT")]
    vault: Option<PathBuf>,
    /// Archivo de configuración global alternativo.
    #[arg(long, global = true, env = "FOLIO_CONFIG")]
    config: Option<PathBuf>,
    /// Contraseña de la bóveda cifrada (mejor usa FOLIO_PASSWORD).
    #[arg(long, global = true, env = "FOLIO_PASSWORD", hide_env_values = true)]
    password: Option<String>,
    /// Frase de recuperación de 24 palabras.
    #[arg(long, global = true, hide_env_values = true)]
    recovery: Option<String>,
    /// Cierra la app tras inicializar la ventana (benchmarks y CI).
    #[arg(long, hide = true)]
    quit_after_init: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Bóveda: crear, inspeccionar.
    Vault {
        #[command(subcommand)]
        command: VaultCommand,
    },
    /// Libros: crear, listar, renombrar, archivar, borrar.
    Book {
        #[command(subcommand)]
        command: BookCommand,
    },
    /// Notas: crear, listar, mostrar, mover, borrar, versiones.
    Note {
        #[command(subcommand)]
        command: NoteCommand,
    },
    /// Diagnóstico y reparación.
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long = "rebuild-index")]
        rebuild_index: bool,
    },
    /// Índice SQLite/FTS.
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },
    /// Búsqueda full-text.
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Exportar una nota a Markdown o HTML.
    Export {
        #[arg(long, default_value = "html")]
        format: String,
        #[arg(long)]
        note: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Importar Markdown como nota.
    Import {
        #[arg(long)]
        book: String,
        #[arg(long)]
        file: PathBuf,
    },
    /// Configuración: TOML, CLI y esquema.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Plugins: instalar, probar, habilitar.
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    /// Backups verificables.
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    /// Papelera.
    Trash {
        #[command(subcommand)]
        command: TrashCommand,
    },
    /// Genera los libros diarios que falten.
    Backfill {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
    },
}

#[derive(Debug, Subcommand)]
enum VaultCommand {
    Init {
        path: PathBuf,
        #[arg(long, default_value = "Folio")]
        name: String,
        /// Cifra la bóveda con contraseña (Argon2id + XChaCha20-Poly1305).
        #[arg(long)]
        encrypt: bool,
    },
    Info,
    /// Cifra una bóveda existente (haz backup antes).
    Encrypt,
    /// Descifra una bóveda existente.
    Decrypt,
}

#[derive(Debug, Subcommand)]
enum BookCommand {
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        daily: bool,
        #[arg(long)]
        date: Option<String>,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    Rename {
        id: String,
        title: String,
    },
    Archive {
        id: String,
        #[arg(long)]
        undo: bool,
    },
    Delete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
enum NoteCommand {
    Create {
        #[arg(long, default_value = "today")]
        book: String,
        #[arg(long, default_value = "")]
        title: String,
        #[arg(long)]
        text: Option<String>,
    },
    List {
        #[arg(long, default_value = "today")]
        book: String,
        #[arg(long)]
        json: bool,
    },
    Show {
        id: String,
    },
    Delete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
    Move {
        id: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        before: Option<String>,
    },
    Versions {
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum IndexCommand {
    Rebuild,
    Stats,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Path {
        #[arg(long)]
        vault: bool,
    },
    Get {
        key: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Set {
        key: String,
        value: String,
        #[arg(long)]
        global: bool,
    },
    Unset {
        key: String,
        #[arg(long)]
        global: bool,
    },
    Validate,
    Schema,
    Diff,
}

#[derive(Debug, Subcommand)]
enum PluginCommand {
    List {
        #[arg(long)]
        json: bool,
    },
    Install {
        path: PathBuf,
        #[arg(long)]
        allow_unsigned: bool,
        #[arg(long)]
        public_key: Option<String>,
    },
    Uninstall {
        id: String,
    },
    Enable {
        id: String,
    },
    Disable {
        id: String,
    },
    Test {
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum BackupCommand {
    Create,
    List,
    Verify { path: Option<PathBuf> },
    Restore { path: PathBuf, target: PathBuf },
}

#[derive(Debug, Subcommand)]
enum TrashCommand {
    List,
    Restore {
        path: PathBuf,
    },
    Purge {
        #[arg(long, default_value_t = 30)]
        days: u32,
    },
}

fn main() {
    let cli = Cli::parse();
    init_logging();
    install_panic_hook();
    let code = match run(cli) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("folio: {error}");
            1
        }
    };
    std::process::exit(code);
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(command) = cli.command {
        if let Command::Vault {
            command:
                VaultCommand::Init {
                    path,
                    name,
                    encrypt,
                },
        } = &command
        {
            let clock = SystemClock;
            if *encrypt {
                let password = cli
                    .password
                    .clone()
                    .or_else(|| std::env::var("FOLIO_PASSWORD").ok())
                    .map(Ok)
                    .unwrap_or_else(commands::prompt_new_password)?;
                let (store, phrase) = Store::init_encrypted(path, name, &clock, &password)?;
                commands::remember_vault(path)?;
                println!(
                    "Bóveda cifrada creada: {} ({})",
                    path.display(),
                    store.vault.meta.id
                );
                eprintln!("FRASE DE RECUPERACIÓN (guárdala):");
                eprintln!("{phrase}");
            } else {
                let store = Store::init(path, name, &clock)?;
                commands::remember_vault(path)?;
                println!(
                    "Bóveda creada: {} ({})",
                    path.display(),
                    store.vault.meta.id
                );
            }
            return Ok(());
        }
        let vault_path = commands::resolve_vault(cli.vault.as_deref());
        let store = match &vault_path {
            Some(path) => Some(commands::open_store(
                path,
                cli.password.as_deref(),
                cli.recovery.as_deref(),
            )?),
            None => None,
        };
        dispatch(
            command,
            store.as_ref(),
            cli.vault.as_deref(),
            cli.config.as_deref(),
            cli.password.as_deref(),
        )?;
        return Ok(());
    }

    let vault = commands::resolve_vault(cli.vault.as_deref());
    let config = commands::load_config(vault.as_deref(), cli.config.as_deref())
        .map_err(FolioError::Invalid)?;
    if let Some(path) = &vault {
        commands::remember_vault(path)?;
    }
    let exit = folio_ui::run(vault, config, cli.quit_after_init, cli.password.clone());
    std::process::exit(i32::from(exit.get()));
}

fn dispatch(
    command: Command,
    store: Option<&Store>,
    vault_path: Option<&Path>,
    config_path: Option<&Path>,
    password: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let clock = SystemClock;
    let require_store = || -> Result<&Store, FolioError> {
        store.ok_or_else(|| {
            FolioError::NotFound("no hay bóveda; usa --vault o 'folio vault init'".into())
        })
    };
    match command {
        Command::Vault {
            command:
                VaultCommand::Init {
                    path,
                    name,
                    encrypt,
                },
        } => {
            if encrypt {
                let password = password
                    .map(str::to_string)
                    .or_else(|| std::env::var("FOLIO_PASSWORD").ok())
                    .map(Ok)
                    .unwrap_or_else(commands::prompt_new_password)?;
                let (store, phrase) = Store::init_encrypted(&path, &name, &clock, &password)?;
                commands::remember_vault(&path)?;
                println!(
                    "Bóveda cifrada creada: {} ({})",
                    path.display(),
                    store.vault.meta.id
                );
                eprintln!("FRASE DE RECUPERACIÓN (guárdala):");
                eprintln!("{phrase}");
            } else {
                let store = Store::init(&path, &name, &clock)?;
                commands::remember_vault(&path)?;
                println!(
                    "Bóveda creada: {} ({})",
                    path.display(),
                    store.vault.meta.id
                );
            }
        }
        Command::Vault {
            command: VaultCommand::Encrypt,
        } => {
            let mut store = require_store()?.clone();
            if store.is_encrypted() {
                return Err("la bóveda ya está cifrada".into());
            }
            let password = password
                .map(str::to_string)
                .or_else(|| std::env::var("FOLIO_PASSWORD").ok())
                .map(Ok)
                .unwrap_or_else(commands::prompt_new_password)?;
            let phrase = store.encrypt_in_place(&password)?;
            commands::remember_vault(store.root())?;
            println!("Bóveda cifrada: {}", store.root().display());
            eprintln!("FRASE DE RECUPERACIÓN (guárdala):");
            eprintln!("{phrase}");
        }
        Command::Vault {
            command: VaultCommand::Decrypt,
        } => {
            let mut store = require_store()?.clone();
            let password = password
                .map(str::to_string)
                .or_else(|| std::env::var("FOLIO_PASSWORD").ok())
                .map(Ok)
                .unwrap_or_else(|| commands::prompt_secret("Contraseña de la bóveda: "))?;
            store.decrypt_in_place(&password)?;
            println!("Bóveda descifrada: {}", store.root().display());
        }
        Command::Vault {
            command: VaultCommand::Info,
        } => {
            let store = require_store()?;
            println!("Nombre: {}", store.vault.meta.name);
            println!("ID: {}", store.vault.meta.id);
            println!("Ruta: {}", store.root().display());
            println!("Libros: {}", store.list_books()?.len());
            println!("Notas: {}", store.note_count()?);
        }
        Command::Book { command } => {
            let store = require_store()?;
            let config = commands::load_config(Some(store.root()), config_path)?;
            match command {
                BookCommand::Create { title, daily, date } => {
                    let (kind, daily_date, folder) = if daily {
                        let date = date.unwrap_or_else(|| {
                            commands::today_date(&config, &clock).unwrap_or_default()
                        });
                        (Some("daily"), Some(date.clone()), date)
                    } else {
                        (None, None, title.clone())
                    };
                    let book = store.create_book(
                        &title,
                        kind.unwrap_or("manual"),
                        daily_date.as_deref(),
                        None,
                        &folder,
                        &clock,
                    )?;
                    println!("{}", book.meta.id);
                }
                BookCommand::List { json } => {
                    let books = store.list_books()?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(
                                &books
                                    .iter()
                                    .map(|book| serde_json::json!({
                                        "id": book.meta.id,
                                        "title": book.meta.title,
                                        "kind": book.meta.kind,
                                        "daily_date": book.meta.daily_date,
                                        "archived": book.meta.archived,
                                        "notes": store.notes(book).map(|n| n.len()).unwrap_or(0),
                                    }))
                                    .collect::<Vec<_>>()
                            )?
                        );
                    } else {
                        for book in books {
                            println!(
                                "{}\t{}\t{} nota(s)\t{}",
                                book.meta.id,
                                book.meta.title,
                                store.notes(&book)?.len(),
                                book.meta.kind
                            );
                        }
                    }
                }
                BookCommand::Rename { id, title } => {
                    let mut book = store.book(&id)?;
                    book.meta.title = title;
                    store.save_book(&mut book, &clock)?;
                    println!("{}", book.meta.id);
                }
                BookCommand::Archive { id, undo } => {
                    let mut book = store.book(&id)?;
                    book.meta.archived = !undo;
                    store.save_book(&mut book, &clock)?;
                    println!("archived={}", book.meta.archived);
                }
                BookCommand::Delete { id, yes } => {
                    let book = store.book(&id)?;
                    if !yes {
                        return Err("confirma con --yes".into());
                    }
                    let trashed = folio_store::trash::trash_book(
                        &store.vault,
                        &book,
                        &clock,
                        store.crypto(),
                    )?;
                    println!("{}", trashed.display());
                }
            }
        }
        Command::Note { command } => {
            let store = require_store()?;
            let config = commands::load_config(Some(store.root()), config_path)?;
            match command {
                NoteCommand::Create { book, title, text } => {
                    let book = commands::resolve_book(store, &config, &clock, &book)?;
                    let mut note = store.create_note(&book, &title, &clock)?;
                    if let Some(text) = text {
                        commands::apply_markdown_block(&mut note, &text);
                        store.save_note(&book, &mut note, &clock)?;
                    }
                    println!("{}", note.id);
                }
                NoteCommand::List { book, json } => {
                    let book = commands::resolve_book(store, &config, &clock, &book)?;
                    let notes = store.notes(&book)?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(
                                &notes
                                    .iter()
                                    .map(|note| serde_json::json!({
                                        "id": note.id,
                                        "title": note.title,
                                        "updated_at": note.updated_at,
                                        "words": note.word_count,
                                    }))
                                    .collect::<Vec<_>>()
                            )?
                        );
                    } else {
                        for note in notes {
                            println!("{}\t{}\t{}", note.id, note.title, note.updated_at);
                        }
                    }
                }
                NoteCommand::Show { id } => {
                    let (_book, note) = store.find_note(&id)?;
                    println!("{}", serde_json::to_string_pretty(&note)?);
                }
                NoteCommand::Delete { id, yes } => {
                    if !yes {
                        return Err("confirma con --yes".into());
                    }
                    let (book, _) = store.find_note(&id)?;
                    let path = store.delete_note(&book, &id, &clock)?;
                    println!("{}", path.display());
                }
                NoteCommand::Move { id, to, before } => {
                    let (from, _) = store.find_note(&id)?;
                    let to = commands::resolve_book(store, &config, &clock, &to)?;
                    let path = commands::move_note(store, &id, &from, &to, before.as_deref())?;
                    println!("{}", path.display());
                }
                NoteCommand::Versions { id } => {
                    for version in commands::list_versions(store, &id)? {
                        println!("{}\t{}", version.timestamp, version.path.display());
                    }
                }
            }
        }
        Command::Doctor {
            json,
            rebuild_index,
        } => {
            let store = require_store()?;
            let _lock = folio_store::VaultLock::acquire(&store.vault)?;
            let report = folio_store::doctor::repair(store, rebuild_index)?;
            commands::print_doctor(&report, json)?;
        }
        Command::Index { command } => {
            let store = require_store()?;
            match command {
                IndexCommand::Rebuild => {
                    let count = commands::rebuild_index(store)?;
                    println!("{count} notas indexadas");
                }
                IndexCommand::Stats => {
                    let index = commands::open_index(store)?;
                    let stats = index.stats()?;
                    println!("libros: {}", stats.books);
                    println!("notas: {}", stats.notes);
                    println!("bloques: {}", stats.blocks);
                    println!("fts: {}", stats.fts_rows);
                }
            }
        }
        Command::Search { query, limit, json } => {
            let store = require_store()?;
            let index = commands::open_index(store)?;
            let hits = index.search(&query, limit)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&hits)?);
            } else {
                for hit in hits {
                    println!(
                        "{}\t{}\t{}",
                        hit.note_id,
                        hit.title,
                        hit.snippet.replace(['<', '>'], "")
                    );
                }
            }
        }
        Command::Export { format, note, out } => {
            let store = require_store()?;
            let path = commands::export_note(commands::ExportRequest {
                store,
                note_id: &note,
                format: &format,
                out: &out,
            })?;
            println!("{}", path.display());
        }
        Command::Import { book, file } => {
            let store = require_store()?;
            let config = commands::load_config(Some(store.root()), config_path)?;
            let book = commands::resolve_book(store, &config, &clock, &book)?;
            let note = commands::import_markdown_file(store, &book, &file, &clock)?;
            println!("{}", note.id);
        }
        Command::Config { command } => {
            let global =
                commands::load_config(vault_path, config_path).map_err(FolioError::Invalid)?;
            match command {
                ConfigCommand::Path { vault } => {
                    let scope = if vault {
                        commands::ConfigScope::Vault
                    } else {
                        commands::ConfigScope::Global
                    };
                    println!(
                        "{}",
                        commands::config_file(scope, vault_path, config_path)?.display()
                    );
                }
                ConfigCommand::Get { key, json } => match key {
                    Some(key) => {
                        let value = global
                            .get(&key)
                            .ok_or_else(|| FolioError::NotFound(key.clone()))?;
                        if json {
                            println!("{}", serde_json::to_string_pretty(value)?);
                        } else {
                            println!("{}", display_value(value));
                        }
                    }
                    None => {
                        if json {
                            println!("{}", serde_json::to_string_pretty(global.as_map())?);
                        } else {
                            print!("{}", folio_config::format_effective(&global));
                        }
                    }
                },
                ConfigCommand::Set {
                    key,
                    value,
                    global: is_global,
                } => {
                    let scope = if is_global {
                        commands::ConfigScope::Global
                    } else {
                        commands::ConfigScope::Vault
                    };
                    let file = commands::config_file(scope, vault_path, config_path)?;
                    commands::set_config_value(&file, &key, &value)?;
                    println!("{} = {} ({})", key, value, file.display());
                }
                ConfigCommand::Unset {
                    key,
                    global: is_global,
                } => {
                    let scope = if is_global {
                        commands::ConfigScope::Global
                    } else {
                        commands::ConfigScope::Vault
                    };
                    let file = commands::config_file(scope, vault_path, config_path)?;
                    let removed = folio_config::unset_value(&file, &key)?;
                    println!("{}", if removed { "eliminada" } else { "no existía" });
                }
                ConfigCommand::Validate => {
                    commands::load_config(vault_path, config_path).map_err(FolioError::Invalid)?;
                    println!("configuración válida");
                }
                ConfigCommand::Schema => {
                    println!("{}", serde_json::to_string_pretty(&global.schema_json())?);
                }
                ConfigCommand::Diff => {
                    let defaults = Config::defaults();
                    for (key, value) in global.as_map() {
                        if defaults.get(key) != Some(value) {
                            println!(
                                "{key} = {}  ({})",
                                display_value(value),
                                global.origin(key).unwrap_or("?")
                            );
                        }
                    }
                }
            }
        }
        Command::Plugin { command } => {
            let store = require_store()?;
            let config = commands::load_config(Some(store.root()), config_path)?;
            let mut registry = folio_plugins::registry::Registry::load(store.root())?;
            match command {
                PluginCommand::List { json } => {
                    let manifests = registry.available_manifests(store.root());
                    if json {
                        println!("{}", serde_json::to_string_pretty(&manifests)?);
                    } else {
                        for manifest in manifests {
                            let installed = registry.installed(&manifest.id);
                            println!(
                                "{}\t{}\t{}\t{}",
                                manifest.id,
                                manifest.version,
                                if installed.map(|i| i.enabled).unwrap_or(false) {
                                    "activo"
                                } else if installed.is_some() {
                                    "desactivado"
                                } else {
                                    "de referencia"
                                },
                                manifest.name
                            );
                        }
                    }
                }
                PluginCommand::Install {
                    path,
                    allow_unsigned,
                    public_key,
                } => {
                    let installed = registry.install_from_dir(
                        store.root(),
                        &path,
                        allow_unsigned || config.bool("plugins.allow_unsigned"),
                        public_key.as_deref(),
                        &clock.now_rfc3339(),
                    )?;
                    println!("{} {}", installed.id, installed.version);
                }
                PluginCommand::Uninstall { id } => {
                    registry.uninstall(store.root(), &id)?;
                    println!("{id} desinstalado; el estado de sus bloques se conserva");
                }
                PluginCommand::Enable { id } => {
                    registry.set_enabled(&id, true)?;
                    registry.save(store.root())?;
                    println!("{id} activado");
                }
                PluginCommand::Disable { id } => {
                    registry.set_enabled(&id, false)?;
                    registry.save(store.root())?;
                    println!("{id} desactivado");
                }
                PluginCommand::Test { path } => {
                    let manifest = folio_plugins::manifest::load_file(&path.join("manifest.toml"))?;
                    let mut runtime = folio_plugins::runtime::BuiltinRuntime::default();
                    let context = registry.context(&manifest.id, 64, 2000);
                    let response = runtime.call(
                        &manifest,
                        &folio_plugin_sdk::Request {
                            command: "render".into(),
                            block_state: serde_json::json!({}),
                            payload: serde_json::Value::Null,
                        },
                        &context,
                    )?;
                    response.ui.validate()?;
                    println!("{} OK ({} nodos)", manifest.id, response.ui.count());
                }
            }
        }
        Command::Backup { command } => {
            let store = require_store()?;
            match command {
                BackupCommand::Create => {
                    let path = folio_store::backup::create(&store.vault, &clock)?;
                    println!("{}", path.display());
                }
                BackupCommand::List => {
                    for entry in commands::list_backups(store)? {
                        println!(
                            "{}\t{} archivos\t{}",
                            entry.dir.display(),
                            entry.manifest.files.len(),
                            entry.manifest.created_at
                        );
                    }
                }
                BackupCommand::Verify { path } => {
                    let path = match path {
                        Some(path) => path,
                        None => commands::list_backups(store)?
                            .last()
                            .map(|entry| entry.dir.clone())
                            .ok_or_else(|| FolioError::NotFound("backup".into()))?,
                    };
                    let manifest = folio_store::backup::verify(&path)?;
                    println!("{} archivos verificados", manifest.files.len());
                }
                BackupCommand::Restore { path, target } => {
                    let count = folio_store::backup::restore(&path, &target)?;
                    println!("{count} archivos restaurados en {}", target.display());
                }
            }
        }
        Command::Trash { command } => {
            let store = require_store()?;
            match command {
                TrashCommand::List => {
                    for entry in commands::list_trash(store)? {
                        println!(
                            "{}\t{}\t{}",
                            entry.dir.display(),
                            entry.meta.kind,
                            entry.meta.title
                        );
                    }
                }
                TrashCommand::Restore { path } => {
                    let restored =
                        folio_store::trash::restore(&store.vault, &path, store.crypto())?;
                    println!("{}", restored.display());
                }
                TrashCommand::Purge { days } => {
                    let count =
                        folio_store::trash::purge(&store.vault, days, &clock, store.crypto())?;
                    println!("{count} entradas purgadas");
                }
            }
        }
        Command::Backfill { from, to } => {
            let store = require_store()?;
            let config = commands::load_config(Some(store.root()), config_path)?;
            let from: jiff::civil::Date = from
                .parse()
                .map_err(|_| FolioError::Invalid("fecha from".into()))?;
            let to: jiff::civil::Date = to
                .parse()
                .map_err(|_| FolioError::Invalid("fecha to".into()))?;
            let created = commands::backfill(store, &config, &clock, from, to)?;
            for book in created {
                println!("{}\t{}", book.meta.id, book.meta.title);
            }
        }
    }
    Ok(())
}

fn display_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let dir = commands::state_dir().join("logs");
    let file = fs::create_dir_all(&dir).ok().and_then(|_| {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("folio.log"))
            .ok()
    });
    if let Some(file) = file {
        let writer = SharedWriter(Arc::new(Mutex::new(file)));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(writer)
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
    }
}

#[derive(Clone)]
struct SharedWriter(Arc<Mutex<fs::File>>);

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedWriter {
    type Writer = SharedWriterGuard<'a>;
    fn make_writer(&'a self) -> Self::Writer {
        SharedWriterGuard {
            file: self.0.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

struct SharedWriterGuard<'a> {
    file: Arc<Mutex<fs::File>>,
    // Conserva el lifetime para que el tipo no sea 'static por accidente.
    _marker: std::marker::PhantomData<&'a ()>,
}

impl Write for SharedWriterGuard<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| std::io::Error::other("log lock"))?;
        file.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| std::io::Error::other("log lock"))?;
        file.flush()
    }
}

fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let dir = commands::state_dir().join("crashes");
        let _ = fs::create_dir_all(&dir);
        let stamp = SystemClock.now().as_second();
        let path = dir.join(format!("crash-{stamp}.log"));
        if let Ok(mut file) = fs::File::create(path) {
            let _ = writeln!(file, "Folio {}", env!("CARGO_PKG_VERSION"));
            let _ = writeln!(file, "{info}");
            let backtrace = std::backtrace::Backtrace::force_capture();
            let _ = writeln!(file, "{backtrace}");
        }
        default(info);
    }));
}

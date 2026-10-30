use std::path::{Path, PathBuf};

use folio_config::Config;
use folio_core::{Block, FolioError, Note, SystemClock, UndoStack, model::TextContent};
use folio_daily::DailyConfig;
use folio_plugin_sdk::{Manifest, Request, Response};
use folio_plugins::registry::Registry;
use folio_plugins::runtime::{BuiltinRuntime, PluginRuntime, SandboxLimits};
use folio_plugins::{PluginError, builtin, manifest};
use folio_store::{Book, Index, MovePosition, Store};

use crate::helpers;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Document,
    Canvas,
    Preview,
}

pub struct AppState {
    pub store: Store,
    pub config: Config,
    pub index: Index,
    pub registry: Registry,
    pub current_book: Option<String>,
    pub current_note: Option<Note>,
    pub undo: UndoStack,
    pub dirty: bool,
    pub mode: ViewMode,
    pub canvas_scale: f64,
    pub search: String,
    pub config_error: Option<String>,
    pub last_message: Option<String>,
    pub locked: bool,
    pub clock: SystemClock,
}

impl AppState {
    pub fn new(store: Store, config: Config) -> Result<Self, FolioError> {
        if !store.is_unlocked() {
            return Err(FolioError::Locked(store.root().display().to_string()));
        }
        let mut index = if store.is_encrypted() {
            Index::open_in_memory()?
        } else {
            Index::open(&store.vault.index_path())?
        };
        if store.is_encrypted() || index.stats().map(|stats| stats.notes).unwrap_or(0) == 0 {
            index.rebuild(&store)?;
        }
        let registry = Registry::load(store.root())
            .map_err(|error| FolioError::Invalid(format!("plugins: {error}")))?;
        let daily = daily_config(&config);
        let mode = if config.string("canvas.default_mode") == "canvas" {
            ViewMode::Canvas
        } else {
            ViewMode::Document
        };
        let canvas_scale = 1.0;
        let mut state = Self {
            store,
            config,
            index,
            registry,
            current_book: None,
            current_note: None,
            undo: UndoStack::default(),
            dirty: false,
            mode,
            canvas_scale,
            search: String::new(),
            config_error: None,
            last_message: None,
            locked: false,
            clock: SystemClock,
        };
        if daily.enabled {
            // Abrir «Hoy» sin crear libros si la política es manual.
            if let Ok(book) =
                helpers::ensure_today(&state.store, &state.config, &state.clock, false)
            {
                state.current_book = Some(book.meta.id);
            }
        }
        if state.current_book.is_none()
            && let Some(first) = state.store.list_books()?.into_iter().next()
        {
            state.current_book = Some(first.meta.id);
        }
        Ok(state)
    }

    pub fn daily(&self) -> DailyConfig {
        daily_config(&self.config)
    }

    pub fn books(&self) -> Result<Vec<Book>, FolioError> {
        if self.locked {
            return Ok(Vec::new());
        }
        self.store.list_books()
    }

    pub fn current_book(&self) -> Option<Book> {
        let id = self.current_book.as_ref()?;
        self.store.book(id).ok()
    }

    pub fn notes(&self) -> Result<Vec<folio_store::NoteSummary>, FolioError> {
        if self.locked {
            return Ok(Vec::new());
        }
        if !self.search.is_empty() {
            let hits = self.index.search(&self.search, 100)?;
            let mut notes = Vec::new();
            for hit in hits {
                if let Ok((book, note)) = self.store.find_note(&hit.note_id)
                    && let Ok(summaries) = self.store.notes(&book)
                    && let Some(summary) = summaries.into_iter().find(|s| s.id == note.id)
                {
                    notes.push(summary);
                }
            }
            return Ok(notes);
        }
        match self.current_book() {
            Some(book) => self.store.notes(&book),
            None => Ok(Vec::new()),
        }
    }

    pub fn flush(&mut self) -> Result<(), FolioError> {
        if self.dirty && !self.locked {
            self.save()?;
        }
        Ok(())
    }

    /// Bloquea la bóveda: descarta la clave de memoria y los buffers visibles.
    pub fn lock(&mut self) {
        if !self.store.is_encrypted() {
            return;
        }
        self.flush().ok();
        self.store.lock();
        self.current_note = None;
        self.dirty = false;
        self.search.clear();
        self.undo.clear();
        self.locked = true;
    }

    /// Desbloquea con contraseña o frase de recuperación y reconstruye el índice
    /// efímero (nunca se escribe un índice en claro).
    pub fn unlock(&mut self, password: &str) -> Result<(), FolioError> {
        self.store.unlock(password)?;
        self.unlock_after_key()
    }

    pub fn unlock_recovery(&mut self, phrase: &str) -> Result<(), FolioError> {
        self.store.unlock_recovery(phrase)?;
        self.unlock_after_key()
    }

    fn unlock_after_key(&mut self) -> Result<(), FolioError> {
        self.locked = false;
        let mut index = if self.store.is_encrypted() {
            Index::open_in_memory()?
        } else {
            Index::open(&self.store.vault.index_path())?
        };
        index.rebuild(&self.store)?;
        self.index = index;
        Ok(())
    }

    pub fn open_book(&mut self, id: &str) -> Result<(), FolioError> {
        self.flush()?;
        let book = self.store.book(id)?;
        self.current_book = Some(book.meta.id);
        self.current_note = None;
        self.dirty = false;
        self.search.clear();
        self.undo.clear();
        Ok(())
    }

    pub fn open_note(&mut self, id: &str) -> Result<(), FolioError> {
        self.flush()?;
        let (book, mut note) = self.store.find_note(id)?;
        self.current_book = Some(book.meta.id);
        if let Ok(Some(recovery)) =
            folio_store::doctor::read_recovery(&self.store.vault, id, self.store.crypto())
            && recovery.updated_at > note.updated_at
        {
            note = recovery;
            self.last_message = Some("Recuperada una edición no guardada".into());
        }
        self.current_note = Some(note);
        self.undo.clear();
        self.dirty = false;
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), FolioError> {
        if self.locked {
            return Err(FolioError::Locked("la bóveda está bloqueada".into()));
        }
        let Some(book) = self.current_book() else {
            return Ok(());
        };
        let Some(note) = self.current_note.as_mut() else {
            return Ok(());
        };
        note.touch(&self.clock);
        let path = self.store.save_note(&book, note, &self.clock)?;
        if let Some((key, _)) = folio_store::repository::parse_note_filename(
            &path.file_name().unwrap_or_default().to_string_lossy(),
        ) {
            self.index.index_note(&book, note, &path, &key)?;
        }
        let _ = folio_store::doctor::clear_recovery(&self.store.vault, &note.id);
        self.dirty = false;
        Ok(())
    }

    pub fn create_note(&mut self) -> Result<Note, FolioError> {
        self.flush()?;
        let book = self
            .current_book()
            .ok_or_else(|| FolioError::NotFound("libro actual".into()))?;
        let note = self.store.create_note(&book, "", &self.clock)?;
        self.current_note = Some(note.clone());
        self.dirty = false;
        self.undo.clear();
        Ok(note)
    }

    pub fn create_book(&mut self, title: &str) -> Result<Book, FolioError> {
        self.flush()?;
        let book = self
            .store
            .create_book(title, "manual", None, None, title, &self.clock)?;
        self.current_book = Some(book.meta.id.clone());
        self.current_note = None;
        self.dirty = false;
        Ok(book)
    }

    pub fn today(&mut self) -> Result<Book, FolioError> {
        self.flush()?;
        let book = helpers::ensure_today(&self.store, &self.config, &self.clock, true)?;
        self.current_book = Some(book.meta.id.clone());
        self.dirty = false;
        Ok(book)
    }

    pub fn rename_current_book(&mut self, title: &str) -> Result<(), FolioError> {
        let mut book = self
            .current_book()
            .ok_or_else(|| FolioError::NotFound("libro".into()))?;
        book.meta.title = title.to_string();
        self.store.save_book(&mut book, &self.clock)
    }

    pub fn delete_current_note(&mut self) -> Result<PathBuf, FolioError> {
        let note = self
            .current_note
            .as_ref()
            .ok_or_else(|| FolioError::NotFound("nota".into()))?;
        let id = note.id.clone();
        let book = self
            .current_book()
            .ok_or_else(|| FolioError::NotFound("libro".into()))?;
        let path = self.store.delete_note(&book, &id, &self.clock)?;
        self.index.remove_note(&id)?;
        self.current_note = None;
        self.dirty = false;
        Ok(path)
    }

    pub fn delete_book(&mut self, id: &str) -> Result<PathBuf, FolioError> {
        self.flush()?;
        let book = self.store.book(id)?;
        let path = folio_store::trash::trash_book(
            &self.store.vault,
            &book,
            &self.clock,
            self.store.crypto(),
        )?;
        if self.current_book.as_deref() == Some(id) {
            self.current_book = None;
            self.current_note = None;
        }
        Ok(path)
    }

    pub fn move_note(
        &mut self,
        note_id: &str,
        to: &str,
        before: Option<&str>,
    ) -> Result<(), FolioError> {
        self.flush()?;
        let (from, _) = self.store.find_note(note_id)?;
        let to = self.store.book(to)?;
        let position = match before {
            Some(id) => MovePosition::Before(id),
            None => MovePosition::End,
        };
        self.store.move_note(note_id, &from, &to, position)?;
        self.reindex_note(note_id)
    }

    pub fn move_note_start(&mut self, note_id: &str, to: &str) -> Result<(), FolioError> {
        self.flush()?;
        let (from, _) = self.store.find_note(note_id)?;
        let to = self.store.book(to)?;
        self.store
            .move_note(note_id, &from, &to, MovePosition::Start)?;
        self.reindex_note(note_id)
    }

    fn reindex_note(&mut self, note_id: &str) -> Result<(), FolioError> {
        if let Ok((book, note)) = self.store.find_note(note_id)
            && let Ok(notes) = self.store.notes(&book)
            && let Some(summary) = notes.into_iter().find(|summary| summary.id == note_id)
        {
            self.index
                .index_note(&book, &note, &summary.path, &summary.order_key)?;
        }
        Ok(())
    }

    pub fn update_text(&mut self, block_id: &str, content: TextContent) {
        let Some(note) = self.current_note.as_mut() else {
            return;
        };
        if let Some(block) = note.block_mut(block_id) {
            block.content = folio_core::model::BlockContent::Text(content);
            note.touch(&self.clock);
            self.dirty = true;
        }
    }

    pub fn insert_block(&mut self, block: Block) -> Result<(), FolioError> {
        let Some(note) = self.current_note.as_mut() else {
            return Ok(());
        };
        self.undo.checkpoint(note);
        note.blocks.push(block);
        note.touch(&self.clock);
        self.dirty = true;
        Ok(())
    }

    pub fn remove_block(&mut self, block_id: &str) {
        if let Some(note) = self.current_note.as_mut() {
            self.undo.checkpoint(note);
            note.blocks.retain(|block| block.id != block_id);
            note.touch(&self.clock);
            self.dirty = true;
        }
    }

    pub fn duplicate_block(&mut self, block_id: &str) {
        if let Some(note) = self.current_note.as_mut()
            && let Some(mut copy) = note.block(block_id).cloned()
        {
            self.undo.checkpoint(note);
            copy.id = folio_core::new_id();
            if let folio_core::Placement::Free { x, y, .. } = &mut copy.placement {
                *x += 24.0;
                *y += 24.0;
            }
            note.blocks.push(copy);
            note.touch(&self.clock);
            self.dirty = true;
        }
    }

    pub fn move_block(&mut self, block_id: &str, delta: isize) {
        if let Some(note) = self.current_note.as_mut()
            && let Some(index) = note.blocks.iter().position(|block| block.id == block_id)
        {
            let target = (index as isize + delta).clamp(0, note.blocks.len() as isize - 1) as usize;
            if target != index {
                self.undo.checkpoint(note);
                note.blocks.swap(index, target);
                note.touch(&self.clock);
                self.dirty = true;
            }
        }
    }

    pub fn anchor_block(&mut self, block_id: &str, free: bool) {
        if let Some(note) = self.current_note.as_mut()
            && let Some(index) = note.blocks.iter().position(|block| block.id == block_id)
        {
            self.undo.checkpoint(note);
            if free {
                let y = 40.0 + index as f64 * 60.0;
                note.blocks[index].placement = folio_core::Placement::free(820.0, y, 320.0, 120.0);
            } else {
                note.blocks[index].placement = folio_core::Placement::Flow;
            }
            note.touch(&self.clock);
            self.dirty = true;
        }
    }

    pub fn set_placement(&mut self, block_id: &str, placement: folio_core::Placement) {
        if let Some(note) = self.current_note.as_mut()
            && let Some(block) = note.block_mut(block_id)
        {
            block.placement = placement;
            note.touch(&self.clock);
            self.dirty = true;
        }
    }

    pub fn recompose(&mut self) -> Result<(), FolioError> {
        let Some(note) = self.current_note.as_mut() else {
            return Ok(());
        };
        self.undo.checkpoint(note);
        let layout = folio_layout::layout_note(note, estimate_height);
        let order = layout.recompose_order();
        note.blocks.sort_by_key(|block| {
            order
                .iter()
                .position(|id| id == &block.id)
                .unwrap_or(usize::MAX)
        });
        for block in &mut note.blocks {
            block.placement = folio_core::Placement::Flow;
        }
        note.touch(&self.clock);
        self.dirty = true;
        self.save()?;
        Ok(())
    }

    pub fn set_config(&mut self, key: &str, raw: &str) -> Result<(), String> {
        let field = folio_config::schema()
            .into_iter()
            .find(|field| field.path == key)
            .ok_or_else(|| format!("clave desconocida: {key}"))?;
        let file = match field.scope {
            folio_config::Scope::Vault => self.store.root().join("vault.toml"),
            _ => folio_config::global_config_path(),
        };
        helpers::set_config_value(&file, key, raw)?;
        let config = helpers::load_config(Some(self.store.root()))?;
        self.config = config;
        Ok(())
    }

    pub fn plugin_manifest(&self, id: &str) -> Result<Manifest, PluginError> {
        if let Some(manifest) = builtin::manifest_for(id) {
            return Ok(manifest);
        }
        let path = self
            .store
            .root()
            .join("plugins")
            .join(id)
            .join("manifest.toml");
        manifest::load_file(&path)
    }

    /// Ejecuta un plugin instalado o de referencia sin mutar la nota.
    pub fn plugin_run(&self, id: &str, request: &Request) -> Result<Response, PluginError> {
        let manifest = self.plugin_manifest(id)?;
        let memory = self.config.int("plugins.max_memory_mb").max(8) as u32;
        let timeout = self.config.int("plugins.timeout_ms").max(10) as u32;
        let fuel = self.config.int("plugins.max_fuel").max(1) as u64;
        let context = self.registry.context(id, memory, timeout);
        let limits = SandboxLimits {
            memory_limit_mb: memory,
            timeout: std::time::Duration::from_millis(timeout as u64),
            max_state_bytes: 1024 * 1024,
            fuel,
        };
        if builtin::manifest_for(id).is_some() {
            let mut runtime = BuiltinRuntime { limits };
            runtime.call(&manifest, request, &context)
        } else {
            if !self.registry.enabled(id) {
                return Err(PluginError::NotFound(format!("{id} desactivado")));
            }
            let path = folio_plugins::registry::Registry::package_dir(self.store.root(), id)
                .join(&manifest.entry);
            let mut runtime = folio_plugins::wasm::WasmRuntime::from_file(&path, limits)?;
            runtime.call(&manifest, request, &context)
        }
    }

    pub fn plugin_command(
        &mut self,
        block_id: &str,
        command: &str,
        payload: serde_json::Value,
    ) -> Result<(), PluginError> {
        let (plugin_id, state) = {
            let note = self
                .current_note
                .as_ref()
                .ok_or_else(|| PluginError::NotFound("nota".into()))?;
            let block = note
                .block(block_id)
                .ok_or_else(|| PluginError::NotFound(block_id.into()))?;
            match &block.content {
                folio_core::model::BlockContent::Plugin(plugin) => {
                    (plugin.plugin.clone(), plugin.state.clone())
                }
                _ => return Err(PluginError::NotFound(block_id.into())),
            }
        };
        let response = self.plugin_run(
            &plugin_id,
            &folio_plugin_sdk::Request {
                command: command.to_string(),
                block_state: state,
                payload,
            },
        )?;
        if let Some(note) = self.current_note.as_mut()
            && let Some(block) = note.block_mut(block_id)
            && let folio_core::model::BlockContent::Plugin(plugin) = &mut block.content
        {
            plugin.state = response.state;
            note.touch(&self.clock);
            self.dirty = true;
        }
        Ok(())
    }

    pub fn insert_plugin_block(&mut self, plugin_id: &str, block: &str) -> Result<(), PluginError> {
        let note = self
            .current_note
            .as_ref()
            .ok_or_else(|| PluginError::NotFound("nota".into()))?;
        let id = note.id.clone();
        let _ = id;
        let content = folio_core::model::PluginContent {
            plugin: plugin_id.to_string(),
            block: block.to_string(),
            schema: 1,
            state: serde_json::json!({}),
            height: Some(240.0),
            extra: Default::default(),
        };
        let block = Block::new("plugin", folio_core::model::BlockContent::Plugin(content));
        let block_id = block.id.clone();
        self.insert_block(block)
            .map_err(|error| PluginError::Crash(error.to_string()))?;
        let _ = self.plugin_command(&block_id, "render", serde_json::Value::Null);
        Ok(())
    }

    pub fn restore_trash(&mut self, path: &Path) -> Result<PathBuf, FolioError> {
        folio_store::trash::restore(&self.store.vault, path, self.store.crypto())
    }

    pub fn create_backup(&mut self) -> Result<PathBuf, FolioError> {
        self.flush()?;
        folio_store::backup::create(&self.store.vault, &self.clock)
    }

    pub fn search(&mut self, query: &str) -> Result<(), FolioError> {
        self.search = query.to_string();
        Ok(())
    }

    pub fn export_current(&mut self, format: &str, out: &Path) -> Result<PathBuf, FolioError> {
        self.flush()?;
        let id = self
            .current_note
            .as_ref()
            .ok_or_else(|| FolioError::NotFound("nota".into()))?
            .id
            .clone();
        helpers::export_note(&self.store, &id, format, out)
    }

    pub fn import_markdown(&mut self, path: &Path) -> Result<Note, FolioError> {
        self.flush()?;
        let book = self
            .current_book()
            .ok_or_else(|| FolioError::NotFound("libro".into()))?;
        let note = helpers::import_markdown_file(&self.store, &book, path, &self.clock)?;
        self.current_note = Some(note.clone());
        let _ = self.index.rebuild(&self.store);
        Ok(note)
    }
}

pub fn estimate_height(block: &Block, width: f64) -> f64 {
    match &block.content {
        folio_core::model::BlockContent::Text(text) => {
            let lines: usize = text
                .paragraphs
                .iter()
                .map(|paragraph| {
                    let chars = paragraph.plain_text().chars().count();
                    (chars / (width.max(200.0) as usize / 8)).max(1)
                })
                .sum();
            (lines as f64 * 24.0 + 24.0).max(48.0)
        }
        folio_core::model::BlockContent::Table(table) => {
            (table.rows.max(1) as f64 * 32.0 + 16.0).max(64.0)
        }
        folio_core::model::BlockContent::Image(image) => image.height.unwrap_or(220.0).max(80.0),
        folio_core::model::BlockContent::Drawing(drawing) => drawing.height.max(200.0) * 0.5,
        folio_core::model::BlockContent::Audio(_) => 90.0,
        folio_core::model::BlockContent::Video(_) => 220.0,
        folio_core::model::BlockContent::File(_) => 80.0,
        folio_core::model::BlockContent::Web(_) => 120.0,
        folio_core::model::BlockContent::Plugin(plugin) => plugin.height.unwrap_or(240.0),
        folio_core::model::BlockContent::Raw(_) => 80.0,
    }
}

pub fn daily_config(config: &Config) -> DailyConfig {
    helpers::daily_config(config)
}

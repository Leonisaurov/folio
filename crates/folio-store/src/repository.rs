use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use folio_core::ordering::{FIRST_KEY, key_after, key_between, validate_sequence};
use folio_core::{Clock, FolioError, Note, ids};
use serde::{Deserialize, Serialize};

use crate::atomic::{ensure_dir, write_atomic};
use crate::book::{Book, folder_name};
use crate::trash;
use crate::vault::Vault;
use crate::versions;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteSummary {
    pub id: String,
    pub book_id: String,
    pub path: PathBuf,
    pub order_key: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub word_count: usize,
    pub block_count: usize,
    pub has_media: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovePosition<'a> {
    Start,
    End,
    Before(&'a str),
    After(&'a str),
}

#[derive(Debug, Clone)]
pub struct Store {
    pub vault: Vault,
    crypto: Option<Arc<crate::crypto::Cipher>>,
}

pub struct BookScan {
    pub books: Vec<Book>,
    pub errors: Vec<(PathBuf, FolioError)>,
}

impl Store {
    pub fn init(root: &Path, name: &str, clock: &dyn Clock) -> Result<Self, FolioError> {
        Ok(Self {
            vault: Vault::init(root, name, clock)?,
            crypto: None,
        })
    }

    /// Crea una bóveda cifrada con contraseña y devuelve la frase de recuperación.
    pub fn init_encrypted(
        root: &Path,
        name: &str,
        clock: &dyn Clock,
        password: &str,
    ) -> Result<(Self, String), FolioError> {
        let vault = Vault::init(root, name, clock)?;
        let (cipher, keyring, phrase) =
            crate::crypto::Cipher::create(password, &crate::crypto::KdfParams::default())?;
        crate::crypto::save_keyring(root, &keyring)?;
        let mut vault = vault;
        vault.meta.encryption.mode = "password".into();
        vault.save_meta()?;
        Ok((
            Self {
                vault,
                crypto: Some(Arc::new(cipher)),
            },
            phrase,
        ))
    }

    pub fn open(root: &Path) -> Result<Self, FolioError> {
        Ok(Self {
            vault: Vault::open(root)?,
            crypto: None,
        })
    }

    /// Devuelve `none` o `password` sin necesidad de desbloquear.
    pub fn encryption_mode(root: &Path) -> Result<String, FolioError> {
        Ok(Vault::open(root)?.meta.encryption.mode)
    }

    /// Convierte una bóveda en claro en cifrada. El llamador debe hacer backup.
    pub fn encrypt_in_place(&mut self, password: &str) -> Result<String, FolioError> {
        if self.is_encrypted() {
            return Err(FolioError::Conflict("la bóveda ya está cifrada".into()));
        }
        let (cipher, keyring, phrase) =
            crate::crypto::Cipher::create(password, &crate::crypto::KdfParams::default())?;
        let mut paths = Vec::new();
        for root in [
            self.vault.books_dir(),
            self.vault.assets_dir(),
            self.vault.folio_dir().join("versions"),
            self.vault.folio_dir().join("trash"),
            self.vault.folio_dir().join("recovery"),
        ] {
            if !root.exists() {
                continue;
            }
            for path in crate::atomic::walk_files(&root)? {
                paths.push(path);
            }
        }
        // El índice en claro no debe sobrevivir a la conversión.
        let index = self.vault.index_path();
        if index.exists() {
            let _ = fs::remove_file(&index);
            let _ = fs::remove_file(index.with_extension("sqlite-wal"));
            let _ = fs::remove_file(index.with_extension("sqlite-shm"));
        }
        // Primero los datos, después el modo: si algo falla, el keyring no se
        // publica y la bóveda sigue siendo legible como bóveda en claro.
        for path in &paths {
            let plain = fs::read(path)?;
            write_atomic(path, &cipher.encrypt(&plain)?)?;
        }
        crate::crypto::save_keyring(self.root(), &keyring)?;
        self.vault.meta.encryption.mode = "password".into();
        self.vault.save_meta()?;
        self.crypto = Some(Arc::new(cipher));
        Ok(phrase)
    }

    /// Deshace el cifrado dejando los archivos en claro (útil para migrar o
    /// recuperar con herramientas externas). Requiere la contraseña.
    pub fn decrypt_in_place(&mut self, password: &str) -> Result<(), FolioError> {
        if !self.is_encrypted() {
            return Err(FolioError::Conflict("la bóveda no está cifrada".into()));
        }
        let cipher = match &self.crypto {
            Some(cipher) => cipher.clone(),
            None => {
                let keyring = crate::crypto::load_keyring(self.root())?;
                Arc::new(crate::crypto::Cipher::unlock(password, &keyring)?)
            }
        };
        let mut paths = Vec::new();
        for root in [
            self.vault.books_dir(),
            self.vault.assets_dir(),
            self.vault.folio_dir().join("versions"),
            self.vault.folio_dir().join("trash"),
            self.vault.folio_dir().join("recovery"),
        ] {
            if !root.exists() {
                continue;
            }
            for path in crate::atomic::walk_files(&root)? {
                paths.push(path);
            }
        }
        for path in &paths {
            let encrypted = fs::read(path)?;
            let plain = cipher.decrypt(&encrypted)?;
            write_atomic(path, &plain)?;
        }
        let _ = fs::remove_file(crate::crypto::keyring_path(self.root()));
        self.vault.meta.encryption.mode = "none".into();
        self.vault.save_meta()?;
        self.crypto = None;
        Ok(())
    }

    pub fn open_with_password(root: &Path, password: &str) -> Result<Self, FolioError> {
        let vault = Vault::open(root)?;
        if vault.meta.encryption.mode != "password" {
            return Ok(Self {
                vault,
                crypto: None,
            });
        }
        let keyring = crate::crypto::load_keyring(root)?;
        let cipher = crate::crypto::Cipher::unlock(password, &keyring)?;
        // Un índice en claro de una versión anterior nunca debe sobrevivir.
        let index = vault.index_path();
        for path in [
            index.clone(),
            index.with_extension("sqlite-wal"),
            index.with_extension("sqlite-shm"),
        ] {
            let _ = fs::remove_file(path);
        }
        Ok(Self {
            vault,
            crypto: Some(Arc::new(cipher)),
        })
    }

    pub fn open_with_recovery(root: &Path, phrase: &str) -> Result<Self, FolioError> {
        let vault = Vault::open(root)?;
        let keyring = crate::crypto::load_keyring(root)?;
        let cipher = crate::crypto::Cipher::from_recovery(phrase, &keyring)?;
        let index = vault.index_path();
        for path in [
            index.clone(),
            index.with_extension("sqlite-wal"),
            index.with_extension("sqlite-shm"),
        ] {
            let _ = fs::remove_file(path);
        }
        Ok(Self {
            vault,
            crypto: Some(Arc::new(cipher)),
        })
    }

    pub fn is_encrypted(&self) -> bool {
        self.vault.meta.encryption.mode == "password"
    }

    pub fn is_unlocked(&self) -> bool {
        !self.is_encrypted() || self.crypto.is_some()
    }

    pub fn crypto(&self) -> Option<&crate::crypto::Cipher> {
        self.crypto.as_deref()
    }

    pub fn unlock(&mut self, password: &str) -> Result<(), FolioError> {
        if !self.is_encrypted() {
            return Ok(());
        }
        let keyring = crate::crypto::load_keyring(self.root())?;
        self.crypto = Some(Arc::new(crate::crypto::Cipher::unlock(password, &keyring)?));
        Ok(())
    }

    pub fn unlock_recovery(&mut self, phrase: &str) -> Result<(), FolioError> {
        if !self.is_encrypted() {
            return Ok(());
        }
        let keyring = crate::crypto::load_keyring(self.root())?;
        self.crypto = Some(Arc::new(crate::crypto::Cipher::from_recovery(
            phrase, &keyring,
        )?));
        Ok(())
    }

    pub fn lock(&mut self) {
        self.crypto = None;
        self.clear_plain_cache();
    }

    /// Borra las copias descifradas de assets usadas solo para mostrar.
    pub fn clear_plain_cache(&self) {
        let cache = self.vault.folio_dir().join("cache").join("plain");
        if cache.exists() {
            let _ = fs::remove_dir_all(cache);
        }
    }

    fn ensure_unlocked(&self) -> Result<(), FolioError> {
        if !self.is_unlocked() {
            return Err(FolioError::Locked(self.vault.root.display().to_string()));
        }
        Ok(())
    }

    pub fn read_file(&self, path: &Path) -> Result<Vec<u8>, FolioError> {
        self.ensure_unlocked()?;
        let bytes = fs::read(path)?;
        match &self.crypto {
            Some(cipher) => {
                if !crate::crypto::is_encrypted(&bytes) {
                    return Err(FolioError::Corrupt(format!(
                        "archivo sin cifrar en bóveda cifrada: {}",
                        path.display()
                    )));
                }
                cipher.decrypt(&bytes)
            }
            None => Ok(bytes),
        }
    }

    pub fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<(), FolioError> {
        self.ensure_unlocked()?;
        match &self.crypto {
            Some(cipher) => write_atomic(path, &cipher.encrypt(bytes)?),
            None => write_atomic(path, bytes),
        }
    }

    pub fn load_note_at(&self, path: &Path) -> Result<Note, FolioError> {
        let bytes = self.read_file(path)?;
        let note = Note::from_json(&bytes).map_err(|error| match error {
            FolioError::Json(_) => {
                FolioError::Corrupt(format!("{}: json inválido", path.display()))
            }
            other => other,
        })?;
        note.validate()?;
        Ok(note)
    }

    pub fn asset_display_path(&self, hash: &str) -> Result<Option<PathBuf>, FolioError> {
        let Some(source) = crate::assets::resolve(&self.vault, hash)? else {
            return Ok(None);
        };
        let Some(cipher) = &self.crypto else {
            return Ok(Some(source));
        };
        let dir = self.vault.folio_dir().join("cache").join("plain");
        std::fs::create_dir_all(&dir)?;
        let extension = source
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("bin");
        let target = dir.join(format!("{hash}.{extension}"));
        if !target.exists() {
            let encrypted = fs::read(&source)?;
            let plain = cipher.decrypt(&encrypted)?;
            write_atomic(&target, &plain)?;
            #[cfg(unix)]
            crate::atomic::set_mode(&target, 0o600)?;
        }
        Ok(Some(target))
    }

    pub fn root(&self) -> &Path {
        &self.vault.root
    }

    /// Lista libros tolerando carpetas corruptas (se reportan aparte).
    pub fn scan_books(&self) -> Result<BookScan, FolioError> {
        self.ensure_unlocked()?;
        let dir = self.vault.books_dir();
        ensure_dir(&dir)?;
        let mut books = Vec::new();
        let mut errors = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let path = entry.path();
            let folder = entry.file_name().to_string_lossy().to_string();
            match Book::load_with(&path, folder, self.crypto()) {
                Ok(book) => books.push(book),
                Err(error) => errors.push((path, error)),
            }
        }
        books.sort_by(|a, b| {
            b.meta
                .pinned
                .cmp(&a.meta.pinned)
                .then_with(|| a.meta.sort.cmp(&b.meta.sort))
                .then_with(|| a.meta.title.cmp(&b.meta.title))
        });
        Ok(BookScan { books, errors })
    }

    pub fn list_books(&self) -> Result<Vec<Book>, FolioError> {
        Ok(self.scan_books()?.books)
    }

    pub fn book(&self, book_id: &str) -> Result<Book, FolioError> {
        self.list_books()?
            .into_iter()
            .find(|book| book.meta.id == book_id)
            .ok_or_else(|| FolioError::NotFound(format!("libro {book_id}")))
    }

    pub fn daily_book(&self, date: &str) -> Result<Option<Book>, FolioError> {
        Ok(self.list_books()?.into_iter().find(|book| {
            book.meta.kind == "daily" && book.meta.daily_date.as_deref() == Some(date)
        }))
    }

    pub fn create_book(
        &self,
        title: &str,
        kind: &str,
        daily_date: Option<&str>,
        timezone: Option<&str>,
        folder_base: &str,
        clock: &dyn Clock,
    ) -> Result<Book, FolioError> {
        if kind == "daily"
            && let Some(date) = daily_date
            && let Some(existing) = self.daily_book(date)?
        {
            return Ok(existing);
        }
        let id = ids::new_id();
        let folder = if self.is_encrypted() {
            id.clone()
        } else {
            folder_name(folder_base, &id)
        };
        let mut book = Book::new(
            title,
            kind,
            daily_date.map(str::to_string),
            timezone.map(str::to_string),
            folder,
            clock,
        );
        book.meta.id = id;
        book.create_on_disk_with(&self.vault, self.crypto())?;
        Ok(book)
    }

    pub fn save_book(&self, book: &mut Book, clock: &dyn Clock) -> Result<(), FolioError> {
        book.touch(clock);
        book.save_with(self.crypto())
    }

    pub fn notes(&self, book: &Book) -> Result<Vec<NoteSummary>, FolioError> {
        self.ensure_unlocked()?;
        let dir = book.notes_dir();
        ensure_dir(&dir)?;
        let mut notes = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let Some((order_key, _)) = parse_note_filename(&name) else {
                continue;
            };
            match load_note_file_with(&entry.path(), self.crypto()) {
                Ok(note) => notes.push(summary(&note, &book.meta.id, entry.path(), order_key)),
                Err(error) => {
                    tracing::warn!("nota ilegible {}: {error}", entry.path().display());
                }
            }
        }
        notes.sort_by(|a, b| a.order_key.cmp(&b.order_key));
        Ok(notes)
    }

    pub fn note_count(&self) -> Result<usize, FolioError> {
        let mut count = 0;
        for book in self.list_books()? {
            count += self.notes(&book)?.len();
        }
        Ok(count)
    }

    pub fn create_note(
        &self,
        book: &Book,
        title: &str,
        clock: &dyn Clock,
    ) -> Result<Note, FolioError> {
        ensure_dir(&book.notes_dir())?;
        let keys = self.note_keys(book)?;
        let key = match keys.last() {
            Some(last) => key_after(Some(last)).ok_or_else(|| {
                FolioError::Conflict("sin espacio de orden; rebalancea el libro".into())
            })?,
            None => FIRST_KEY.to_string(),
        };
        let note = Note::new(title, clock);
        let path = book.notes_dir().join(format!("{key}_{}.json", note.id));
        let json = note.to_pretty_json()?;
        self.write_file(&path, json.as_bytes())?;
        Ok(note)
    }

    pub fn load_note(&self, book: &Book, note_id: &str) -> Result<Note, FolioError> {
        let (path, _key) = self.note_location(book, note_id)?;
        self.load_note_at(&path)
    }

    pub fn find_note(&self, note_id: &str) -> Result<(Book, Note), FolioError> {
        for book in self.list_books()? {
            if let Ok((path, _key)) = self.note_location(&book, note_id) {
                let note = self.load_note_at(&path)?;
                return Ok((book, note));
            }
        }
        Err(FolioError::NotFound(format!("nota {note_id}")))
    }

    /// Guarda una nota en su libro, conservando el `order_key` si ya existía.
    pub fn save_note(
        &self,
        book: &Book,
        note: &mut Note,
        clock: &dyn Clock,
    ) -> Result<PathBuf, FolioError> {
        note.schema = folio_core::model::SCHEMA_VERSION;
        note.touch(clock);
        note.validate()?;
        let path = match self.note_location(book, &note.id) {
            Ok((path, _key)) => path,
            Err(FolioError::NotFound(_)) => {
                let keys = self.note_keys(book)?;
                let key = match keys.last() {
                    Some(last) => key_after(Some(last)).unwrap_or_else(|| FIRST_KEY.to_string()),
                    None => FIRST_KEY.to_string(),
                };
                book.notes_dir().join(format!("{key}_{}.json", note.id))
            }
            Err(error) => return Err(error),
        };
        let json = note.to_pretty_json()?;
        self.write_file(&path, json.as_bytes())?;
        let _ = versions::maybe_snapshot(&self.vault, note, clock, false, self.crypto());
        Ok(path)
    }

    pub fn delete_note(
        &self,
        book: &Book,
        note_id: &str,
        clock: &dyn Clock,
    ) -> Result<PathBuf, FolioError> {
        let (path, _key) = self.note_location(book, note_id)?;
        let note = self.load_note_at(&path)?;
        trash::trash_note(&self.vault, book, &path, &note, clock, self.crypto())
    }

    pub fn move_note(
        &self,
        note_id: &str,
        from: &Book,
        to: &Book,
        position: MovePosition<'_>,
    ) -> Result<PathBuf, FolioError> {
        if from.meta.id == to.meta.id {
            return self.reorder_note(from, note_id, position);
        }
        let (source, _key) = self.note_location(from, note_id)?;
        let note = self.load_note_at(&source)?;
        ensure_dir(&to.notes_dir())?;
        let target_keys = self.note_keys(to)?;
        let key = match position {
            MovePosition::Start => match target_keys.first() {
                Some(first) => match key_between(None, Some(first)) {
                    Some(key) => key,
                    None => {
                        self.rebalance(to)?;
                        let keys = self.note_keys(to)?;
                        match keys.first() {
                            Some(first) => key_between(None, Some(first)).ok_or_else(|| {
                                FolioError::Conflict("sin espacio al inicio".into())
                            })?,
                            None => FIRST_KEY.to_string(),
                        }
                    }
                },
                None => FIRST_KEY.to_string(),
            },
            MovePosition::End => match target_keys.last() {
                Some(last) => key_after(Some(last))
                    .ok_or_else(|| FolioError::Conflict("sin espacio al final".into()))?,
                None => FIRST_KEY.to_string(),
            },
            MovePosition::Before(id) => {
                let anchor = self
                    .notes(to)?
                    .into_iter()
                    .find(|summary| summary.id == id)
                    .ok_or_else(|| FolioError::NotFound(format!("nota {id}")))?;
                let position = self
                    .note_keys(to)?
                    .iter()
                    .position(|key| key == &anchor.order_key)
                    .unwrap_or(0);
                let keys = self.note_keys(to)?;
                let prev = position.checked_sub(1).and_then(|index| keys.get(index));
                key_between(prev.map(String::as_str), Some(&anchor.order_key))
                    .ok_or_else(|| FolioError::Conflict("sin espacio entre notas".into()))?
            }
            MovePosition::After(id) => {
                let anchor = self
                    .notes(to)?
                    .into_iter()
                    .find(|summary| summary.id == id)
                    .ok_or_else(|| FolioError::NotFound(format!("nota {id}")))?;
                let keys = self.note_keys(to)?;
                let position = keys
                    .iter()
                    .position(|key| key == &anchor.order_key)
                    .unwrap_or(keys.len().saturating_sub(1));
                let next = keys.get(position + 1);
                key_between(Some(&anchor.order_key), next.map(String::as_str))
                    .ok_or_else(|| FolioError::Conflict("sin espacio entre notas".into()))?
            }
        };
        let target = to.notes_dir().join(format!("{key}_{}.json", note.id));
        if target.exists() {
            return Err(FolioError::Conflict(format!(
                "el destino ya existe: {}",
                target.display()
            )));
        }
        // El rename es atómico dentro del mismo sistema de archivos.
        match fs::rename(&source, &target) {
            Ok(()) => {}
            Err(error) if error.raw_os_error() == Some(18) => {
                // EXDEV: copia + borrado, con el mismo nombre final.
                fs::copy(&source, &target)?;
                fs::remove_file(&source)?;
            }
            Err(error) => return Err(error.into()),
        }
        Ok(target)
    }

    pub fn reorder_note(
        &self,
        book: &Book,
        note_id: &str,
        position: MovePosition<'_>,
    ) -> Result<PathBuf, FolioError> {
        let (source, old_key) = self.note_location(book, note_id)?;
        let keys: Vec<String> = self
            .note_keys(book)?
            .into_iter()
            .filter(|key| key != &old_key)
            .collect();
        let key = match position {
            MovePosition::Start => match keys.first() {
                Some(first) => match key_between(None, Some(first)) {
                    Some(key) => key,
                    None => {
                        // Sin espacio: rebalanceo y reintento una vez.
                        self.rebalance(book)?;
                        let (source, old_key) = self.note_location(book, note_id)?;
                        let keys: Vec<String> = self
                            .note_keys(book)?
                            .into_iter()
                            .filter(|key| key != &old_key)
                            .collect();
                        let _ = source;
                        match keys.first() {
                            Some(first) => key_between(None, Some(first)).ok_or_else(|| {
                                FolioError::Conflict("sin espacio al inicio".into())
                            })?,
                            None => FIRST_KEY.to_string(),
                        }
                    }
                },
                None => FIRST_KEY.to_string(),
            },
            MovePosition::End => match keys.last() {
                Some(last) => key_after(Some(last))
                    .ok_or_else(|| FolioError::Conflict("sin espacio al final".into()))?,
                None => FIRST_KEY.to_string(),
            },
            MovePosition::Before(id) => {
                let summaries = self.notes(book)?;
                let anchor = summaries
                    .iter()
                    .find(|summary| summary.id == id)
                    .ok_or_else(|| FolioError::NotFound(format!("nota {id}")))?;
                let position = keys
                    .iter()
                    .position(|key| key == &anchor.order_key)
                    .unwrap_or(0);
                let prev = position.checked_sub(1).and_then(|index| keys.get(index));
                key_between(prev.map(String::as_str), Some(&anchor.order_key))
                    .ok_or_else(|| FolioError::Conflict("sin espacio entre notas".into()))?
            }
            MovePosition::After(id) => {
                let summaries = self.notes(book)?;
                let anchor = summaries
                    .iter()
                    .find(|summary| summary.id == id)
                    .ok_or_else(|| FolioError::NotFound(format!("nota {id}")))?;
                let position = keys
                    .iter()
                    .position(|key| key == &anchor.order_key)
                    .unwrap_or(keys.len().saturating_sub(1));
                let next = keys.get(position + 1);
                key_between(Some(&anchor.order_key), next.map(String::as_str))
                    .ok_or_else(|| FolioError::Conflict("sin espacio entre notas".into()))?
            }
        };
        let target = book.notes_dir().join(format!("{key}_{}.json", note_id));
        if target.exists() {
            return Err(FolioError::Conflict(format!(
                "ya existe {}",
                target.display()
            )));
        }
        fs::rename(&source, &target)?;
        Ok(target)
    }

    pub fn rebalance(&self, book: &Book) -> Result<(), FolioError> {
        let summaries = self.notes(book)?;
        let mut next = FIRST_KEY.to_string();
        for summary in &summaries {
            let mut target = book.notes_dir().join(format!("{next}_{}.json", summary.id));
            // Nunca sobrescribir otra nota durante el rebalanceo: si la clave
            // nueva ya está ocupada, se avanza hasta encontrar un hueco.
            while target.exists() && target != summary.path {
                next = key_after(Some(&next))
                    .ok_or_else(|| FolioError::Conflict("rebalanceo agotado".into()))?;
                target = book.notes_dir().join(format!("{next}_{}.json", summary.id));
            }
            if target != summary.path {
                fs::rename(&summary.path, &target)?;
            }
            next = key_after(Some(&next))
                .ok_or_else(|| FolioError::Conflict("rebalanceo agotado".into()))?;
        }
        validate_sequence(
            &self
                .notes(book)?
                .into_iter()
                .map(|summary| summary.order_key)
                .collect::<Vec<_>>(),
        )
    }

    pub fn note_keys(&self, book: &Book) -> Result<Vec<String>, FolioError> {
        let mut keys: Vec<String> = fs::read_dir(book.notes_dir())?
            .filter_map(Result::ok)
            .filter_map(|entry| parse_note_filename(&entry.file_name().to_string_lossy()))
            .map(|(key, _)| key)
            .collect();
        keys.sort();
        Ok(keys)
    }

    fn note_location(&self, book: &Book, note_id: &str) -> Result<(PathBuf, String), FolioError> {
        for entry in fs::read_dir(book.notes_dir())? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some((key, id)) = parse_note_filename(&name)
                && id == note_id
            {
                return Ok((entry.path(), key));
            }
        }
        Err(FolioError::NotFound(format!("nota {note_id}")))
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<NoteSummary>, FolioError> {
        let terms: Vec<String> = query
            .split_whitespace()
            .filter(|term| !term.is_empty())
            .map(|term| term.to_lowercase())
            .collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let mut results = Vec::new();
        for book in self.list_books()? {
            for summary in self.notes(&book)? {
                if results.len() >= limit {
                    return Ok(results);
                }
                let note = match self.load_note_at(&summary.path) {
                    Ok(note) => note,
                    Err(_) => continue,
                };
                let haystack = format!(
                    "{}\n{}",
                    note.display_title().to_lowercase(),
                    note.plain_text().to_lowercase()
                );
                if terms.iter().all(|term| haystack.contains(term)) {
                    results.push(summary);
                }
            }
        }
        Ok(results)
    }
}

fn summary(note: &Note, book_id: &str, path: PathBuf, order_key: String) -> NoteSummary {
    NoteSummary {
        id: note.id.clone(),
        book_id: book_id.to_string(),
        path,
        order_key,
        title: note.display_title(),
        created_at: note.created_at.clone(),
        updated_at: note.updated_at.clone(),
        word_count: note.meta.word_count.max(note.word_count()),
        block_count: note.blocks.len(),
        has_media: note.has_media(),
    }
}

pub fn parse_note_filename(name: &str) -> Option<(String, String)> {
    let stem = name.strip_suffix(".json")?;
    if stem.starts_with('.') {
        return None;
    }
    let (key, id) = stem.split_once('_')?;
    if key.is_empty() || id.is_empty() || !folio_core::ordering::is_valid_key(key) {
        return None;
    }
    Some((key.to_string(), id.to_string()))
}

pub fn load_note_file(path: &Path) -> Result<Note, FolioError> {
    let bytes = fs::read(path)?;
    let note = Note::from_json(&bytes).map_err(|error| match error {
        FolioError::Json(_) => FolioError::Corrupt(format!("{}: json inválido", path.display())),
        other => other,
    })?;
    note.validate()?;
    Ok(note)
}

pub fn load_note_file_with(
    path: &Path,
    cipher: Option<&crate::crypto::Cipher>,
) -> Result<Note, FolioError> {
    let bytes = fs::read(path)?;
    let bytes = match cipher {
        Some(cipher) => {
            if !crate::crypto::is_encrypted(&bytes) {
                return Err(FolioError::Corrupt(format!(
                    "nota sin cifrar en bóveda cifrada: {}",
                    path.display()
                )));
            }
            cipher.decrypt(&bytes)?
        }
        None => bytes,
    };
    let note = Note::from_json(&bytes).map_err(|error| match error {
        FolioError::Json(_) => FolioError::Corrupt(format!("{}: json inválido", path.display())),
        other => other,
    })?;
    note.validate()?;
    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;
    use folio_core::{FixedClock, model::TextContent};

    fn store() -> (tempfile::TempDir, Store, FixedClock) {
        let dir = tempfile::tempdir().unwrap();
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let store = Store::init(dir.path(), "Test", &clock).unwrap();
        (dir, store, clock)
    }

    #[test]
    fn book_and_note_crud() {
        let (_dir, store, clock) = store();
        let book = store
            .create_book(
                "Hoy",
                "daily",
                Some("2026-10-29"),
                Some("UTC"),
                "2026-10-29",
                &clock,
            )
            .unwrap();
        let mut note = store.create_note(&book, "Primera", &clock).unwrap();
        note.blocks
            .push(folio_core::Block::text_plain("hola mundo"));
        store.save_note(&book, &mut note, &clock).unwrap();
        let loaded = store.load_note(&book, &note.id).unwrap();
        assert_eq!(loaded.plain_text(), "hola mundo");
        let summaries = store.notes(&book).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].title, "Primera");
    }

    #[test]
    fn daily_book_is_unique_by_date() {
        let (_dir, store, clock) = store();
        let a = store
            .create_book(
                "Hoy",
                "daily",
                Some("2026-10-29"),
                None,
                "2026-10-29",
                &clock,
            )
            .unwrap();
        let b = store
            .create_book(
                "Otra vez",
                "daily",
                Some("2026-10-29"),
                None,
                "2026-10-29",
                &clock,
            )
            .unwrap();
        assert_eq!(a.meta.id, b.meta.id);
        assert_eq!(store.list_books().unwrap().len(), 1);
    }

    #[test]
    fn rename_book_does_not_create_duplicate() {
        let (_dir, store, clock) = store();
        let mut book = store
            .create_book(
                "29/10/2026",
                "daily",
                Some("2026-10-29"),
                None,
                "2026-10-29",
                &clock,
            )
            .unwrap();
        book.meta.title = "Mi día".into();
        store.save_book(&mut book, &clock).unwrap();
        let again = store
            .create_book(
                "29/10/2026",
                "daily",
                Some("2026-10-29"),
                None,
                "2026-10-29",
                &clock,
            )
            .unwrap();
        assert_eq!(again.meta.id, book.meta.id);
        assert_eq!(again.meta.title, "Mi día");
    }

    #[test]
    fn order_and_move_100_notes_between_books() {
        let (_dir, store, clock) = store();
        let a = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        let b = store
            .create_book("B", "manual", None, None, "b", &clock)
            .unwrap();
        let mut ids = Vec::new();
        for index in 0..100 {
            let note = store.create_note(&a, &format!("n{index}"), &clock).unwrap();
            ids.push(note.id);
        }
        // Mover la mitad a B, intercalando posiciones.
        for id in ids.iter().take(50) {
            store.move_note(id, &a, &b, MovePosition::End).unwrap();
        }
        assert_eq!(store.notes(&a).unwrap().len(), 50);
        assert_eq!(store.notes(&b).unwrap().len(), 50);
        // Mover de vuelta al principio.
        for id in ids.iter().take(50) {
            store.move_note(id, &b, &a, MovePosition::Start).unwrap();
        }
        assert_eq!(store.notes(&a).unwrap().len(), 100);
        assert!(store.notes(&b).unwrap().is_empty());
        let keys: Vec<String> = store
            .notes(&a)
            .unwrap()
            .into_iter()
            .map(|summary| summary.order_key)
            .collect();
        assert_eq!(keys.len(), 100);
        validate_sequence(&keys).unwrap();
        assert_eq!(store.note_count().unwrap(), 100);
    }

    #[test]
    fn reorder_within_book() {
        let (_dir, store, clock) = store();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        let mut created = Vec::new();
        for index in 0..5 {
            created.push(
                store
                    .create_note(&book, &format!("n{index}"), &clock)
                    .unwrap(),
            );
        }
        store
            .reorder_note(&book, &created[4].id, MovePosition::Start)
            .unwrap();
        let order: Vec<String> = store
            .notes(&book)
            .unwrap()
            .into_iter()
            .map(|summary| summary.id)
            .collect();
        assert_eq!(order[0], created[4].id);
        store
            .reorder_note(&book, &created[4].id, MovePosition::After(&created[1].id))
            .unwrap();
        let order: Vec<String> = store
            .notes(&book)
            .unwrap()
            .into_iter()
            .map(|summary| summary.id)
            .collect();
        assert_eq!(order[2], created[4].id);
    }

    #[test]
    fn delete_note_goes_to_trash_and_restores() {
        let (_dir, store, clock) = store();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        let note = store.create_note(&book, "x", &clock).unwrap();
        let trashed = store.delete_note(&book, &note.id, &clock).unwrap();
        assert!(trashed.exists());
        assert!(store.load_note(&book, &note.id).is_err());
        let restored = trash::restore(&store.vault, &trashed, None).unwrap();
        assert!(restored.exists());
        assert_eq!(store.load_note(&book, &note.id).unwrap().id, note.id);
    }

    #[test]
    fn save_keeps_raw_unknown_content() {
        let (_dir, store, clock) = store();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        let note = store.create_note(&book, "x", &clock).unwrap();
        let path = store.notes(&book).unwrap()[0].path.clone();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["blocks"] = serde_json::json!([{
            "id": "b", "type": "futuro", "placement": {"kind": "flow"},
            "content": {"campo": 42}
        }]);
        write_atomic(&path, serde_json::to_string(&value).unwrap().as_bytes()).unwrap();
        let loaded = store.load_note(&book, &note.id).unwrap();
        assert_eq!(loaded.blocks[0].kind, "futuro");
        assert_eq!(
            loaded.blocks[0].content,
            folio_core::model::BlockContent::Raw(serde_json::json!({"campo":42}))
        );
        // Y una nota de texto vacía sigue siendo válida.
        let _ = TextContent::default();
    }
}

use std::fs;
use std::path::{Path, PathBuf};

use folio_core::{Clock, FolioError, Note};
use serde::{Deserialize, Serialize};

use crate::atomic::{ensure_dir, write_atomic};
use crate::book::Book;
use crate::vault::Vault;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashMeta {
    pub kind: String,
    pub original_path: PathBuf,
    pub book_id: String,
    pub note_id: String,
    pub title: String,
    pub deleted_at: String,
}

#[derive(Debug, Clone)]
pub struct TrashEntry {
    pub dir: PathBuf,
    pub meta: TrashMeta,
}

pub fn trash_note(
    vault: &Vault,
    book: &Book,
    note_path: &Path,
    note: &Note,
    clock: &dyn Clock,
) -> Result<PathBuf, FolioError> {
    let dir = vault
        .trash_dir()
        .join(format!("{}-{}", clock.now().as_millisecond(), note.id));
    ensure_dir(&dir)?;
    let meta = TrashMeta {
        kind: "note".into(),
        original_path: note_path.to_path_buf(),
        book_id: book.meta.id.clone(),
        note_id: note.id.clone(),
        title: note.display_title(),
        deleted_at: clock.now_rfc3339(),
    };
    write_atomic(
        &dir.join("meta.json"),
        serde_json::to_vec_pretty(&meta)?.as_slice(),
    )?;
    let payload = dir.join(format!("{}.json", note.id));
    fs::rename(note_path, &payload).or_else(|_| {
        fs::copy(note_path, &payload)?;
        fs::remove_file(note_path)?;
        Ok::<(), std::io::Error>(())
    })?;
    Ok(dir)
}

pub fn list(vault: &Vault) -> Result<Vec<TrashEntry>, FolioError> {
    let mut entries = Vec::new();
    ensure_dir(&vault.trash_dir())?;
    for entry in fs::read_dir(vault.trash_dir())? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let meta_path = entry.path().join("meta.json");
        if !meta_path.exists() {
            continue;
        }
        let meta: TrashMeta = serde_json::from_slice(&fs::read(&meta_path)?)?;
        entries.push(TrashEntry {
            dir: entry.path(),
            meta,
        });
    }
    entries.sort_by(|a, b| b.meta.deleted_at.cmp(&a.meta.deleted_at));
    Ok(entries)
}

/// Restaura una entrada de papelera a su ruta original.
pub fn restore(vault: &Vault, entry_dir: &Path) -> Result<PathBuf, FolioError> {
    let meta_path = entry_dir.join("meta.json");
    let meta: TrashMeta = serde_json::from_slice(&fs::read(&meta_path)?)?;
    if meta.kind == "book" {
        restore_book(vault, entry_dir, &meta)
    } else {
        restore_note(vault, entry_dir, &meta)
    }
}

fn restore_note(vault: &Vault, entry_dir: &Path, meta: &TrashMeta) -> Result<PathBuf, FolioError> {
    let payload = entry_dir.join(format!("{}.json", meta.note_id));
    if !payload.exists() {
        return Err(FolioError::Corrupt(format!(
            "entrada de papelera sin payload: {}",
            entry_dir.display()
        )));
    }
    let target = if meta.original_path.starts_with(vault.root.as_path()) {
        meta.original_path.clone()
    } else {
        // Nunca escribimos fuera de la bóveda.
        vault
            .books_dir()
            .join(&meta.book_id)
            .join("notes")
            .join(format!("restored-{}.json", meta.note_id))
    };
    if target.exists() {
        return Err(FolioError::Conflict(format!(
            "la ruta original ya existe: {}",
            target.display()
        )));
    }
    ensure_dir(target.parent().expect("ruta con padre"))?;
    fs::rename(&payload, &target)?;
    fs::remove_dir_all(entry_dir)?;
    Ok(target)
}

fn restore_book(vault: &Vault, entry_dir: &Path, meta: &TrashMeta) -> Result<PathBuf, FolioError> {
    let payload = entry_dir.join("payload");
    if !payload.exists() {
        return Err(FolioError::Corrupt(
            "entrada de papelera sin payload".into(),
        ));
    }
    let target = if meta.original_path.starts_with(vault.root.as_path()) {
        meta.original_path.clone()
    } else {
        vault.books_dir().join(&meta.note_id)
    };
    if target.exists() {
        return Err(FolioError::Conflict(format!(
            "el libro ya existe: {}",
            target.display()
        )));
    }
    fs::rename(&payload, &target)?;
    fs::remove_dir_all(entry_dir)?;
    Ok(target)
}

pub fn trash_book(vault: &Vault, book: &Book, clock: &dyn Clock) -> Result<PathBuf, FolioError> {
    let dir = vault.trash_dir().join(format!(
        "book-{}-{}",
        clock.now().as_millisecond(),
        book.meta.id
    ));
    ensure_dir(&dir)?;
    let meta = TrashMeta {
        kind: "book".into(),
        original_path: book.dir.clone(),
        book_id: book.meta.id.clone(),
        note_id: book.meta.id.clone(),
        title: book.meta.title.clone(),
        deleted_at: clock.now_rfc3339(),
    };
    write_atomic(
        &dir.join("meta.json"),
        serde_json::to_vec_pretty(&meta)?.as_slice(),
    )?;
    fs::rename(&book.dir, dir.join("payload"))?;
    Ok(dir)
}

/// Purga entradas más antiguas que `days` (0 = desactivado).
pub fn purge(vault: &Vault, days: u32, clock: &dyn Clock) -> Result<usize, FolioError> {
    if days == 0 {
        return Ok(0);
    }
    let cutoff = clock.now().as_second() - i64::from(days) * 86_400;
    let mut removed = 0;
    for entry in list(vault)? {
        let deleted: Result<jiff::Timestamp, _> = entry.meta.deleted_at.parse();
        if let Ok(deleted) = deleted
            && deleted.as_second() < cutoff
        {
            fs::remove_dir_all(&entry.dir)?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::Store;
    use folio_core::FixedClock;

    #[test]
    fn note_trash_restore() {
        let dir = tempfile::tempdir().unwrap();
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let store = Store::init(dir.path(), "v", &clock).unwrap();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        let note = store.create_note(&book, "x", &clock).unwrap();
        let trashed = store.delete_note(&book, &note.id, &clock).unwrap();
        assert_eq!(list(&store.vault).unwrap().len(), 1);
        restore(&store.vault, &trashed).unwrap();
        assert!(store.load_note(&book, &note.id).is_ok());
        assert!(list(&store.vault).unwrap().is_empty());
    }
}

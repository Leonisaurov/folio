use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use folio_core::{FolioError, Note, model::referenced_assets};
use serde::{Deserialize, Serialize};

use crate::atomic::{ensure_dir, write_atomic};
use crate::index::Index;
use crate::repository::{Store, load_note_file};
use crate::vault::Vault;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub books: usize,
    pub notes: usize,
    pub corrupt: Vec<String>,
    pub quarantined: Vec<String>,
    pub orphan_assets: Vec<String>,
    pub missing_assets: Vec<String>,
    pub reindexed: usize,
    pub pending_ops: usize,
}

/// Repara lo reparable y pone corruptos en cuarentena (nunca los borra).
pub fn repair(store: &Store, reindex: bool) -> Result<DoctorReport, FolioError> {
    let vault = &store.vault;
    ensure_dir(&vault.quarantine_dir())?;
    let mut books = 0;
    let mut notes = 0;
    let mut corrupt = Vec::new();
    let mut quarantined = Vec::new();
    let mut referenced: HashSet<String> = HashSet::new();
    let mut missing = Vec::new();

    for book in store.list_books()? {
        books += 1;
        for entry in fs::read_dir(book.notes_dir())? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".json") {
                continue;
            }
            match load_note_file(&entry.path()) {
                Ok(note) => {
                    notes += 1;
                    for hash in referenced_assets(&note) {
                        if !hash.is_empty() {
                            if crate::assets::resolve(vault, &hash)?.is_none() {
                                missing.push(hash.clone());
                            }
                            referenced.insert(hash);
                        }
                    }
                }
                Err(error) => {
                    corrupt.push(format!("{}: {error}", entry.path().display()));
                    let target = vault
                        .quarantine_dir()
                        .join(format!("{}-{name}", uuid_stamp()));
                    fs::rename(entry.path(), &target)?;
                    quarantined.push(target.to_string_lossy().to_string());
                }
            }
        }
    }

    let orphan_assets = crate::assets::list_assets(vault)?
        .into_iter()
        .filter(|(hash, _)| !referenced.contains(hash))
        .map(|(hash, _)| hash)
        .collect();

    let mut reindexed = 0;
    if reindex {
        let mut index = Index::open(&vault.index_path())?;
        reindexed = index.rebuild(store)?;
    }

    let pending_ops = fs::read_dir(vault.folio_dir().join("ops"))
        .map(|entries| entries.filter_map(Result::ok).count())
        .unwrap_or(0);

    Ok(DoctorReport {
        books,
        notes,
        corrupt,
        quarantined,
        orphan_assets,
        missing_assets: missing,
        reindexed,
        pending_ops,
    })
}

fn uuid_stamp() -> String {
    folio_core::new_id()
}

/// Escribe un snapshot de recuperación (journal de autosave).
pub fn write_recovery(vault: &Vault, note: &Note) -> Result<PathBuf, FolioError> {
    let dir = vault.folio_dir().join("recovery");
    ensure_dir(&dir)?;
    let path = dir.join(format!("{}.json", note.id));
    write_atomic(&path, serde_json::to_vec_pretty(note)?.as_slice())?;
    Ok(path)
}

pub fn read_recovery(vault: &Vault, note_id: &str) -> Result<Option<Note>, FolioError> {
    let path = vault
        .folio_dir()
        .join("recovery")
        .join(format!("{note_id}.json"));
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(load_note_file(&path)?))
}

pub fn clear_recovery(vault: &Vault, note_id: &str) -> Result<(), FolioError> {
    let path = vault
        .folio_dir()
        .join("recovery")
        .join(format!("{note_id}.json"));
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::Store;
    use folio_core::FixedClock;

    #[test]
    fn quarantines_corrupt_and_reindexes() {
        let dir = tempfile::tempdir().unwrap();
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let store = Store::init(dir.path(), "v", &clock).unwrap();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        let note = store.create_note(&book, "x", &clock).unwrap();
        let path = store.notes(&book).unwrap()[0].path.clone();
        fs::write(&path, b"{ roto").unwrap();
        let report = repair(&store, true).unwrap();
        assert_eq!(report.corrupt.len(), 1);
        assert_eq!(report.quarantined.len(), 1);
        assert!(!path.exists());
        assert!(
            dir.path()
                .join(".folio/quarantine")
                .read_dir()
                .unwrap()
                .next()
                .is_some()
        );
        assert_eq!(report.reindexed, 0);
        let _ = note;
    }

    #[test]
    fn reports_orphan_and_missing_assets() {
        let dir = tempfile::tempdir().unwrap();
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let store = Store::init(dir.path(), "v", &clock).unwrap();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        let mut note = store.create_note(&book, "x", &clock).unwrap();
        let missing = "a".repeat(64);
        note.blocks.push(folio_core::Block::new(
            "image",
            folio_core::model::BlockContent::Image(folio_core::model::ImageContent {
                asset: missing.clone(),
                ..Default::default()
            }),
        ));
        store.save_note(&book, &mut note, &clock).unwrap();
        crate::assets::import_bytes(&store.vault, b"huerfano", "txt").unwrap();
        let report = repair(&store, false).unwrap();
        assert_eq!(report.orphan_assets.len(), 1);
        assert_eq!(report.missing_assets, vec![missing]);
    }
}

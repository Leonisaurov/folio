use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use folio_core::{Clock, FolioError};
use serde::{Deserialize, Serialize};

use crate::atomic::{ensure_dir, link_or_copy, walk_files, write_atomic};
use crate::vault::Vault;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub schema: u32,
    pub created_at: String,
    pub vault_id: String,
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct BackupEntry {
    pub dir: PathBuf,
    pub manifest: BackupManifest,
}

/// Snapshot por hardlinks. Excluye el propio directorio de backups, el índice
/// y la papelera: los datos de usuario viven en `books/`, `assets/`, `plugins/`
/// y `vault.toml`.
pub fn create(vault: &Vault, clock: &dyn Clock) -> Result<PathBuf, FolioError> {
    let stamp: String = clock
        .now_rfc3339()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let target = vault.backups_dir().join(stamp);
    ensure_dir(&target)?;
    let mut files = BTreeMap::new();
    for path in walk_files(&vault.root)? {
        let relative = path
            .strip_prefix(&vault.root)
            .map_err(|_| FolioError::Invalid("archivo fuera de la bóveda".into()))?;
        let text = relative.to_string_lossy();
        // El índice y la papelera no entran; `vault.toml` y `plugins/` sí.
        if text.starts_with(".folio/") || text.starts_with(".git/") {
            continue;
        }
        let dest = target.join(relative);
        link_or_copy(&path, &dest)?;
        let hash = crate::assets::hash_file(&path)?;
        files.insert(text.to_string(), hash);
    }
    let manifest = BackupManifest {
        schema: 1,
        created_at: clock.now_rfc3339(),
        vault_id: vault.meta.id.clone(),
        files,
    };
    write_atomic(
        &target.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?.as_slice(),
    )?;
    prune(vault, 7)?;
    Ok(target)
}

pub fn list(vault: &Vault) -> Result<Vec<BackupEntry>, FolioError> {
    let mut entries = Vec::new();
    ensure_dir(&vault.backups_dir())?;
    for entry in fs::read_dir(vault.backups_dir())? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let manifest: BackupManifest = serde_json::from_slice(&fs::read(&manifest_path)?)?;
        entries.push(BackupEntry {
            dir: entry.path(),
            manifest,
        });
    }
    entries.sort_by(|a, b| a.manifest.created_at.cmp(&b.manifest.created_at));
    Ok(entries)
}

/// Comprueba hashes del manifiesto; detecta corrupción o archivos ausentes.
pub fn verify(backup_dir: &Path) -> Result<BackupManifest, FolioError> {
    let manifest: BackupManifest =
        serde_json::from_slice(&fs::read(backup_dir.join("manifest.json"))?)?;
    for (relative, expected) in &manifest.files {
        let path = backup_dir.join(relative);
        if !path.exists() {
            return Err(FolioError::Corrupt(format!("falta en backup: {relative}")));
        }
        let actual = crate::assets::hash_file(&path)?;
        if &actual != expected {
            return Err(FolioError::Corrupt(format!(
                "hash distinto en backup: {relative}"
            )));
        }
    }
    Ok(manifest)
}

/// Restaura sobre una ruta nueva (nunca in-place).
pub fn restore(backup_dir: &Path, target: &Path) -> Result<usize, FolioError> {
    if target.exists() {
        return Err(FolioError::Conflict(format!(
            "el destino ya existe: {}",
            target.display()
        )));
    }
    verify(backup_dir)?;
    let mut restored = 0;
    for path in walk_files(backup_dir)? {
        let relative = path
            .strip_prefix(backup_dir)
            .map_err(|_| FolioError::Invalid("archivo fuera del backup".into()))?;
        if relative == Path::new("manifest.json") {
            continue;
        }
        let dest = target.join(relative);
        if let Some(parent) = dest.parent() {
            ensure_dir(parent)?;
        }
        fs::copy(&path, &dest)?;
        restored += 1;
    }
    Ok(restored)
}

fn prune(vault: &Vault, keep: usize) -> Result<(), FolioError> {
    let entries = list(vault)?;
    if entries.len() > keep {
        for entry in entries.iter().take(entries.len() - keep) {
            fs::remove_dir_all(&entry.dir)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::Store;
    use folio_core::FixedClock;

    #[test]
    fn backup_verify_and_restore_identical() {
        let dir = tempfile::tempdir().unwrap();
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let store = Store::init(dir.path(), "v", &clock).unwrap();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        let mut note = store.create_note(&book, "x", &clock).unwrap();
        note.blocks.push(folio_core::Block::text_plain("hola"));
        store.save_note(&book, &mut note, &clock).unwrap();
        let backup = create(&store.vault, &clock).unwrap();
        verify(&backup).unwrap();
        // Modificar el original no altera el backup (escritura atómica).
        note.title = "cambiado".into();
        store.save_note(&book, &mut note, &clock).unwrap();
        let restored_dir = tempfile::tempdir().unwrap();
        let target = restored_dir.path().join("restored");
        restore(&backup, &target).unwrap();
        let restored_store = Store::open(&target).unwrap();
        let restored_note = restored_store
            .load_note(&restored_store.book(&book.meta.id).unwrap(), &note.id)
            .unwrap();
        assert_eq!(restored_note.title, "x");
        assert_eq!(restored_note.blocks[0].plain_text(), "hola");
    }

    #[test]
    fn verify_detects_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let store = Store::init(dir.path(), "v", &clock).unwrap();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        store.create_note(&book, "x", &clock).unwrap();
        let backup = create(&store.vault, &clock).unwrap();
        let note = store.notes(&book).unwrap()[0].path.clone();
        let relative = note.strip_prefix(&store.vault.root).unwrap();
        // En un hardlink una escritura in-place afectaría al backup; el test
        // corrompe el backup directamente para comprobar la verificación.
        let target = backup.join(relative);
        fs::write(&target, b"corrupto").unwrap();
        assert!(verify(&backup).is_err());
    }
}

use std::fs;
use std::path::{Path, PathBuf};

use folio_core::{Clock, FolioError, Note};
use jiff::Timestamp;

use crate::atomic::{ensure_dir, write_atomic};
use crate::vault::Vault;

const MIN_INTERVAL_SECONDS: i64 = 300;

#[derive(Debug, Clone)]
pub struct VersionEntry {
    pub timestamp: Timestamp,
    pub path: PathBuf,
}

pub fn version_dir(vault: &Vault, note_id: &str) -> PathBuf {
    vault.versions_dir().join(note_id)
}

/// Crea un snapshot comprimido si pasaron 5 minutos o si `force`.
pub fn maybe_snapshot(
    vault: &Vault,
    note: &Note,
    clock: &dyn Clock,
    force: bool,
) -> Result<Option<PathBuf>, FolioError> {
    let dir = version_dir(vault, &note.id);
    ensure_dir(&dir)?;
    if !force {
        let latest = list(vault, &note.id)?.into_iter().next_back();
        if let Some(latest) = latest
            && clock.now().as_second() - latest.timestamp.as_second() < MIN_INTERVAL_SECONDS
        {
            return Ok(None);
        }
    }
    let seconds = clock.now().as_second();
    let path = dir.join(format!("{seconds}.json.zst"));
    if path.exists() {
        return Ok(Some(path));
    }
    let json = serde_json::to_vec_pretty(note)?;
    let compressed = zstd::encode_all(json.as_slice(), 3).map_err(std::io::Error::other)?;
    write_atomic(&path, &compressed)?;
    prune(vault, &note.id, 50, 30, clock)?;
    Ok(Some(path))
}

pub fn list(vault: &Vault, note_id: &str) -> Result<Vec<VersionEntry>, FolioError> {
    let dir = version_dir(vault, note_id);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut versions = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(seconds) = name.strip_suffix(".json.zst") else {
            continue;
        };
        let Ok(seconds) = seconds.parse::<i64>() else {
            continue;
        };
        let Ok(timestamp) = Timestamp::from_second(seconds) else {
            continue;
        };
        versions.push(VersionEntry {
            timestamp,
            path: entry.path(),
        });
    }
    versions.sort_by_key(|entry| entry.timestamp);
    Ok(versions)
}

pub fn read(path: &Path) -> Result<Note, FolioError> {
    let compressed = fs::read(path)?;
    let json = zstd::decode_all(compressed.as_slice()).map_err(std::io::Error::other)?;
    Note::from_json(&json)
}

/// Restaurar crea una versión nueva: nunca borra la actual.
pub fn restore(
    vault: &Vault,
    note_id: &str,
    path: &Path,
    clock: &dyn Clock,
) -> Result<Note, FolioError> {
    let _ = vault;
    let mut note = read(path)?;
    note.id = note_id.to_string();
    note.updated_at = clock.now_rfc3339();
    Ok(note)
}

fn prune(
    vault: &Vault,
    note_id: &str,
    max: usize,
    days: u32,
    clock: &dyn Clock,
) -> Result<(), FolioError> {
    let versions = list(vault, note_id)?;
    let cutoff = clock.now().as_second() - i64::from(days) * 86_400;
    let mut keep_from = versions.len().saturating_sub(max);
    while keep_from < versions.len()
        && versions[keep_from].timestamp.as_second() < cutoff
        && versions.len() - keep_from > 1
    {
        keep_from += 1;
    }
    for entry in versions.iter().take(keep_from) {
        if entry.timestamp.as_second() < cutoff {
            fs::remove_file(&entry.path)?;
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
    fn snapshot_throttle_and_restore() {
        let dir = tempfile::tempdir().unwrap();
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let store = Store::init(dir.path(), "v", &clock).unwrap();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        let mut note = store.create_note(&book, "x", &clock).unwrap();
        note.title = "versión 1".into();
        store.save_note(&book, &mut note, &clock).unwrap();
        let versions = list(&store.vault, &note.id).unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions.len(), 1);
        // Dentro de 5 minutos no se crea otra.
        clock.advance_ms(1000);
        assert!(
            maybe_snapshot(&store.vault, &note, &clock, false)
                .unwrap()
                .is_none()
        );
        clock.advance_ms(5 * 60 * 1000);
        note.title = "versión 2".into();
        maybe_snapshot(&store.vault, &note, &clock, true).unwrap();
        assert_eq!(list(&store.vault, &note.id).unwrap().len(), 2);
        let restored = restore(&store.vault, &note.id, &versions[0].path, &clock).unwrap();
        assert_eq!(restored.title, "versión 1");
    }
}

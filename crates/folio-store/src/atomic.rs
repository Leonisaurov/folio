use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use folio_core::FolioError;
use uuid::Uuid;

/// Escritura atómica: temporal en el mismo directorio, fsync, rename y fsync
/// del directorio. Si el proceso muere, el archivo final nunca queda a medias.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), FolioError> {
    let dir = path
        .parent()
        .ok_or_else(|| FolioError::Invalid(format!("ruta sin directorio: {}", path.display())))?;
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".tmp-{}", Uuid::now_v7()));
    {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
    }
    match fs::rename(&tmp, path) {
        Ok(()) => {}
        Err(error) => {
            let _ = fs::remove_file(&tmp);
            return Err(error.into());
        }
    }
    fsync_dir(dir)?;
    Ok(())
}

pub fn fsync_dir(dir: &Path) -> io::Result<()> {
    let file = File::open(dir)?;
    file.sync_all()
}

/// Copia un archivo de forma atómica (útil para assets y backups).
pub fn copy_atomic(from: &Path, to: &Path) -> Result<(), FolioError> {
    let dir = to
        .parent()
        .ok_or_else(|| FolioError::Invalid(format!("ruta sin directorio: {}", to.display())))?;
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".tmp-{}", Uuid::now_v7()));
    {
        let mut src = File::open(from)?;
        let mut dst = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        io::copy(&mut src, &mut dst)?;
        dst.flush()?;
        dst.sync_all()?;
    }
    fs::rename(&tmp, to)?;
    fsync_dir(dir)?;
    Ok(())
}

/// Crea un hardlink o copia si el sistema de archivos no lo permite.
pub fn link_or_copy(from: &Path, to: &Path) -> Result<(), FolioError> {
    if to.exists() {
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::hard_link(from, to) {
        Ok(()) => Ok(()),
        Err(_) => copy_atomic(from, to),
    }
}

pub fn ensure_dir(path: &Path) -> Result<(), FolioError> {
    fs::create_dir_all(path)?;
    Ok(())
}

#[cfg(unix)]
pub fn set_mode(path: &Path, mode: u32) -> Result<(), FolioError> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn set_mode(_path: &Path, _mode: u32) -> Result<(), FolioError> {
    Ok(())
}

/// Recorre archivos de un árbol (sin seguir symlinks).
pub fn walk_files(root: &Path) -> Result<Vec<PathBuf>, FolioError> {
    let mut files = Vec::new();
    fn walk(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                walk(&path, files)?;
            } else if file_type.is_file() {
                files.push(path);
            }
        }
        Ok(())
    }
    walk(root, &mut files)?;
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_roundtrip_and_no_temp_left() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.json");
        write_atomic(&path, b"{\"a\":1}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}");
        write_atomic(&path, b"{\"a\":2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":2}");
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn interruption_leaves_previous_file_intact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.json");
        write_atomic(&path, b"original").unwrap();
        // Simula un temporal huérfano de un proceso muerto.
        fs::write(dir.path().join(".tmp-orphan"), b"parcial").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }
}

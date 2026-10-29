use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use folio_core::FolioError;

/// Lock de instancia única en `.folio/lock`.
pub struct VaultLock {
    path: PathBuf,
    held: bool,
}

impl VaultLock {
    /// `Ok(None)` significa que otra instancia viva tiene el lock.
    pub fn acquire(vault: &crate::vault::Vault) -> Result<Option<Self>, FolioError> {
        let path = vault.folio_dir().join("lock");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        for _ in 0..2 {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    writeln!(file, "{}", std::process::id())?;
                    file.sync_all()?;
                    return Ok(Some(Self { path, held: true }));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if !pid_alive(&path) {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    return Ok(None);
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(None)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn pid_alive(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(pid) = text.trim().parse::<u32>() else {
        return false;
    };
    Path::new(&format!("/proc/{pid}")).exists()
}

impl Drop for VaultLock {
    fn drop(&mut self) {
        if self.held {
            let _ = fs::remove_file(&self.path);
            self.held = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Vault;
    use folio_core::SystemClock;

    #[test]
    fn lock_excludes_second_instance_and_releases() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::init(dir.path(), "v", &SystemClock).unwrap();
        let first = VaultLock::acquire(&vault).unwrap();
        assert!(first.is_some());
        assert!(VaultLock::acquire(&vault).unwrap().is_none());
        drop(first);
        assert!(VaultLock::acquire(&vault).unwrap().is_some());
    }

    #[test]
    fn stale_lock_is_reclaimed() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::init(dir.path(), "v", &SystemClock).unwrap();
        fs::write(vault.folio_dir().join("lock"), "999999\n").unwrap();
        assert!(VaultLock::acquire(&vault).unwrap().is_some());
    }
}

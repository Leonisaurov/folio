use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use folio_core::FolioError;

use crate::atomic::{copy_atomic, ensure_dir, write_atomic};
use crate::vault::Vault;

pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

pub fn hash_file(path: &Path) -> Result<String, FolioError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn sanitized_extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ext.chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .take(10)
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|ext| !ext.is_empty())
        .unwrap_or_else(|| "bin".to_string())
}

fn relative_path(hash: &str, ext: &str) -> PathBuf {
    PathBuf::from(&hash[0..2])
        .join(&hash[2..4])
        .join(format!("{hash}.{ext}"))
}

/// Importa un archivo, deduplicando por BLAKE3.
pub fn import_file(
    vault: &Vault,
    source: &Path,
    cipher: Option<&crate::crypto::Cipher>,
) -> Result<String, FolioError> {
    let hash = hash_file(source)?;
    let ext = sanitized_extension(source);
    let dest = vault.assets_dir().join(relative_path(&hash, &ext));
    if !dest.exists() {
        if let Some(cipher) = cipher {
            let plain = fs::read(source)?;
            write_atomic(&dest, &cipher.encrypt(&plain)?)?;
        } else {
            copy_atomic(source, &dest)?;
        }
    }
    Ok(hash)
}

pub fn import_bytes(
    vault: &Vault,
    bytes: &[u8],
    extension: &str,
    cipher: Option<&crate::crypto::Cipher>,
) -> Result<String, FolioError> {
    let hash = hash_bytes(bytes);
    let ext: String = extension
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(10)
        .collect::<String>()
        .to_ascii_lowercase();
    let ext = if ext.is_empty() { "bin".into() } else { ext };
    let dest = vault.assets_dir().join(relative_path(&hash, &ext));
    if !dest.exists() {
        match cipher {
            Some(cipher) => write_atomic(&dest, &cipher.encrypt(bytes)?)?,
            None => write_atomic(&dest, bytes)?,
        }
    }
    Ok(hash)
}

/// Localiza un asset por hash probando las extensiones presentes.
pub fn resolve(vault: &Vault, hash: &str) -> Result<Option<PathBuf>, FolioError> {
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(FolioError::Invalid(format!(
            "hash de asset inválido: {hash}"
        )));
    }
    let dir = vault.assets_dir().join(&hash[0..2]).join(&hash[2..4]);
    if !dir.exists() {
        return Ok(None);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(hash) {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

pub fn read_bytes(
    vault: &Vault,
    hash: &str,
    cipher: Option<&crate::crypto::Cipher>,
) -> Result<Vec<u8>, FolioError> {
    let path =
        resolve(vault, hash)?.ok_or_else(|| FolioError::NotFound(format!("asset {hash}")))?;
    let bytes = fs::read(path)?;
    match cipher {
        Some(cipher) => cipher.decrypt(&bytes),
        None => Ok(bytes),
    }
}

pub fn list_assets(vault: &Vault) -> Result<Vec<(String, PathBuf)>, FolioError> {
    ensure_dir(&vault.assets_dir())?;
    let mut assets = Vec::new();
    for prefix in fs::read_dir(vault.assets_dir())? {
        let prefix = prefix?;
        if !prefix.file_type()?.is_dir() {
            continue;
        }
        for sub in fs::read_dir(prefix.path())? {
            let sub = sub?;
            if !sub.file_type()?.is_dir() {
                continue;
            }
            for file in fs::read_dir(sub.path())? {
                let file = file?;
                let name = file.file_name().to_string_lossy().to_string();
                if let Some(hash) = name.split('.').next()
                    && hash.len() == 64
                {
                    assets.push((hash.to_string(), file.path()));
                }
            }
        }
    }
    assets.sort();
    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Vault;
    use folio_core::SystemClock;

    #[test]
    fn deduplicates_assets() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::init(dir.path(), "v", &SystemClock).unwrap();
        let a = import_bytes(&vault, b"mismo contenido", "txt", None).unwrap();
        let b = import_bytes(&vault, b"mismo contenido", "txt", None).unwrap();
        assert_eq!(a, b);
        assert_eq!(list_assets(&vault).unwrap().len(), 1);
        assert_eq!(read_bytes(&vault, &a, None).unwrap(), b"mismo contenido");
        assert_eq!(
            resolve(&vault, &a).unwrap().unwrap().extension().unwrap(),
            "txt"
        );
    }

    #[test]
    fn rejects_path_traversal_hash() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::init(dir.path(), "v", &SystemClock).unwrap();
        assert!(resolve(&vault, "../../etc/passwd").is_err());
    }
}

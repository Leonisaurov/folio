use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use folio_core::{Clock, FolioError, new_id};
use serde::{Deserialize, Serialize};

use crate::atomic::{ensure_dir, set_mode, write_atomic};

pub const VAULT_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VaultMeta {
    #[serde(default = "default_schema")]
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub created_at: String,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub daily: DailyVaultConfig,
    #[serde(default)]
    pub encryption: EncryptionConfig,
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

fn default_schema() -> u32 {
    VAULT_SCHEMA
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageConfig {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_fsync")]
    pub fsync: String,
    #[serde(default = "default_assets")]
    pub assets: String,
    #[serde(default = "default_max_asset_mb")]
    pub max_asset_mb: u64,
}

fn default_format() -> String {
    "json".into()
}
fn default_fsync() -> String {
    "always".into()
}
fn default_assets() -> String {
    "copy".into()
}
fn default_max_asset_mb() -> u64 {
    2048
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            format: default_format(),
            fsync: default_fsync(),
            assets: default_assets(),
            max_asset_mb: default_max_asset_mb(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyVaultConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for DailyVaultConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EncryptionConfig {
    #[serde(default = "default_encryption")]
    pub mode: String,
}

fn default_encryption() -> String {
    "none".into()
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            mode: default_encryption(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Vault {
    pub root: PathBuf,
    pub meta: VaultMeta,
}

impl Vault {
    /// Crea (o abre si ya existe) una bóveda.
    pub fn init(root: &Path, name: &str, clock: &dyn Clock) -> Result<Self, FolioError> {
        ensure_dir(root)?;
        let meta_path = root.join("vault.toml");
        if meta_path.exists() {
            return Self::open(root);
        }
        let meta = VaultMeta {
            schema: VAULT_SCHEMA,
            id: new_id(),
            name: name.to_string(),
            created_at: clock.now_rfc3339(),
            storage: StorageConfig::default(),
            daily: DailyVaultConfig::default(),
            encryption: EncryptionConfig::default(),
            extra: BTreeMap::new(),
        };
        for dir in [
            "books",
            "assets",
            "plugins",
            ".folio/trash",
            ".folio/quarantine",
            ".folio/backups",
            ".folio/versions",
            ".folio/cache/thumbs",
            ".folio/cache/waveforms",
            ".folio/plugins-data",
            ".folio/journal",
            ".folio/ops",
        ] {
            ensure_dir(&root.join(dir))?;
        }
        let text =
            toml::to_string_pretty(&meta).map_err(|error| FolioError::Toml(error.to_string()))?;
        write_atomic(&meta_path, text.as_bytes())?;
        set_mode(root, 0o700)?;
        set_mode(&meta_path, 0o600)?;
        set_mode(&root.join(".folio"), 0o700)?;
        Ok(Self {
            root: root.to_path_buf(),
            meta,
        })
    }

    pub fn open(root: &Path) -> Result<Self, FolioError> {
        let meta_path = root.join("vault.toml");
        let bytes = fs::read(&meta_path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                FolioError::NotFound(format!("bóveda en {}", root.display()))
            } else {
                error.into()
            }
        })?;
        let text = String::from_utf8_lossy(&bytes);
        let meta: VaultMeta = toml::from_str(&text)
            .map_err(|error| FolioError::Toml(format!("{}: {error}", meta_path.display())))?;
        if meta.schema > VAULT_SCHEMA {
            return Err(FolioError::Unsupported(format!(
                "vault.toml schema {} mayor que {VAULT_SCHEMA}",
                meta.schema
            )));
        }
        // Repara directorios imprescindibles sin tocar datos.
        for dir in [
            "books",
            "assets",
            "plugins",
            ".folio/trash",
            ".folio/quarantine",
            ".folio/backups",
            ".folio/versions",
            ".folio/journal",
            ".folio/ops",
        ] {
            ensure_dir(&root.join(dir))?;
        }
        Ok(Self {
            root: root.to_path_buf(),
            meta,
        })
    }

    pub fn save_meta(&self) -> Result<(), FolioError> {
        let text = toml::to_string_pretty(&self.meta)
            .map_err(|error| FolioError::Toml(error.to_string()))?;
        write_atomic(&self.root.join("vault.toml"), text.as_bytes())
    }

    pub fn books_dir(&self) -> PathBuf {
        self.root.join("books")
    }
    pub fn notes_dir_of(&self, folder: &str) -> PathBuf {
        self.root.join("books").join(folder).join("notes")
    }
    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }
    pub fn folio_dir(&self) -> PathBuf {
        self.root.join(".folio")
    }
    pub fn index_path(&self) -> PathBuf {
        self.folio_dir().join("index.sqlite")
    }
    pub fn trash_dir(&self) -> PathBuf {
        self.folio_dir().join("trash")
    }
    pub fn versions_dir(&self) -> PathBuf {
        self.folio_dir().join("versions")
    }
    pub fn backups_dir(&self) -> PathBuf {
        self.folio_dir().join("backups")
    }
    pub fn quarantine_dir(&self) -> PathBuf {
        self.folio_dir().join("quarantine")
    }
    pub fn plugins_dir(&self) -> PathBuf {
        self.root.join("plugins")
    }
    pub fn name(&self) -> &str {
        &self.meta.name
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("vault.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use folio_core::SystemClock;

    #[test]
    fn init_creates_structure_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let clock = SystemClock;
        let vault = Vault::init(dir.path(), "Prueba", &clock).unwrap();
        assert!(dir.path().join("vault.toml").exists());
        assert!(vault.books_dir().exists());
        assert!(vault.trash_dir().exists());
        let again = Vault::init(dir.path(), "Otro", &clock).unwrap();
        assert_eq!(again.meta.id, vault.meta.id);
        assert_eq!(again.meta.name, "Prueba");
    }

    #[test]
    fn unknown_toml_fields_survive() {
        let dir = tempfile::tempdir().unwrap();
        let clock = SystemClock;
        Vault::init(dir.path(), "Prueba", &clock).unwrap();
        let path = dir.path().join("vault.toml");
        let text = fs::read_to_string(&path).unwrap();
        let text = text.replacen("[storage]", "futuro = \"sí\"\n\n[storage]", 1);
        fs::write(&path, text).unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        assert!(vault.meta.extra.contains_key("futuro"));
    }
}

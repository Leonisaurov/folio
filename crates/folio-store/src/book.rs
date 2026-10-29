use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use folio_core::{Clock, FolioError, ids};
use serde::{Deserialize, Serialize};

use crate::atomic::{ensure_dir, write_atomic};
use crate::vault::Vault;

pub const BOOK_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BookMeta {
    #[serde(default = "default_schema")]
    pub schema: u32,
    pub id: String,
    pub title: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_timezone: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default = "default_sort")]
    pub sort: String,
    #[serde(default)]
    pub template: String,
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

fn default_schema() -> u32 {
    BOOK_SCHEMA
}
fn default_kind() -> String {
    "manual".into()
}
fn default_sort() -> String {
    "manual".into()
}

#[derive(Debug, Clone)]
pub struct Book {
    pub meta: BookMeta,
    pub folder: String,
    pub dir: PathBuf,
}

impl Book {
    pub fn new(
        title: impl Into<String>,
        kind: &str,
        daily_date: Option<String>,
        daily_timezone: Option<String>,
        folder: String,
        clock: &dyn Clock,
    ) -> Self {
        let now = clock.now_rfc3339();
        Self {
            meta: BookMeta {
                schema: BOOK_SCHEMA,
                id: ids::new_id(),
                title: title.into(),
                kind: kind.into(),
                daily_date,
                daily_timezone,
                created_at: now.clone(),
                updated_at: now,
                icon: if kind == "daily" {
                    "x-office-calendar-symbolic".into()
                } else {
                    "x-office-document-symbolic".into()
                },
                color: String::new(),
                pinned: false,
                archived: false,
                sort: default_sort(),
                template: String::new(),
                extra: BTreeMap::new(),
            },
            folder,
            dir: PathBuf::new(),
        }
    }

    pub fn notes_dir(&self) -> PathBuf {
        self.dir.join("notes")
    }

    pub fn create_on_disk(&mut self, vault: &Vault) -> Result<(), FolioError> {
        self.dir = vault.books_dir().join(&self.folder);
        ensure_dir(&self.dir.join("notes"))?;
        self.save()?;
        Ok(())
    }

    pub fn create_on_disk_with(
        &mut self,
        vault: &Vault,
        cipher: Option<&crate::crypto::Cipher>,
    ) -> Result<(), FolioError> {
        self.dir = vault.books_dir().join(&self.folder);
        ensure_dir(&self.dir.join("notes"))?;
        self.save_with(cipher)?;
        Ok(())
    }

    pub fn load(dir: &Path, folder: String) -> Result<Self, FolioError> {
        Self::load_with(dir, folder, None)
    }

    pub fn load_with(
        dir: &Path,
        folder: String,
        cipher: Option<&crate::crypto::Cipher>,
    ) -> Result<Self, FolioError> {
        let path = dir.join("book.toml");
        let bytes = fs::read(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                FolioError::Corrupt(format!("falta {}", path.display()))
            } else {
                error.into()
            }
        })?;
        let bytes = match cipher {
            Some(cipher) => {
                if !crate::crypto::is_encrypted(&bytes) {
                    return Err(FolioError::Corrupt(format!(
                        "book.toml sin cifrar en bóveda cifrada: {}",
                        path.display()
                    )));
                }
                cipher.decrypt(&bytes)?
            }
            None => bytes,
        };
        let text = String::from_utf8_lossy(&bytes);
        let meta: BookMeta = toml::from_str(&text)
            .map_err(|error| FolioError::Toml(format!("{}: {error}", path.display())))?;
        if meta.kind == "daily" && meta.daily_date.is_none() {
            return Err(FolioError::Corrupt(format!(
                "libro diario sin daily_date: {folder}"
            )));
        }
        Ok(Self {
            meta,
            folder,
            dir: dir.to_path_buf(),
        })
    }

    pub fn save(&self) -> Result<(), FolioError> {
        self.save_with(None)
    }

    pub fn save_with(&self, cipher: Option<&crate::crypto::Cipher>) -> Result<(), FolioError> {
        self.meta.validate()?;
        let text = toml::to_string_pretty(&self.meta)
            .map_err(|error| FolioError::Toml(error.to_string()))?;
        let bytes = match cipher {
            Some(cipher) => cipher.encrypt(text.as_bytes())?,
            None => text.into_bytes(),
        };
        write_atomic(&self.dir.join("book.toml"), &bytes)
    }

    pub fn touch(&mut self, clock: &dyn Clock) {
        self.meta.updated_at = clock.now_rfc3339();
    }
}

impl BookMeta {
    pub fn validate(&self) -> Result<(), FolioError> {
        if self.schema > BOOK_SCHEMA {
            return Err(FolioError::Unsupported(format!(
                "book.toml schema {}",
                self.schema
            )));
        }
        if self.title.trim().is_empty() {
            return Err(FolioError::Invalid("título de libro vacío".into()));
        }
        if self.kind == "daily" {
            let date = self
                .daily_date
                .as_deref()
                .ok_or_else(|| FolioError::Invalid("libro diario sin daily_date".into()))?;
            if date.len() != 10
                || date.as_bytes().get(4) != Some(&b'-')
                || date.as_bytes().get(7) != Some(&b'-')
            {
                return Err(FolioError::Invalid(format!("daily_date inválida: {date}")));
            }
        } else if self.kind != "manual" {
            return Err(FolioError::Invalid(format!(
                "kind de libro inválido: {}",
                self.kind
            )));
        }
        Ok(())
    }
}

/// Carpeta estable `fecha_slug` o `slug` para manuales.
pub fn folder_name(base: &str, id: &str) -> String {
    let safe: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = safe.trim_matches('-');
    let trimmed = if trimmed.is_empty() { "libro" } else { trimmed };
    format!("{trimmed}_{}", ids::short_slug(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use folio_core::SystemClock;

    #[test]
    fn book_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::init(dir.path(), "v", &SystemClock).unwrap();
        let mut book = Book::new(
            "29/10/2026",
            "daily",
            Some("2026-10-29".into()),
            Some("Europe/Madrid".into()),
            "2026-10-29_abc123".into(),
            &SystemClock,
        );
        book.create_on_disk(&vault).unwrap();
        let loaded = Book::load(&book.dir, book.folder.clone()).unwrap();
        assert_eq!(loaded.meta.id, book.meta.id);
        assert_eq!(loaded.meta.daily_date.as_deref(), Some("2026-10-29"));
    }

    #[test]
    fn daily_without_date_is_invalid() {
        let mut book = Book::new("x", "daily", None, None, "f".into(), &SystemClock);
        assert!(book.meta.validate().is_err());
        book.meta.daily_date = Some("2026-10-29".into());
        book.meta.validate().unwrap();
    }
}

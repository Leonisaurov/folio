use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use folio_plugin_sdk::{HostContext, Manifest, Permission};
use serde::{Deserialize, Serialize};

use crate::manifest::{self, package_hash, verify_signature};
use crate::{InstalledPlugin, PluginError};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default = "default_schema")]
    pub schema: u32,
    #[serde(default)]
    pub plugins: BTreeMap<String, InstalledPlugin>,
}

fn default_schema() -> u32 {
    1
}

impl Registry {
    pub fn load(vault_root: &Path) -> Result<Self, PluginError> {
        let path = registry_path(vault_root);
        if !path.exists() {
            return Ok(Self {
                schema: 1,
                plugins: BTreeMap::new(),
            });
        }
        let text = fs::read_to_string(path)?;
        let registry: Registry =
            toml::from_str(&text).map_err(|error| PluginError::Toml(error.to_string()))?;
        Ok(registry)
    }

    pub fn save(&self, vault_root: &Path) -> Result<(), PluginError> {
        let path = registry_path(vault_root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text =
            toml::to_string_pretty(self).map_err(|error| PluginError::Toml(error.to_string()))?;
        let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
        fs::write(&tmp, text)?;
        fs::rename(tmp, path)?;
        Ok(())
    }

    pub fn installed(&self, id: &str) -> Option<&InstalledPlugin> {
        self.plugins.get(id)
    }

    pub fn enabled(&self, id: &str) -> bool {
        self.plugins
            .get(id)
            .map(|plugin| plugin.enabled)
            .unwrap_or(false)
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), PluginError> {
        let plugin = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| PluginError::NotFound(id.to_string()))?;
        plugin.enabled = enabled;
        Ok(())
    }

    pub fn grant(&mut self, id: &str, permission: Permission) -> Result<(), PluginError> {
        let plugin = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| PluginError::NotFound(id.to_string()))?;
        if !plugin.permissions.contains(&permission) {
            plugin.permissions.push(permission);
        }
        Ok(())
    }

    pub fn revoke(&mut self, id: &str, permission: Permission) -> Result<(), PluginError> {
        let plugin = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| PluginError::NotFound(id.to_string()))?;
        plugin.permissions.retain(|granted| granted != &permission);
        Ok(())
    }

    pub fn context(&self, id: &str, memory_limit_mb: u32, timeout_ms: u32) -> HostContext {
        let permissions = self
            .plugins
            .get(id)
            .map(|plugin| plugin.permissions.clone())
            .unwrap_or_default();
        HostContext {
            permissions,
            memory_limit_mb,
            timeout_ms,
        }
    }

    /// Instala un paquete desde una carpeta con `manifest.toml` y `plugin.wasm`.
    pub fn install_from_dir(
        &mut self,
        vault_root: &Path,
        source: &Path,
        allow_unsigned: bool,
        public_key_b64: Option<&str>,
        installed_at: &str,
    ) -> Result<InstalledPlugin, PluginError> {
        let manifest_path = source.join("manifest.toml");
        let manifest_text = fs::read_to_string(&manifest_path)?;
        let manifest = manifest::parse(&manifest_text)?;
        let wasm = fs::read(source.join(&manifest.entry))?;
        let hash = package_hash(manifest_text.as_bytes(), &wasm);
        if !manifest.signature.is_empty() {
            let key = public_key_b64.ok_or_else(|| {
                PluginError::Manifest("plugin firmado pero no hay clave pública configurada".into())
            })?;
            verify_signature(key, &hash, &manifest.signature)?;
        } else if !allow_unsigned {
            return Err(PluginError::Manifest(
                "el plugin no está firmado y allow_unsigned = false".into(),
            ));
        }

        let target = vault_root.join("plugins").join(&manifest.id);
        fs::create_dir_all(&target)?;
        fs::write(target.join("manifest.toml"), &manifest_text)?;
        fs::write(target.join(&manifest.entry), &wasm)?;
        fs::write(target.join("package.blake3"), hash.as_bytes())?;

        let installed = InstalledPlugin {
            id: manifest.id.clone(),
            version: manifest.version.clone(),
            enabled: true,
            permissions: manifest.permissions.clone(),
            installed_at: installed_at.to_string(),
        };
        self.plugins.insert(manifest.id, installed.clone());
        self.save(vault_root)?;
        Ok(installed)
    }

    /// Desinstalar conserva el estado de los bloques en las notas.
    pub fn uninstall(&mut self, vault_root: &Path, id: &str) -> Result<(), PluginError> {
        if self.plugins.remove(id).is_none() {
            return Err(PluginError::NotFound(id.to_string()));
        }
        let target = vault_root.join("plugins").join(id);
        if target.exists() {
            fs::remove_dir_all(target)?;
        }
        self.save(vault_root)?;
        Ok(())
    }

    pub fn package_dir(vault_root: &Path, id: &str) -> PathBuf {
        vault_root.join("plugins").join(id)
    }

    pub fn available_manifests(&self, vault_root: &Path) -> Vec<Manifest> {
        let mut manifests = crate::builtin::manifests();
        for plugin in self.plugins.keys() {
            let path = vault_root
                .join("plugins")
                .join(plugin)
                .join("manifest.toml");
            if let Ok(manifest) = manifest::load_file(&path)
                && !manifests.iter().any(|existing| existing.id == manifest.id)
            {
                manifests.push(manifest);
            }
        }
        manifests
    }
}

pub fn registry_path(vault_root: &Path) -> PathBuf {
    vault_root.join("plugins").join("registry.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    fn package(dir: &Path, signed: bool) -> (SigningKey, String) {
        let key = SigningKey::from_bytes(&[3u8; 32]);
        let wasm = b"\0asm\x01\0\0\0";
        fs::write(dir.join("plugin.wasm"), wasm).unwrap();
        let mut text = r#"
id = "io.github.acme.hello"
name = "Hello"
version = "1.0.0"
api = 1
entry = "plugin.wasm"
permissions = ["write_own_block"]
"#
        .to_string();
        let hash = package_hash(text.as_bytes(), wasm);
        if signed {
            let engine = base64::engine::general_purpose::STANDARD;
            let signature = key.sign(hash.as_bytes());
            text.push_str(&format!(
                "\nsignature = \"{}\"\n",
                engine.encode(signature.to_bytes())
            ));
        }
        fs::write(dir.join("manifest.toml"), text).unwrap();
        let engine = base64::engine::general_purpose::STANDARD;
        let public = engine.encode(key.verifying_key().to_bytes());
        (key, public)
    }

    #[test]
    fn install_signed_package_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault");
        fs::create_dir_all(&vault).unwrap();
        let source = tempfile::tempdir().unwrap();
        let (_key, public) = package(source.path(), true);
        let mut registry = Registry::load(&vault).unwrap();
        let installed = registry
            .install_from_dir(
                &vault,
                source.path(),
                false,
                Some(&public),
                "2026-10-29T09:00:00Z",
            )
            .unwrap();
        assert_eq!(installed.id, "io.github.acme.hello");
        assert!(
            vault
                .join("plugins/io.github.acme.hello/plugin.wasm")
                .exists()
        );
        let reloaded = Registry::load(&vault).unwrap();
        assert!(reloaded.enabled("io.github.acme.hello"));
        assert_eq!(
            reloaded
                .context("io.github.acme.hello", 64, 2000)
                .permissions,
            vec![Permission::WriteOwnBlock]
        );
    }

    #[test]
    fn unsigned_is_rejected_unless_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault");
        fs::create_dir_all(&vault).unwrap();
        let source = tempfile::tempdir().unwrap();
        package(source.path(), false);
        let mut registry = Registry::load(&vault).unwrap();
        assert!(
            registry
                .install_from_dir(&vault, source.path(), false, None, "now")
                .is_err()
        );
        assert!(
            registry
                .install_from_dir(&vault, source.path(), true, None, "now")
                .is_ok()
        );
    }

    #[test]
    fn revoke_and_uninstall() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault");
        fs::create_dir_all(&vault).unwrap();
        let source = tempfile::tempdir().unwrap();
        package(source.path(), false);
        let mut registry = Registry::load(&vault).unwrap();
        registry
            .install_from_dir(&vault, source.path(), true, None, "now")
            .unwrap();
        registry
            .grant("io.github.acme.hello", Permission::Network)
            .unwrap();
        assert!(
            registry
                .context("io.github.acme.hello", 64, 2000)
                .allows(Permission::Network)
        );
        registry
            .revoke("io.github.acme.hello", Permission::Network)
            .unwrap();
        assert!(
            !registry
                .context("io.github.acme.hello", 64, 2000)
                .allows(Permission::Network)
        );
        registry.uninstall(&vault, "io.github.acme.hello").unwrap();
        assert!(registry.installed("io.github.acme.hello").is_none());
        assert!(!vault.join("plugins/io.github.acme.hello").exists());
    }
}

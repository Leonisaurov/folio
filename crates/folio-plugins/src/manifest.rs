use std::path::Path;

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use folio_plugin_sdk::{BlockDeclaration, CommandDeclaration, Manifest, Permission};
use serde::Deserialize;

use crate::PluginError;

pub const API_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
struct RawManifest {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_api")]
    api: u32,
    #[serde(default)]
    entry: String,
    #[serde(default)]
    permissions: Vec<Permission>,
    #[serde(default)]
    blocks: Vec<BlockDeclaration>,
    #[serde(default)]
    commands: Vec<CommandDeclaration>,
    #[serde(default)]
    signature: String,
}

fn default_api() -> u32 {
    API_VERSION
}

impl From<RawManifest> for Manifest {
    fn from(raw: RawManifest) -> Self {
        Self {
            id: raw.id,
            name: raw.name,
            version: raw.version,
            description: raw.description,
            api: raw.api,
            entry: if raw.entry.is_empty() {
                "plugin.wasm".into()
            } else {
                raw.entry
            },
            permissions: raw.permissions,
            blocks: raw.blocks,
            commands: raw.commands,
            signature: raw.signature,
        }
    }
}

pub fn parse(text: &str) -> Result<Manifest, PluginError> {
    let raw: RawManifest =
        toml::from_str(text).map_err(|error| PluginError::Manifest(error.to_string()))?;
    let manifest: Manifest = raw.into();
    validate(&manifest)?;
    Ok(manifest)
}

pub fn load_file(path: &Path) -> Result<Manifest, PluginError> {
    let text = std::fs::read_to_string(path)?;
    parse(&text)
}

pub fn validate(manifest: &Manifest) -> Result<(), PluginError> {
    if manifest.id.len() < 3
        || !manifest.id.contains('.')
        || !manifest
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(PluginError::Manifest(format!(
            "id inverso inválido: {}",
            manifest.id
        )));
    }
    if manifest.name.trim().is_empty() {
        return Err(PluginError::Manifest("nombre vacío".into()));
    }
    let parts: Vec<&str> = manifest.version.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|part| part.parse::<u32>().is_err()) {
        return Err(PluginError::Manifest(format!(
            "versión no semver: {}",
            manifest.version
        )));
    }
    if manifest.api != API_VERSION {
        return Err(PluginError::Manifest(format!(
            "api {} no soportada (se espera {API_VERSION}); actualiza Folio o el plugin",
            manifest.api
        )));
    }
    if !manifest.entry.ends_with(".wasm")
        || manifest.entry.contains("..")
        || manifest.entry.starts_with('/')
    {
        return Err(PluginError::Manifest(
            "entry debe ser un .wasm relativo".into(),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for permission in &manifest.permissions {
        if !seen.insert(permission) {
            return Err(PluginError::Manifest(format!(
                "permiso duplicado: {permission:?}"
            )));
        }
    }
    let mut blocks = std::collections::HashSet::new();
    for block in &manifest.blocks {
        if block.id.trim().is_empty() || !blocks.insert(block.id.clone()) {
            return Err(PluginError::Manifest(format!(
                "bloque inválido o duplicado: {}",
                block.id
            )));
        }
    }
    let mut commands = std::collections::HashSet::new();
    for command in &manifest.commands {
        if command.id.trim().is_empty() || !commands.insert(command.id.clone()) {
            return Err(PluginError::Manifest(format!(
                "comando inválido o duplicado: {}",
                command.id
            )));
        }
    }
    Ok(())
}

/// Huella estable del paquete: manifiesto sin la firma + wasm.
pub fn package_hash(manifest_bytes: &[u8], wasm: &[u8]) -> String {
    let text = String::from_utf8_lossy(manifest_bytes);
    let unsigned = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("signature"))
        .collect::<Vec<_>>()
        .join("\n");
    let unsigned = unsigned.trim_end();
    let mut hasher = blake3::Hasher::new();
    hasher.update(unsigned.as_bytes());
    hasher.update(&[0]);
    hasher.update(wasm);
    hasher.finalize().to_hex().to_string()
}

/// Verifica una firma Ed25519 en base64 sobre el hash del paquete.
pub fn verify_signature(
    public_key_b64: &str,
    package_hash: &str,
    signature_b64: &str,
) -> Result<(), PluginError> {
    let engine = base64::engine::general_purpose::STANDARD;
    let key_bytes = engine
        .decode(public_key_b64)
        .map_err(|error| PluginError::Manifest(format!("clave pública base64: {error}")))?;
    let key_bytes: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| PluginError::Manifest("la clave pública debe tener 32 bytes".into()))?;
    let key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|error| PluginError::Manifest(format!("clave pública: {error}")))?;
    let signature_bytes = engine
        .decode(signature_b64)
        .map_err(|error| PluginError::Manifest(format!("firma base64: {error}")))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|error| PluginError::Manifest(format!("firma: {error}")))?;
    key.verify(package_hash.as_bytes(), &signature)
        .map_err(|_| PluginError::Manifest("firma Ed25519 no válida".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
id = "io.github.folio.progress"
name = "Progreso"
version = "0.1.0"
description = "Barra de progreso"
api = 1
entry = "plugin.wasm"
permissions = ["write_own_block"]
[[blocks]]
id = "progress"
name = "Progreso"
default_height = 120.0
[[commands]]
id = "increment"
name = "Incrementar"
"#;

    #[test]
    fn parses_and_validates() {
        let manifest = parse(VALID).unwrap();
        assert_eq!(manifest.id, "io.github.folio.progress");
        assert_eq!(manifest.permissions, vec![Permission::WriteOwnBlock]);
        assert_eq!(manifest.blocks[0].id, "progress");
    }

    #[test]
    fn rejects_bad_api_and_entry() {
        let bad = VALID.replace("api = 1", "api = 99");
        assert!(parse(&bad).is_err());
        let bad = VALID.replace("plugin.wasm", "../evil.wasm");
        assert!(parse(&bad).is_err());
        let bad = VALID.replace("version = \"0.1.0\"", "version = \"x\"");
        assert!(parse(&bad).is_err());
    }

    #[test]
    fn rejects_duplicate_permissions() {
        let bad = VALID.replace(
            "permissions = [\"write_own_block\"]",
            "permissions = [\"write_own_block\", \"write_own_block\"]",
        );
        assert!(parse(&bad).is_err());
    }

    #[test]
    fn ed25519_signature_roundtrip() {
        use ed25519_dalek::{Signer, SigningKey};
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let hash = package_hash(VALID.as_bytes(), b"wasm");
        let signature = key.sign(hash.as_bytes());
        let engine = base64::engine::general_purpose::STANDARD;
        let key_b64 = engine.encode(key.verifying_key().to_bytes());
        let sig_b64 = engine.encode(signature.to_bytes());
        verify_signature(&key_b64, &hash, &sig_b64).unwrap();
        assert!(verify_signature(&key_b64, &package_hash(b"otro", b"wasm"), &sig_b64).is_err());
    }
}

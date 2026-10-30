//! Host de plugins de Folio.
//!
//! El MVP usa el mismo contrato (`Manifest` + `Request`/`Response` + `UiNode`)
//! para plugins de referencia construidos en Rust. El cargador WASM con ABI
//! JSON queda detrás de `PluginRuntime`; si un plugin no puede ejecutarse, se
//! muestra su marcador y nunca se pierde el estado del bloque.

pub mod builtin;
pub mod manifest;
pub mod registry;
pub mod runtime;
pub mod wasm;

use folio_plugin_sdk::Permission;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("manifiesto inválido: {0}")]
    Manifest(String),
    #[error("permiso denegado: {0}")]
    Permission(String),
    #[error("plugin no encontrado: {0}")]
    NotFound(String),
    #[error("el plugin agotó su tiempo o instrucciones")]
    FuelExhausted,
    #[error("límite de memoria del plugin superado")]
    OutOfMemory,
    #[error("el plugin falló: {0}")]
    Crash(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml: {0}")]
    Toml(String),
}

/// Estado de instalación de un plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstalledPlugin {
    pub id: String,
    pub version: String,
    pub enabled: bool,
    pub permissions: Vec<Permission>,
    pub installed_at: String,
}

impl InstalledPlugin {
    pub fn allows(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }
}

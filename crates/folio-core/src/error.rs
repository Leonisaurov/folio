use serde::{Deserialize, Serialize};

/// Código estable para que la UI y la CLI puedan actuar sin mirar el texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotFound,
    Invalid,
    Io,
    Conflict,
    Locked,
    Unsupported,
    Corrupt,
}

#[derive(Debug, thiserror::Error)]
pub enum FolioError {
    #[error("no encontrado: {0}")]
    NotFound(String),
    #[error("datos inválidos: {0}")]
    Invalid(String),
    #[error("base de datos: {0}")]
    Database(String),
    #[error("conflicto: {0}")]
    Conflict(String),
    #[error("bóveda bloqueada: {0}")]
    Locked(String),
    #[error("no soportado: {0}")]
    Unsupported(String),
    #[error("archivo corrupto: {0}")]
    Corrupt(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml: {0}")]
    Toml(String),
}

impl FolioError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Invalid(_) => ErrorCode::Invalid,
            Self::Database(_) => ErrorCode::Io,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::Locked(_) => ErrorCode::Locked,
            Self::Unsupported(_) => ErrorCode::Unsupported,
            Self::Corrupt(_) => ErrorCode::Corrupt,
            Self::Io(_) => ErrorCode::Io,
            Self::Json(_) | Self::Toml(_) => ErrorCode::Invalid,
        }
    }

    /// Acción sugerida, apta para mostrar en un toast o banner.
    pub fn action(&self) -> &'static str {
        match self.code() {
            ErrorCode::NotFound => "Actualiza la lista o comprueba la ruta.",
            ErrorCode::Invalid => "Revisa el valor indicado y vuelve a intentarlo.",
            ErrorCode::Io => "Comprueba permisos y espacio libre en disco.",
            ErrorCode::Conflict => "Recarga el contenido antes de guardar.",
            ErrorCode::Locked => "Cierra la otra instancia o usa solo lectura.",
            ErrorCode::Unsupported => "Actualiza Folio para abrir este contenido.",
            ErrorCode::Corrupt => "Ejecuta «folio doctor» para reparar o aislar el archivo.",
        }
    }
}

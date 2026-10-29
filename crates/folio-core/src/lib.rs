//! Dominio puro de Folio: modelo, identificadores, comandos y undo.
//!
//! Este crate no conoce GTK, SQLite ni el sistema de archivos.

pub mod clock;
pub mod command;
pub mod error;
pub mod ids;
pub mod model;
pub mod ordering;
pub mod text;
pub mod undo;
pub mod validate;

pub use clock::{Clock, FixedClock, SystemClock};
pub use command::NoteCommand;
pub use error::{ErrorCode, FolioError};
pub use ids::{is_uuid, new_id};
pub use model::*;
pub use undo::UndoStack;

/// Versión de la biblioteca.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

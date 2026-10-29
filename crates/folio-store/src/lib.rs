//! Almacenamiento de Folio: bóveda, escritura atómica, repositorio, índice,
//! assets, papelera, versiones, backups, watcher y doctor.

pub mod assets;
pub mod atomic;
pub mod backup;
pub mod book;
pub mod crypto;
pub mod doctor;
pub mod index;
pub mod lock;
pub mod repository;
pub mod trash;
pub mod vault;
pub mod versions;
pub mod watcher;

pub use book::{Book, BookMeta};
pub use doctor::DoctorReport;
pub use index::{Index, SearchHit};
pub use lock::VaultLock;
pub use repository::{MovePosition, NoteSummary, Store};
pub use vault::{Vault, VaultMeta};

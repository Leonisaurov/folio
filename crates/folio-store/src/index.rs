use std::path::{Path, PathBuf};

use folio_core::{FolioError, Note, model::BlockContent};
use rusqlite::{Connection, OptionalExtension, params};

use crate::book::Book;
use crate::repository::Store;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SearchHit {
    pub note_id: String,
    pub book_id: String,
    pub title: String,
    pub snippet: String,
    pub updated_at: String,
    pub rank: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexStats {
    pub books: i64,
    pub notes: i64,
    pub blocks: i64,
    pub fts_rows: i64,
}

/// Error interno del índice; en la API pública se convierte a `FolioError`.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("sqlite: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Core(#[from] FolioError),
}

impl From<IndexError> for FolioError {
    fn from(error: IndexError) -> Self {
        match error {
            IndexError::Db(error) => FolioError::Database(error.to_string()),
            IndexError::Io(error) => FolioError::Io(error),
            IndexError::Core(error) => error,
        }
    }
}

pub struct Index {
    conn: Connection,
    path: PathBuf,
}

impl Index {
    pub fn open(path: &Path) -> Result<Self, IndexError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        let index = Self {
            conn,
            path: path.to_path_buf(),
        };
        index.init_schema()?;
        Ok(index)
    }

    pub fn open_in_memory() -> Result<Self, IndexError> {
        let conn = Connection::open_in_memory()?;
        let index = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        index.init_schema()?;
        Ok(index)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn init_schema(&self) -> Result<(), IndexError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS books (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                kind TEXT NOT NULL,
                daily_date TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0,
                archived INTEGER NOT NULL DEFAULT 0,
                sort TEXT NOT NULL DEFAULT 'manual',
                icon TEXT,
                color TEXT,
                mtime_ns INTEGER NOT NULL DEFAULT 0,
                size INTEGER NOT NULL DEFAULT 0
             );
             CREATE UNIQUE INDEX IF NOT EXISTS books_daily ON books(daily_date) WHERE kind = 'daily';
             CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                path TEXT NOT NULL UNIQUE,
                order_key TEXT NOT NULL,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                word_count INTEGER NOT NULL DEFAULT 0,
                block_count INTEGER NOT NULL DEFAULT 0,
                has_media INTEGER NOT NULL DEFAULT 0,
                pinned INTEGER NOT NULL DEFAULT 0,
                archived INTEGER NOT NULL DEFAULT 0,
                mtime_ns INTEGER NOT NULL DEFAULT 0,
                size INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS notes_book_order ON notes(book_id, order_key);
             CREATE INDEX IF NOT EXISTS notes_updated ON notes(updated_at DESC);
             CREATE TABLE IF NOT EXISTS note_blocks (
                note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
                block_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                text TEXT,
                asset TEXT,
                plugin TEXT,
                PRIMARY KEY (note_id, block_id)
             );
             CREATE TABLE IF NOT EXISTS assets (
                hash TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                size INTEGER NOT NULL,
                mime TEXT,
                refcount INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS note_assets (
                note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
                hash TEXT NOT NULL,
                PRIMARY KEY (note_id, hash)
             );
             CREATE TABLE IF NOT EXISTS plugins (
                id TEXT PRIMARY KEY,
                version TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                permissions TEXT NOT NULL,
                installed_at TEXT NOT NULL
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
                note_id UNINDEXED,
                title,
                body,
                tags,
                tokenize = 'unicode61 remove_diacritics 2'
             );",
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO meta(key, value) VALUES('schema', '1')",
            [],
        )?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), IndexError> {
        self.conn.execute_batch(
            "DELETE FROM notes_fts;
             DELETE FROM note_assets;
             DELETE FROM note_blocks;
             DELETE FROM notes;
             DELETE FROM books;",
        )?;
        Ok(())
    }

    pub fn index_book(&mut self, book: &Book) -> Result<(), IndexError> {
        self.conn.execute(
            "INSERT INTO books(id, path, title, kind, daily_date, created_at, updated_at, pinned, archived, sort, icon, color)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
               path=excluded.path, title=excluded.title, kind=excluded.kind,
               daily_date=excluded.daily_date, created_at=excluded.created_at,
               updated_at=excluded.updated_at, pinned=excluded.pinned,
               archived=excluded.archived, sort=excluded.sort,
               icon=excluded.icon, color=excluded.color",
            params![
                book.meta.id,
                book.dir.to_string_lossy(),
                book.meta.title,
                book.meta.kind,
                book.meta.daily_date,
                book.meta.created_at,
                book.meta.updated_at,
                book.meta.pinned as i64,
                book.meta.archived as i64,
                book.meta.sort,
                book.meta.icon,
                book.meta.color,
            ],
        )?;
        Ok(())
    }

    pub fn index_note(
        &mut self,
        book: &Book,
        note: &Note,
        path: &Path,
        order_key: &str,
    ) -> Result<(), IndexError> {
        self.index_book(book)?;
        let meta = std::fs::metadata(path)?;
        self.conn.execute(
            "INSERT INTO notes(id, book_id, path, order_key, title, created_at, updated_at,
                word_count, block_count, has_media, pinned, archived, mtime_ns, size)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 0, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                book_id=excluded.book_id, path=excluded.path, order_key=excluded.order_key,
                title=excluded.title, updated_at=excluded.updated_at,
                word_count=excluded.word_count, block_count=excluded.block_count,
                has_media=excluded.has_media, mtime_ns=excluded.mtime_ns, size=excluded.size",
            params![
                note.id,
                book.meta.id,
                path.to_string_lossy(),
                order_key,
                note.display_title(),
                note.created_at,
                note.updated_at,
                note.word_count() as i64,
                note.blocks.len() as i64,
                note.has_media() as i64,
                meta.modified()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as i64)
                    .unwrap_or(0),
                meta.len() as i64,
            ],
        )?;
        self.conn.execute(
            "DELETE FROM note_blocks WHERE note_id = ?1",
            params![note.id],
        )?;
        self.conn.execute(
            "DELETE FROM note_assets WHERE note_id = ?1",
            params![note.id],
        )?;
        for block in &note.blocks {
            let (asset, plugin) = match &block.content {
                BlockContent::Image(image) => (Some(image.asset.clone()), None),
                BlockContent::File(file) => (Some(file.asset.clone()), None),
                BlockContent::Audio(audio) => (Some(audio.asset.clone()), None),
                BlockContent::Video(video) => (Some(video.asset.clone()), None),
                BlockContent::Plugin(plugin) => (None, Some(plugin.plugin.clone())),
                _ => (None, None),
            };
            self.conn.execute(
                "INSERT OR REPLACE INTO note_blocks(note_id, block_id, kind, text, asset, plugin)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    note.id,
                    block.id,
                    block.kind,
                    block.plain_text(),
                    asset,
                    plugin
                ],
            )?;
            if let Some(asset) = asset {
                self.conn.execute(
                    "INSERT OR IGNORE INTO note_assets(note_id, hash) VALUES(?1, ?2)",
                    params![note.id, asset],
                )?;
            }
        }
        // `note_id` no está indexado en FTS5. Usar un rowid estable derivado
        // del id evita un DELETE con escaneo completo por nota (O(n²)).
        let rowid = fts_rowid(&note.id);
        self.conn
            .execute("DELETE FROM notes_fts WHERE rowid = ?1", params![rowid])?;
        self.conn.execute(
            "INSERT INTO notes_fts(rowid, note_id, title, body, tags)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                rowid,
                note.id,
                note.display_title(),
                note.plain_text(),
                note.meta.tags.join(" ")
            ],
        )?;
        Ok(())
    }

    pub fn remove_note(&mut self, note_id: &str) -> Result<(), IndexError> {
        self.conn.execute(
            "DELETE FROM notes_fts WHERE rowid = ?1",
            params![fts_rowid(note_id)],
        )?;
        self.conn
            .execute("DELETE FROM notes WHERE id = ?1", params![note_id])?;
        Ok(())
    }

    /// Reconstruye el índice completo desde los archivos.
    ///
    /// Se ejecuta en una sola transacción: sin ella, SQLite hace un commit por
    /// nota y 10k notas tardan decenas de segundos.
    pub fn rebuild(&mut self, store: &Store) -> Result<usize, IndexError> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| {
            self.clear()?;
            let mut count = 0;
            for book in store.list_books()? {
                self.index_book(&book)?;
                for (key, path) in note_files(&book)? {
                    let note = store.load_note_at(&path)?;
                    self.index_note(&book, &note, &path, &key)?;
                    count += 1;
                }
            }
            Ok::<usize, IndexError>(count)
        })();
        match result {
            Ok(count) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(count)
            }
            Err(error) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(error)
            }
        }
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, IndexError> {
        let fts_query = to_fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let mut statement = self.conn.prepare(
            "SELECT n.id, n.book_id, n.title,
                    snippet(notes_fts, 2, '<mark>', '</mark>', '…', 16),
                    n.updated_at, bm25(notes_fts)
             FROM notes_fts f
             JOIN notes n ON n.id = f.note_id
             WHERE notes_fts MATCH ?1
             ORDER BY bm25(notes_fts)
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![fts_query, limit as i64], |row| {
            Ok(SearchHit {
                note_id: row.get(0)?,
                book_id: row.get(1)?,
                title: row.get(2)?,
                snippet: row.get(3)?,
                updated_at: row.get(4)?,
                rank: row.get(5)?,
            })
        })?;
        let mut hits = Vec::new();
        for row in rows {
            hits.push(row?);
        }
        Ok(hits)
    }

    pub fn stats(&self) -> Result<IndexStats, IndexError> {
        let count = |sql: &str| -> Result<i64, IndexError> {
            Ok(self.conn.query_row(sql, [], |row| row.get(0))?)
        };
        Ok(IndexStats {
            books: count("SELECT count(*) FROM books")?,
            notes: count("SELECT count(*) FROM notes")?,
            blocks: count("SELECT count(*) FROM note_blocks")?,
            fts_rows: count("SELECT count(*) FROM notes_fts")?,
        })
    }

    pub fn note_path(&self, note_id: &str) -> Result<Option<PathBuf>, IndexError> {
        let path: Option<String> = self
            .conn
            .query_row(
                "SELECT path FROM notes WHERE id = ?1",
                params![note_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(path.map(PathBuf::from))
    }
}

fn note_files(book: &Book) -> Result<Vec<(String, PathBuf)>, IndexError> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(book.notes_dir())? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some((key, _)) = crate::repository::parse_note_filename(&name) {
            files.push((key, entry.path()));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

fn fts_rowid(note_id: &str) -> i64 {
    let hash = blake3::hash(note_id.as_bytes());
    let bytes = hash.as_bytes();
    let mut value = [0u8; 8];
    value.copy_from_slice(&bytes[..8]);
    i64::from_le_bytes(value)
}

fn to_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| {
            let escaped = term.replace('"', "\"\"");
            format!("\"{escaped}\"*")
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::Store;
    use folio_core::{Block, FixedClock};

    fn setup() -> (tempfile::TempDir, Store, FixedClock, Index) {
        let dir = tempfile::tempdir().unwrap();
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let store = Store::init(dir.path(), "v", &clock).unwrap();
        let index = Index::open(&store.vault.index_path()).unwrap();
        (dir, store, clock, index)
    }

    #[test]
    fn index_and_search_with_accents() {
        let (_dir, store, clock, mut index) = setup();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        let mut note = store.create_note(&book, "Jardín", &clock).unwrap();
        note.blocks
            .push(Block::text_plain("El jardín tiene tomates y albahaca"));
        let path = store.save_note(&book, &mut note, &clock).unwrap();
        index.index_note(&book, &note, &path, "a0").unwrap();
        let hits = index.search("jardin", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.contains("jardín"));
        let stats = index.stats().unwrap();
        assert_eq!(stats.notes, 1);
        assert_eq!(stats.blocks, 1);
    }

    #[test]
    fn rebuild_from_files_and_delete() {
        let (_dir, store, clock, mut index) = setup();
        let book = store
            .create_book("A", "manual", None, None, "a", &clock)
            .unwrap();
        for index_note in 0..5 {
            let mut note = store
                .create_note(&book, &format!("Nota {index_note}"), &clock)
                .unwrap();
            note.blocks.push(Block::text_plain("contenido de prueba"));
            store.save_note(&book, &mut note, &clock).unwrap();
        }
        assert_eq!(index.rebuild(&store).unwrap(), 5);
        assert_eq!(index.stats().unwrap().notes, 5);
        let note = store.notes(&book).unwrap()[0].clone();
        store.delete_note(&book, &note.id, &clock).unwrap();
        index.remove_note(&note.id).unwrap();
        index.rebuild(&store).unwrap();
        assert_eq!(index.stats().unwrap().notes, 4);
    }
}

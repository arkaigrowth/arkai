//! Store connection management and initialization.
//!
//! The Store wraps a rusqlite Connection with WAL mode, foreign keys,
//! and automatic migration on open. Thread-safe for single-writer
//! (CLI tool pattern).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;

use super::migrations;

/// Configuration for opening a store.
#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// Path to the SQLite database file.
    pub path: PathBuf,
}

impl StoreConfig {
    /// Default store location: ~/.arkai/store.db
    pub fn default_path() -> Result<PathBuf> {
        let home = crate::config::arkai_home()?;
        Ok(home.join("store.db"))
    }
}

/// The canonical arkai store.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (or create) the store at the given path.
    /// Runs pending migrations automatically.
    pub fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create store directory: {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open store: {}", path.display()))?;

        Self::init_connection(&conn)?;
        migrations::run_migrations(&conn)?;

        Ok(Self { conn })
    }

    /// Open an in-memory store (for testing).
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("Failed to open in-memory store")?;

        Self::init_connection(&conn)?;
        migrations::run_migrations(&conn)?;

        Ok(Self { conn })
    }

    /// Configure the connection for arkai's usage pattern.
    fn init_connection(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;",
        )
        .context("Failed to configure store connection")?;
        Ok(())
    }

    /// Get a reference to the underlying connection.
    /// Prefer using query methods on Store directly.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Get a store config value by key.
    pub fn get_config(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT value FROM store_config WHERE key = ?1")?;

        let result = stmt.query_row([key], |row| row.get::<_, String>(0));

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e).context("Failed to get store config"),
        }
    }

    /// Set a store config value.
    pub fn set_config(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO store_config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    /// Get the current schema version.
    pub fn schema_version(&self) -> Result<i64> {
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory_store() {
        let store = Store::open_memory().unwrap();
        assert!(store.schema_version().unwrap() >= 1);
    }

    #[test]
    fn test_open_file_store() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_store.db");

        let store = Store::open(&db_path).unwrap();
        assert!(store.schema_version().unwrap() >= 1);
        assert!(db_path.exists());
    }

    #[test]
    fn test_reopen_preserves_data() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_store.db");

        // Open and write config
        {
            let store = Store::open(&db_path).unwrap();
            store.set_config("test_key", "test_value").unwrap();
        }

        // Reopen and verify
        {
            let store = Store::open(&db_path).unwrap();
            let value = store.get_config("test_key").unwrap();
            assert_eq!(value, Some("test_value".to_string()));
        }
    }

    #[test]
    fn test_get_set_config() {
        let store = Store::open_memory().unwrap();

        // Read default (upgraded by migration 002)
        let model = store.get_config("embedding_model").unwrap();
        assert_eq!(model, Some("mxbai-embed-large".to_string()));

        // Override
        store.set_config("embedding_model", "text-embedding-3-small").unwrap();
        let model = store.get_config("embedding_model").unwrap();
        assert_eq!(model, Some("text-embedding-3-small".to_string()));

        // Missing key
        let missing = store.get_config("nonexistent").unwrap();
        assert_eq!(missing, None);
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let store = Store::open_memory().unwrap();
        let fk: i64 = store
            .conn()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }
}

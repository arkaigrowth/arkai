//! Schema migration system for the arkai store.
//!
//! Every schema change is a numbered migration. The `schema_migrations`
//! table tracks which migrations have been applied. Migrations run
//! exactly once, in order, inside a transaction.

use anyhow::{Context, Result};
use rusqlite::Connection;

/// A single schema migration.
struct Migration {
    /// Monotonically increasing version number (1, 2, 3, ...)
    version: i64,
    /// Human-readable description
    description: &'static str,
    /// SQL to execute (may contain multiple statements separated by ;)
    sql: &'static str,
}

/// All migrations in order. Append new migrations to the end. Never
/// modify or reorder existing migrations.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "initial schema: items, entities, evidence, config",
        sql: r#"
            -- Key-value config (embedding model metadata, etc.)
            CREATE TABLE store_config (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- Canonical items: any content arkai knows about
            CREATE TABLE items (
                id           TEXT PRIMARY KEY,  -- content-addressed SHA256[0:16]
                item_type    TEXT NOT NULL,      -- 'content', 'email', 'calendar_event', 'voice_memo'
                title        TEXT NOT NULL,
                source_url   TEXT,
                content_type TEXT,               -- 'youtube', 'web', 'other' (for content items)
                tags         TEXT DEFAULT '[]',  -- JSON array of strings
                artifacts    TEXT DEFAULT '[]',  -- JSON array of artifact paths
                run_id       TEXT,               -- pipeline run that produced this item
                created_at   TEXT NOT NULL,      -- ISO 8601
                updated_at   TEXT NOT NULL,      -- ISO 8601
                metadata     TEXT DEFAULT '{}'   -- JSON object for type-specific fields
            );
            CREATE INDEX idx_items_type ON items(item_type);
            CREATE INDEX idx_items_content_type ON items(content_type);
            CREATE INDEX idx_items_created ON items(created_at);
            CREATE UNIQUE INDEX idx_items_source ON items(source_url) WHERE source_url IS NOT NULL;

            -- Full-text search on items
            CREATE VIRTUAL TABLE items_fts USING fts5(
                title, tags, metadata,
                content=items,
                content_rowid=rowid
            );

            -- Triggers to keep FTS in sync
            CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
                INSERT INTO items_fts(rowid, title, tags, metadata)
                VALUES (new.rowid, new.title, new.tags, new.metadata);
            END;
            CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
                INSERT INTO items_fts(items_fts, rowid, title, tags, metadata)
                VALUES ('delete', old.rowid, old.title, old.tags, old.metadata);
            END;
            CREATE TRIGGER items_au AFTER UPDATE ON items BEGIN
                INSERT INTO items_fts(items_fts, rowid, title, tags, metadata)
                VALUES ('delete', old.rowid, old.title, old.tags, old.metadata);
                INSERT INTO items_fts(rowid, title, tags, metadata)
                VALUES (new.rowid, new.title, new.tags, new.metadata);
            END;

            -- Canonical entities: people, orgs, concepts, etc.
            CREATE TABLE entities (
                id           TEXT PRIMARY KEY,  -- SHA256-based canonical ID
                name         TEXT NOT NULL,
                entity_type  TEXT NOT NULL,      -- 'person', 'org', 'concept', 'product', 'location', 'event'
                aliases      TEXT DEFAULT '[]',  -- JSON array of alternative names
                first_seen   TEXT NOT NULL,      -- ISO 8601
                metadata     TEXT DEFAULT '{}'   -- JSON object
            );
            CREATE INDEX idx_entities_type ON entities(entity_type);
            CREATE INDEX idx_entities_name ON entities(name);

            -- Junction: which entities appear in which items
            CREATE TABLE item_entities (
                item_id        TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
                entity_id      TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
                confidence     REAL DEFAULT 1.0,
                mention_count  INTEGER DEFAULT 1,
                PRIMARY KEY (item_id, entity_id)
            );
            CREATE INDEX idx_ie_entity ON item_entities(entity_id);

            -- Evidence: grounded claims with provenance
            CREATE TABLE evidence (
                id              TEXT PRIMARY KEY,  -- deterministic evidence ID
                item_id         TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
                claim           TEXT NOT NULL,
                quote           TEXT NOT NULL,
                quote_sha256    TEXT NOT NULL,
                status          TEXT NOT NULL,      -- 'resolved', 'ambiguous', 'unresolved'
                resolution_json TEXT DEFAULT '{}',  -- JSON: method, match_count, match_rank, reason
                span_artifact   TEXT,
                span_start      INTEGER,
                span_end        INTEGER,
                span_sha256     TEXT,
                anchor_text     TEXT,
                video_timestamp TEXT,
                confidence      REAL NOT NULL,
                extractor       TEXT NOT NULL,
                created_at      TEXT NOT NULL       -- ISO 8601
            );
            CREATE INDEX idx_evidence_item ON evidence(item_id);
            CREATE INDEX idx_evidence_status ON evidence(status);
            CREATE INDEX idx_evidence_extractor ON evidence(extractor);

            -- Embeddings: vector storage for semantic search
            -- Model metadata is in store_config, not here.
            CREATE TABLE embeddings (
                item_id    TEXT PRIMARY KEY REFERENCES items(id) ON DELETE CASCADE,
                model      TEXT NOT NULL,       -- model name at embedding time
                dimensions INTEGER NOT NULL,    -- vector length (e.g., 768)
                vector     BLOB NOT NULL,       -- raw f32 bytes (dimensions * 4 bytes)
                created_at TEXT NOT NULL        -- ISO 8601
            );

            -- Seed default config
            INSERT INTO store_config (key, value) VALUES
                ('schema_version', '1'),
                ('embedding_model', 'nomic-embed-text'),
                ('embedding_dimensions', '768'),
                ('embedding_provider', 'ollama');
        "#,
    },
    Migration {
        version: 2,
        description: "upgrade default embedding model to mxbai-embed-large (1024 dims)",
        sql: r#"
            UPDATE store_config SET value = 'mxbai-embed-large' WHERE key = 'embedding_model';
            UPDATE store_config SET value = '1024' WHERE key = 'embedding_dimensions';

            -- Clear stale embeddings from old model so they get re-embedded
            DELETE FROM embeddings WHERE model != 'mxbai-embed-large';
        "#,
    },
];

/// Ensure the schema_migrations table exists.
fn ensure_migrations_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version     INTEGER PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        );",
    )
    .context("Failed to create schema_migrations table")?;
    Ok(())
}

/// Get the current schema version (highest applied migration).
fn current_version(conn: &Connection) -> Result<i64> {
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .context("Failed to query schema version")?;
    Ok(version)
}

/// Run all pending migrations. Returns the number of migrations applied.
pub fn run_migrations(conn: &Connection) -> Result<usize> {
    ensure_migrations_table(conn)?;
    let current = current_version(conn)?;
    let mut applied = 0;

    for migration in MIGRATIONS {
        if migration.version <= current {
            continue;
        }

        conn.execute_batch("BEGIN IMMEDIATE;")
            .context("Failed to begin migration transaction")?;

        match (|| -> Result<()> {
            conn.execute_batch(migration.sql).with_context(|| {
                format!(
                    "Failed to apply migration {}: {}",
                    migration.version, migration.description
                )
            })?;

            conn.execute(
                "INSERT INTO schema_migrations (version, description) VALUES (?1, ?2)",
                rusqlite::params![migration.version, migration.description],
            )
            .context("Failed to record migration")?;

            Ok(())
        })() {
            Ok(()) => {
                conn.execute_batch("COMMIT;")
                    .context("Failed to commit migration")?;
                applied += 1;
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(e);
            }
        }
    }

    Ok(applied)
}

/// Get the latest migration version available in code.
pub fn latest_version() -> i64 {
    MIGRATIONS.last().map(|m| m.version).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        conn
    }

    #[test]
    fn test_migrations_run_idempotently() {
        let conn = memory_db();

        let applied1 = run_migrations(&conn).unwrap();
        assert_eq!(applied1, MIGRATIONS.len()); // all migrations applied

        let applied2 = run_migrations(&conn).unwrap();
        assert_eq!(applied2, 0); // already applied, nothing new
    }

    #[test]
    fn test_current_version_after_migration() {
        let conn = memory_db();
        run_migrations(&conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), latest_version());
    }

    #[test]
    fn test_schema_migrations_table_records() {
        let conn = memory_db();
        run_migrations(&conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, MIGRATIONS.len() as i64);
    }

    #[test]
    fn test_all_tables_created() {
        let conn = memory_db();
        run_migrations(&conn).unwrap();

        let expected_tables = [
            "store_config",
            "items",
            "entities",
            "item_entities",
            "evidence",
            "embeddings",
            "schema_migrations",
        ];

        for table in expected_tables {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists, "Table '{}' should exist after migration", table);
        }
    }

    #[test]
    fn test_fts_table_created() {
        let conn = memory_db();
        run_migrations(&conn).unwrap();

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='items_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists, "FTS5 virtual table should exist");
    }

    #[test]
    fn test_default_config_seeded() {
        let conn = memory_db();
        run_migrations(&conn).unwrap();

        let model: String = conn
            .query_row(
                "SELECT value FROM store_config WHERE key = 'embedding_model'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(model, "mxbai-embed-large"); // upgraded by migration 002

        let dims: String = conn
            .query_row(
                "SELECT value FROM store_config WHERE key = 'embedding_dimensions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dims, "1024"); // upgraded by migration 002
    }

    #[test]
    fn test_schema_version_matches_latest() {
        let conn = memory_db();
        run_migrations(&conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 2);
    }
}

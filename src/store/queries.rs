//! Query layer for the arkai store.
//!
//! All database reads and writes go through these functions.
//! Each function takes a &Store and returns Result<T>.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::db::Store;

// ─────────────────────────────────────────────────────────────────
// Item types
// ─────────────────────────────────────────────────────────────────

/// An item in the canonical store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub item_type: String,
    pub title: String,
    pub source_url: Option<String>,
    pub content_type: Option<String>,
    pub tags: Vec<String>,
    pub artifacts: Vec<String>,
    pub run_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

/// Parameters for inserting or updating an item.
#[derive(Debug, Clone)]
pub struct UpsertItem<'a> {
    pub id: &'a str,
    pub item_type: &'a str,
    pub title: &'a str,
    pub source_url: Option<&'a str>,
    pub content_type: Option<&'a str>,
    pub tags: &'a [String],
    pub artifacts: &'a [String],
    pub run_id: Option<&'a str>,
    pub metadata: &'a serde_json::Value,
}

// ─────────────────────────────────────────────────────────────────
// Entity types
// ─────────────────────────────────────────────────────────────────

/// A canonical entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub aliases: Vec<String>,
    pub first_seen: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

/// Parameters for inserting an entity.
#[derive(Debug, Clone)]
pub struct InsertEntity<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub entity_type: &'a str,
    pub aliases: &'a [String],
    pub metadata: &'a serde_json::Value,
}

// ─────────────────────────────────────────────────────────────────
// Evidence types
// ─────────────────────────────────────────────────────────────────

/// An evidence entry with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRow {
    pub id: String,
    pub item_id: String,
    pub claim: String,
    pub quote: String,
    pub quote_sha256: String,
    pub status: String,
    pub confidence: f64,
    pub extractor: String,
    pub created_at: DateTime<Utc>,
}

/// Search result combining FTS rank with item data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub item: Item,
    pub rank: f64,
}

// ─────────────────────────────────────────────────────────────────
// Item CRUD
// ─────────────────────────────────────────────────────────────────

/// Insert or update an item. Returns true if a new row was created.
pub fn upsert_item(store: &Store, item: &UpsertItem) -> Result<bool> {
    let now = Utc::now().to_rfc3339();
    let tags_json = serde_json::to_string(item.tags)?;
    let artifacts_json = serde_json::to_string(item.artifacts)?;
    let metadata_str = serde_json::to_string(item.metadata)?;

    let changes = store.conn().execute(
        "INSERT INTO items (id, item_type, title, source_url, content_type, tags, artifacts, run_id, created_at, updated_at, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
             title = excluded.title,
             tags = excluded.tags,
             artifacts = excluded.artifacts,
             run_id = COALESCE(excluded.run_id, items.run_id),
             updated_at = excluded.updated_at,
             metadata = excluded.metadata",
        params![
            item.id,
            item.item_type,
            item.title,
            item.source_url,
            item.content_type,
            tags_json,
            artifacts_json,
            item.run_id,
            now,
            metadata_str,
        ],
    )?;

    Ok(changes > 0)
}

/// Get an item by ID.
pub fn get_item(store: &Store, id: &str) -> Result<Option<Item>> {
    let mut stmt = store.conn().prepare_cached(
        "SELECT id, item_type, title, source_url, content_type, tags, artifacts, run_id, created_at, updated_at, metadata
         FROM items WHERE id = ?1",
    )?;

    let result = stmt.query_row([id], |row| {
        Ok(ItemRow {
            id: row.get(0)?,
            item_type: row.get(1)?,
            title: row.get(2)?,
            source_url: row.get(3)?,
            content_type: row.get(4)?,
            tags: row.get(5)?,
            artifacts: row.get(6)?,
            run_id: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            metadata: row.get(10)?,
        })
    });

    match result {
        Ok(row) => Ok(Some(row.into_item()?)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get item"),
    }
}

/// List items, most recent first. Optional limit.
pub fn list_items(store: &Store, limit: Option<usize>) -> Result<Vec<Item>> {
    let limit_val = limit.unwrap_or(100) as i64;
    let mut stmt = store.conn().prepare_cached(
        "SELECT id, item_type, title, source_url, content_type, tags, artifacts, run_id, created_at, updated_at, metadata
         FROM items ORDER BY created_at DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map([limit_val], |row| {
        Ok(ItemRow {
            id: row.get(0)?,
            item_type: row.get(1)?,
            title: row.get(2)?,
            source_url: row.get(3)?,
            content_type: row.get(4)?,
            tags: row.get(5)?,
            artifacts: row.get(6)?,
            run_id: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            metadata: row.get(10)?,
        })
    })?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row?.into_item()?);
    }
    Ok(items)
}

/// Delete an item by ID. Returns true if a row was deleted.
pub fn delete_item(store: &Store, id: &str) -> Result<bool> {
    let changes = store
        .conn()
        .execute("DELETE FROM items WHERE id = ?1", [id])?;
    Ok(changes > 0)
}

/// Count items, optionally filtered by type.
pub fn count_items(store: &Store, item_type: Option<&str>) -> Result<i64> {
    let count = match item_type {
        Some(t) => store.conn().query_row(
            "SELECT COUNT(*) FROM items WHERE item_type = ?1",
            [t],
            |row| row.get(0),
        )?,
        None => store
            .conn()
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))?,
    };
    Ok(count)
}

// ─────────────────────────────────────────────────────────────────
// Search
// ─────────────────────────────────────────────────────────────────

/// Full-text search on items. Returns results ranked by relevance.
pub fn search_items(store: &Store, query: &str, limit: Option<usize>) -> Result<Vec<SearchResult>> {
    let safe_query = crate::store::search::sanitize_fts_query(query);
    if safe_query.is_empty() {
        return Ok(Vec::new());
    }
    let limit_val = limit.unwrap_or(20) as i64;

    let mut stmt = store.conn().prepare_cached(
        "SELECT i.id, i.item_type, i.title, i.source_url, i.content_type,
                i.tags, i.artifacts, i.run_id, i.created_at, i.updated_at, i.metadata,
                fts.rank
         FROM items_fts fts
         JOIN items i ON i.rowid = fts.rowid
         WHERE items_fts MATCH ?1
         ORDER BY fts.rank
         LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![safe_query, limit_val], |row| {
        Ok(SearchRow {
            item: ItemRow {
                id: row.get(0)?,
                item_type: row.get(1)?,
                title: row.get(2)?,
                source_url: row.get(3)?,
                content_type: row.get(4)?,
                tags: row.get(5)?,
                artifacts: row.get(6)?,
                run_id: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                metadata: row.get(10)?,
            },
            rank: row.get(11)?,
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        let sr = row?;
        results.push(SearchResult {
            item: sr.item.into_item()?,
            rank: sr.rank,
        });
    }
    Ok(results)
}

// ─────────────────────────────────────────────────────────────────
// Entity CRUD
// ─────────────────────────────────────────────────────────────────

/// Insert an entity. Returns false if the ID already exists.
pub fn insert_entity(store: &Store, entity: &InsertEntity) -> Result<bool> {
    let now = Utc::now().to_rfc3339();
    let aliases_json = serde_json::to_string(entity.aliases)?;
    let metadata_str = serde_json::to_string(entity.metadata)?;

    let changes = store.conn().execute(
        "INSERT OR IGNORE INTO entities (id, name, entity_type, aliases, first_seen, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            entity.id,
            entity.name,
            entity.entity_type,
            aliases_json,
            now,
            metadata_str,
        ],
    )?;

    Ok(changes > 0)
}

/// Get an entity by ID.
pub fn get_entity(store: &Store, id: &str) -> Result<Option<Entity>> {
    let mut stmt = store.conn().prepare_cached(
        "SELECT id, name, entity_type, aliases, first_seen, metadata
         FROM entities WHERE id = ?1",
    )?;

    let result = stmt.query_row([id], |row| {
        Ok(EntityRow {
            id: row.get(0)?,
            name: row.get(1)?,
            entity_type: row.get(2)?,
            aliases: row.get(3)?,
            first_seen: row.get(4)?,
            metadata: row.get(5)?,
        })
    });

    match result {
        Ok(row) => Ok(Some(row.into_entity()?)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get entity"),
    }
}

/// Link an entity to an item.
pub fn link_entity_to_item(
    store: &Store,
    item_id: &str,
    entity_id: &str,
    confidence: f64,
) -> Result<()> {
    store.conn().execute(
        "INSERT INTO item_entities (item_id, entity_id, confidence, mention_count)
         VALUES (?1, ?2, ?3, 1)
         ON CONFLICT(item_id, entity_id) DO UPDATE SET
             confidence = MAX(item_entities.confidence, excluded.confidence),
             mention_count = item_entities.mention_count + 1",
        params![item_id, entity_id, confidence],
    )?;
    Ok(())
}

/// Get all entities linked to an item.
pub fn entities_for_item(store: &Store, item_id: &str) -> Result<Vec<Entity>> {
    let mut stmt = store.conn().prepare_cached(
        "SELECT e.id, e.name, e.entity_type, e.aliases, e.first_seen, e.metadata
         FROM entities e
         JOIN item_entities ie ON ie.entity_id = e.id
         WHERE ie.item_id = ?1
         ORDER BY ie.confidence DESC",
    )?;

    let rows = stmt.query_map([item_id], |row| {
        Ok(EntityRow {
            id: row.get(0)?,
            name: row.get(1)?,
            entity_type: row.get(2)?,
            aliases: row.get(3)?,
            first_seen: row.get(4)?,
            metadata: row.get(5)?,
        })
    })?;

    let mut entities = Vec::new();
    for row in rows {
        entities.push(row?.into_entity()?);
    }
    Ok(entities)
}

/// Get all items linked to an entity.
pub fn items_for_entity(store: &Store, entity_id: &str) -> Result<Vec<Item>> {
    let mut stmt = store.conn().prepare_cached(
        "SELECT i.id, i.item_type, i.title, i.source_url, i.content_type,
                i.tags, i.artifacts, i.run_id, i.created_at, i.updated_at, i.metadata
         FROM items i
         JOIN item_entities ie ON ie.item_id = i.id
         WHERE ie.entity_id = ?1
         ORDER BY i.created_at DESC",
    )?;

    let rows = stmt.query_map([entity_id], |row| {
        Ok(ItemRow {
            id: row.get(0)?,
            item_type: row.get(1)?,
            title: row.get(2)?,
            source_url: row.get(3)?,
            content_type: row.get(4)?,
            tags: row.get(5)?,
            artifacts: row.get(6)?,
            run_id: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            metadata: row.get(10)?,
        })
    })?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row?.into_item()?);
    }
    Ok(items)
}

// ─────────────────────────────────────────────────────────────────
// Evidence CRUD
// ─────────────────────────────────────────────────────────────────

/// Insert an evidence entry.
pub fn insert_evidence(
    store: &Store,
    id: &str,
    item_id: &str,
    claim: &str,
    quote: &str,
    quote_sha256: &str,
    status: &str,
    resolution_json: &str,
    span_artifact: Option<&str>,
    span_start: Option<i64>,
    span_end: Option<i64>,
    span_sha256: Option<&str>,
    anchor_text: Option<&str>,
    video_timestamp: Option<&str>,
    confidence: f64,
    extractor: &str,
) -> Result<bool> {
    let now = Utc::now().to_rfc3339();
    let changes = store.conn().execute(
        "INSERT OR IGNORE INTO evidence
         (id, item_id, claim, quote, quote_sha256, status, resolution_json,
          span_artifact, span_start, span_end, span_sha256, anchor_text,
          video_timestamp, confidence, extractor, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            id, item_id, claim, quote, quote_sha256, status, resolution_json,
            span_artifact, span_start, span_end, span_sha256, anchor_text,
            video_timestamp, confidence, extractor, now,
        ],
    )?;
    Ok(changes > 0)
}

/// Get all evidence for an item.
pub fn evidence_for_item(store: &Store, item_id: &str) -> Result<Vec<EvidenceRow>> {
    let mut stmt = store.conn().prepare_cached(
        "SELECT id, item_id, claim, quote, quote_sha256, status, confidence, extractor, created_at
         FROM evidence WHERE item_id = ?1 ORDER BY created_at",
    )?;

    let rows = stmt.query_map([item_id], |row| {
        Ok(EvidenceRow {
            id: row.get(0)?,
            item_id: row.get(1)?,
            claim: row.get(2)?,
            quote: row.get(3)?,
            quote_sha256: row.get(4)?,
            status: row.get(5)?,
            confidence: row.get(6)?,
            extractor: row.get(7)?,
            created_at: row.get::<_, String>(8)
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)).unwrap_or_default())?,
        })
    })?;

    let mut evidence = Vec::new();
    for row in rows {
        evidence.push(row?);
    }
    Ok(evidence)
}

// ─────────────────────────────────────────────────────────────────
// Embedding storage
// ─────────────────────────────────────────────────────────────────

/// Store an embedding vector for an item.
pub fn store_embedding(
    store: &Store,
    item_id: &str,
    model: &str,
    dimensions: i32,
    vector: &[f32],
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    // Store as raw bytes: each f32 is 4 bytes, little-endian
    let blob: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();

    store.conn().execute(
        "INSERT INTO embeddings (item_id, model, dimensions, vector, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(item_id) DO UPDATE SET
             model = excluded.model,
             dimensions = excluded.dimensions,
             vector = excluded.vector,
             created_at = excluded.created_at",
        params![item_id, model, dimensions, blob, now],
    )?;
    Ok(())
}

/// Load an embedding vector for an item.
pub fn get_embedding(store: &Store, item_id: &str) -> Result<Option<Vec<f32>>> {
    let mut stmt = store
        .conn()
        .prepare_cached("SELECT vector, dimensions FROM embeddings WHERE item_id = ?1")?;

    let result = stmt.query_row([item_id], |row| {
        let blob: Vec<u8> = row.get(0)?;
        let dims: i32 = row.get(1)?;
        Ok((blob, dims))
    });

    match result {
        Ok((blob, dims)) => {
            let expected_bytes = dims as usize * 4;
            if blob.len() != expected_bytes {
                anyhow::bail!(
                    "Embedding blob size mismatch: expected {} bytes, got {}",
                    expected_bytes,
                    blob.len()
                );
            }
            let vector: Vec<f32> = blob
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            Ok(Some(vector))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("Failed to get embedding"),
    }
}

// ─────────────────────────────────────────────────────────────────
// Chunk CRUD
// ─────────────────────────────────────────────────────────────────

/// A chunk row from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRow {
    pub id: String,
    pub item_id: String,
    pub chunk_index: i64,
    pub text: String,
    pub byte_start: i64,
    pub byte_end: i64,
    pub word_count: i64,
}

/// Insert a chunk. Uses INSERT OR REPLACE for idempotency.
pub fn insert_chunk(
    store: &Store,
    id: &str,
    item_id: &str,
    chunk_index: i64,
    text: &str,
    byte_start: i64,
    byte_end: i64,
    word_count: i64,
    metadata: &str,
) -> Result<()> {
    store.conn().execute(
        "INSERT OR REPLACE INTO chunks (id, item_id, chunk_index, text, byte_start, byte_end, word_count, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, item_id, chunk_index, text, byte_start, byte_end, word_count, metadata],
    )?;
    Ok(())
}

/// Get all chunks for an item, ordered by chunk_index.
pub fn get_chunks_for_item(store: &Store, item_id: &str) -> Result<Vec<ChunkRow>> {
    let mut stmt = store.conn().prepare_cached(
        "SELECT id, item_id, chunk_index, text, byte_start, byte_end, word_count
         FROM chunks WHERE item_id = ?1 ORDER BY chunk_index",
    )?;

    let rows = stmt.query_map([item_id], |row| {
        Ok(ChunkRow {
            id: row.get(0)?,
            item_id: row.get(1)?,
            chunk_index: row.get(2)?,
            text: row.get(3)?,
            byte_start: row.get(4)?,
            byte_end: row.get(5)?,
            word_count: row.get(6)?,
        })
    })?;

    let mut chunks = Vec::new();
    for row in rows {
        chunks.push(row?);
    }
    Ok(chunks)
}

/// Count chunks, optionally for a specific item.
pub fn count_chunks(store: &Store, item_id: Option<&str>) -> Result<i64> {
    let count = match item_id {
        Some(id) => store.conn().query_row(
            "SELECT COUNT(*) FROM chunks WHERE item_id = ?1",
            [id],
            |row| row.get(0),
        )?,
        None => store
            .conn()
            .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?,
    };
    Ok(count)
}

/// Store a chunk embedding.
pub fn store_chunk_embedding(
    store: &Store,
    chunk_id: &str,
    model: &str,
    dimensions: i32,
    vector: &[f32],
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let blob: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();

    store.conn().execute(
        "INSERT INTO chunk_embeddings (chunk_id, model, dimensions, vector, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(chunk_id) DO UPDATE SET
             model = excluded.model,
             dimensions = excluded.dimensions,
             vector = excluded.vector,
             created_at = excluded.created_at",
        params![chunk_id, model, dimensions, blob, now],
    )?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// Capture triage queries
// ─────────────────────────────────────────────────────────────────

/// List active captures: not done, and not snoozed (or snooze expired).
///
/// Uses COALESCE for missing metadata fields so the query is resilient
/// to captures created before status/horizon/priority were populated.
pub fn list_active_captures(store: &Store) -> Result<Vec<Item>> {
    let now = Utc::now().to_rfc3339();
    let mut stmt = store.conn().prepare_cached(
        "SELECT id, item_type, title, source_url, content_type,
                tags, artifacts, run_id, created_at, updated_at, metadata
         FROM items
         WHERE item_type = 'capture'
           AND COALESCE(json_extract(metadata, '$.status'), 'inbox') != 'done'
           AND (
               COALESCE(json_extract(metadata, '$.status'), 'inbox') != 'snoozed'
               OR json_extract(metadata, '$.snoozed_until') <= ?1
               OR json_extract(metadata, '$.snoozed_until') IS NULL
           )
         ORDER BY created_at DESC",
    )?;

    let rows = stmt.query_map([&now], |row| {
        Ok(ItemRow {
            id: row.get(0)?,
            item_type: row.get(1)?,
            title: row.get(2)?,
            source_url: row.get(3)?,
            content_type: row.get(4)?,
            tags: row.get(5)?,
            artifacts: row.get(6)?,
            run_id: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            metadata: row.get(10)?,
        })
    })?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row?.into_item()?);
    }
    Ok(items)
}

/// Count snoozed captures (for the today summary footer).
pub fn count_snoozed_captures(store: &Store) -> Result<i64> {
    let now = Utc::now().to_rfc3339();
    let count: i64 = store.conn().query_row(
        "SELECT COUNT(*) FROM items
         WHERE item_type = 'capture'
           AND json_extract(metadata, '$.status') = 'snoozed'
           AND json_extract(metadata, '$.snoozed_until') > ?1",
        [&now],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Update a capture's status (done, snoozed, etc.) using json_set.
///
/// Returns true if a row was updated, false if the item doesn't exist
/// or is not a capture.
pub fn update_capture_status(
    store: &Store,
    id: &str,
    new_status: &str,
    snoozed_until: Option<&str>,
) -> Result<bool> {
    let now = Utc::now().to_rfc3339();
    let rows_changed = store.conn().execute(
        "UPDATE items
         SET metadata = json_set(metadata, '$.status', ?1, '$.snoozed_until', ?2),
             updated_at = ?3
         WHERE id = ?4 AND item_type = 'capture'",
        params![new_status, snoozed_until, now, id],
    )?;
    Ok(rows_changed > 0)
}

/// Resolve a capture ID from an exact ID or unique prefix.
///
/// Returns the full ID if:
/// - The input matches a capture ID exactly, OR
/// - The input is a unique prefix of exactly one capture ID.
///
/// Returns an error if the prefix is ambiguous (matches multiple captures)
/// or if no capture matches at all.
pub fn resolve_capture_id(store: &Store, id_or_prefix: &str) -> Result<String> {
    // Try exact match first
    let exact: Option<String> = store
        .conn()
        .query_row(
            "SELECT id FROM items WHERE id = ?1 AND item_type = 'capture'",
            [id_or_prefix],
            |row| row.get(0),
        )
        .ok();

    if let Some(full_id) = exact {
        return Ok(full_id);
    }

    // Try prefix match
    let pattern = format!("{}%", id_or_prefix);
    let mut stmt = store.conn().prepare_cached(
        "SELECT id FROM items WHERE id LIKE ?1 AND item_type = 'capture'",
    )?;
    let matches: Vec<String> = stmt
        .query_map([&pattern], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    match matches.len() {
        0 => anyhow::bail!("No capture found matching '{}'", id_or_prefix),
        1 => Ok(matches.into_iter().next().unwrap()),
        n => {
            let previews: Vec<String> = matches.iter().take(5).cloned().collect();
            anyhow::bail!(
                "Ambiguous prefix '{}': matches {} captures ({})",
                id_or_prefix,
                n,
                previews.join(", ")
            )
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Internal row types (avoid exposing rusqlite types)
// ─────────────────────────────────────────────────────────────────

struct ItemRow {
    id: String,
    item_type: String,
    title: String,
    source_url: Option<String>,
    content_type: Option<String>,
    tags: String,
    artifacts: String,
    run_id: Option<String>,
    created_at: String,
    updated_at: String,
    metadata: String,
}

impl ItemRow {
    fn into_item(self) -> Result<Item> {
        Ok(Item {
            id: self.id,
            item_type: self.item_type,
            title: self.title,
            source_url: self.source_url,
            content_type: self.content_type,
            tags: serde_json::from_str(&self.tags).unwrap_or_default(),
            artifacts: serde_json::from_str(&self.artifacts).unwrap_or_default(),
            run_id: self.run_id,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_default(),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_default(),
            metadata: serde_json::from_str(&self.metadata).unwrap_or_default(),
        })
    }
}

struct EntityRow {
    id: String,
    name: String,
    entity_type: String,
    aliases: String,
    first_seen: String,
    metadata: String,
}

impl EntityRow {
    fn into_entity(self) -> Result<Entity> {
        Ok(Entity {
            id: self.id,
            name: self.name,
            entity_type: self.entity_type,
            aliases: serde_json::from_str(&self.aliases).unwrap_or_default(),
            first_seen: DateTime::parse_from_rfc3339(&self.first_seen)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_default(),
            metadata: serde_json::from_str(&self.metadata).unwrap_or_default(),
        })
    }
}

struct SearchRow {
    item: ItemRow,
    rank: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_store() -> Store {
        Store::open_memory().unwrap()
    }

    // Owned data for sample_item to borrow from.
    struct SampleData {
        tags: Vec<String>,
        artifacts: Vec<String>,
        metadata: serde_json::Value,
    }

    impl SampleData {
        fn new() -> Self {
            Self {
                tags: vec!["rust".to_string(), "programming".to_string()],
                artifacts: vec!["transcript.md".to_string()],
                metadata: json!({"duration_seconds": 3600}),
            }
        }

        fn as_upsert(&self) -> UpsertItem {
            UpsertItem {
                id: "abc123def456",
                item_type: "content",
                title: "Introduction to Rust Programming",
                source_url: Some("https://youtube.com/watch?v=abc123"),
                content_type: Some("youtube"),
                tags: &self.tags,
                artifacts: &self.artifacts,
                run_id: Some("run-001"),
                metadata: &self.metadata,
            }
        }
    }

    // ── Item tests ──────────────────────────────────────────────

    #[test]
    fn test_upsert_and_get_item() {
        let store = test_store();
        let data = SampleData::new();
        let item = data.as_upsert();

        let created = upsert_item(&store, &item).unwrap();
        assert!(created);

        let loaded = get_item(&store, "abc123def456").unwrap().unwrap();
        assert_eq!(loaded.title, "Introduction to Rust Programming");
        assert_eq!(loaded.tags, vec!["rust", "programming"]);
        assert_eq!(loaded.content_type, Some("youtube".to_string()));
    }

    #[test]
    fn test_upsert_updates_existing() {
        let store = test_store();
        let tags_v1 = vec!["rust".to_string()];
        let tags_v2 = vec!["rust".to_string(), "advanced".to_string()];

        let item_v1 = UpsertItem {
            id: "abc123def456",
            item_type: "content",
            title: "Rust Basics",
            source_url: Some("https://example.com"),
            content_type: Some("web"),
            tags: &tags_v1,
            artifacts: &[],
            run_id: None,
            metadata: &json!({}),
        };
        upsert_item(&store, &item_v1).unwrap();

        let item_v2 = UpsertItem {
            id: "abc123def456",
            item_type: "content",
            title: "Rust Advanced",
            source_url: Some("https://example.com"),
            content_type: Some("web"),
            tags: &tags_v2,
            artifacts: &[],
            run_id: None,
            metadata: &json!({}),
        };
        upsert_item(&store, &item_v2).unwrap();

        let loaded = get_item(&store, "abc123def456").unwrap().unwrap();
        assert_eq!(loaded.title, "Rust Advanced");
        assert_eq!(loaded.tags.len(), 2);

        // Should still be 1 item, not 2
        assert_eq!(count_items(&store, None).unwrap(), 1);
    }

    #[test]
    fn test_get_nonexistent_item() {
        let store = test_store();
        let result = get_item(&store, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_items_ordered_by_date() {
        let store = test_store();
        let tags = vec![];

        for i in 0..5 {
            let id = format!("item{:012}", i);
            let title = format!("Item {}", i);
            let item = UpsertItem {
                id: &id,
                item_type: "content",
                title: &title,
                source_url: None,
                content_type: None,
                tags: &tags,
                artifacts: &[],
                run_id: None,
                metadata: &json!({}),
            };
            upsert_item(&store, &item).unwrap();
        }

        let items = list_items(&store, Some(3)).unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_delete_item() {
        let store = test_store();
        upsert_item(&store, &SampleData::new().as_upsert()).unwrap();

        assert!(delete_item(&store, "abc123def456").unwrap());
        assert!(!delete_item(&store, "abc123def456").unwrap()); // already gone
        assert!(get_item(&store, "abc123def456").unwrap().is_none());
    }

    #[test]
    fn test_count_items_with_filter() {
        let store = test_store();
        let tags = vec![];

        for (id, item_type) in [("aaa", "content"), ("bbb", "content"), ("ccc", "email")] {
            let padded_id = format!("{:0>12}", id);
            let item = UpsertItem {
                id: &padded_id,
                item_type,
                title: "Test",
                source_url: None,
                content_type: None,
                tags: &tags,
                artifacts: &[],
                run_id: None,
                metadata: &json!({}),
            };
            upsert_item(&store, &item).unwrap();
        }

        assert_eq!(count_items(&store, None).unwrap(), 3);
        assert_eq!(count_items(&store, Some("content")).unwrap(), 2);
        assert_eq!(count_items(&store, Some("email")).unwrap(), 1);
    }

    // ── Search tests ────────────────────────────────────────────

    #[test]
    fn test_fts_search() {
        let store = test_store();
        upsert_item(&store, &SampleData::new().as_upsert()).unwrap();

        let tags = vec![];
        let other = UpsertItem {
            id: "xyz789xyz789",
            item_type: "content",
            title: "Python for Data Science",
            source_url: None,
            content_type: Some("web"),
            tags: &tags,
            artifacts: &[],
            run_id: None,
            metadata: &json!({}),
        };
        upsert_item(&store, &other).unwrap();

        // Search for "rust" should find only the Rust item
        let results = search_items(&store, "rust", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item.title, "Introduction to Rust Programming");

        // Search for "programming" should also find it
        let results = search_items(&store, "programming", None).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_fts_search_no_results() {
        let store = test_store();
        upsert_item(&store, &SampleData::new().as_upsert()).unwrap();

        let results = search_items(&store, "quantum", None).unwrap();
        assert!(results.is_empty());
    }

    // ── Entity tests ────────────────────────────────────────────

    #[test]
    fn test_insert_and_get_entity() {
        let store = test_store();

        let entity = InsertEntity {
            id: "ent_001",
            name: "Alex Hormozi",
            entity_type: "person",
            aliases: &["Hormozi".to_string()],
            metadata: &json!({"role": "entrepreneur"}),
        };
        assert!(insert_entity(&store, &entity).unwrap());

        let loaded = get_entity(&store, "ent_001").unwrap().unwrap();
        assert_eq!(loaded.name, "Alex Hormozi");
        assert_eq!(loaded.aliases, vec!["Hormozi"]);
    }

    #[test]
    fn test_insert_entity_duplicate_ignored() {
        let store = test_store();

        let entity = InsertEntity {
            id: "ent_001",
            name: "Alex Hormozi",
            entity_type: "person",
            aliases: &[],
            metadata: &json!({}),
        };
        assert!(insert_entity(&store, &entity).unwrap());
        assert!(!insert_entity(&store, &entity).unwrap()); // duplicate
    }

    // ── Entity linking tests ────────────────────────────────────

    #[test]
    fn test_link_entity_to_item() {
        let store = test_store();

        upsert_item(&store, &SampleData::new().as_upsert()).unwrap();
        let entity = InsertEntity {
            id: "ent_001",
            name: "Rust Foundation",
            entity_type: "org",
            aliases: &[],
            metadata: &json!({}),
        };
        insert_entity(&store, &entity).unwrap();

        link_entity_to_item(&store, "abc123def456", "ent_001", 0.9).unwrap();

        let entities = entities_for_item(&store, "abc123def456").unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Rust Foundation");

        let items = items_for_entity(&store, "ent_001").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Introduction to Rust Programming");
    }

    #[test]
    fn test_link_entity_increments_mentions() {
        let store = test_store();

        upsert_item(&store, &SampleData::new().as_upsert()).unwrap();
        let entity = InsertEntity {
            id: "ent_001",
            name: "Test",
            entity_type: "concept",
            aliases: &[],
            metadata: &json!({}),
        };
        insert_entity(&store, &entity).unwrap();

        link_entity_to_item(&store, "abc123def456", "ent_001", 0.8).unwrap();
        link_entity_to_item(&store, "abc123def456", "ent_001", 0.95).unwrap();

        // Check mention_count incremented
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT mention_count FROM item_entities WHERE item_id = ?1 AND entity_id = ?2",
                ["abc123def456", "ent_001"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);

        // Check confidence is MAX of both
        let conf: f64 = store
            .conn()
            .query_row(
                "SELECT confidence FROM item_entities WHERE item_id = ?1 AND entity_id = ?2",
                ["abc123def456", "ent_001"],
                |row| row.get(0),
            )
            .unwrap();
        assert!((conf - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cascade_delete_cleans_links() {
        let store = test_store();

        upsert_item(&store, &SampleData::new().as_upsert()).unwrap();
        let entity = InsertEntity {
            id: "ent_001",
            name: "Test",
            entity_type: "concept",
            aliases: &[],
            metadata: &json!({}),
        };
        insert_entity(&store, &entity).unwrap();
        link_entity_to_item(&store, "abc123def456", "ent_001", 0.9).unwrap();

        // Delete item should cascade to item_entities
        delete_item(&store, "abc123def456").unwrap();

        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM item_entities WHERE item_id = 'abc123def456'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    // ── Embedding tests ─────────────────────────────────────────

    #[test]
    fn test_store_and_retrieve_embedding() {
        let store = test_store();
        upsert_item(&store, &SampleData::new().as_upsert()).unwrap();

        let vector: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
        store_embedding(&store, "abc123def456", "nomic-embed-text", 768, &vector).unwrap();

        let loaded = get_embedding(&store, "abc123def456").unwrap().unwrap();
        assert_eq!(loaded.len(), 768);
        assert!((loaded[0] - 0.0).abs() < f64::EPSILON as f32);
        assert!((loaded[767] - 767.0 / 768.0).abs() < 0.001);
    }

    #[test]
    fn test_embedding_upsert_overwrites() {
        let store = test_store();
        upsert_item(&store, &SampleData::new().as_upsert()).unwrap();

        let v1: Vec<f32> = vec![1.0; 768];
        store_embedding(&store, "abc123def456", "model-v1", 768, &v1).unwrap();

        let v2: Vec<f32> = vec![2.0; 768];
        store_embedding(&store, "abc123def456", "model-v2", 768, &v2).unwrap();

        let loaded = get_embedding(&store, "abc123def456").unwrap().unwrap();
        assert!((loaded[0] - 2.0).abs() < f64::EPSILON as f32);
    }

    #[test]
    fn test_get_nonexistent_embedding() {
        let store = test_store();
        let result = get_embedding(&store, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    // ── Evidence tests ──────────────────────────────────────────

    #[test]
    fn test_insert_and_query_evidence() {
        let store = test_store();
        upsert_item(&store, &SampleData::new().as_upsert()).unwrap();

        let inserted = insert_evidence(
            &store,
            "ev_001",
            "abc123def456",
            "Rust is memory safe",
            "Rust guarantees memory safety without garbage collection",
            "sha256:abc123",
            "resolved",
            r#"{"method":"exact","match_count":1,"match_rank":1}"#,
            Some("transcript.md"),
            Some(1024),
            Some(1090),
            Some("sha256:slice123"),
            Some("...guarantees memory safety..."),
            Some("00:15:30"),
            0.95,
            "extract_claims",
        )
        .unwrap();
        assert!(inserted);

        let evidence = evidence_for_item(&store, "abc123def456").unwrap();
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].claim, "Rust is memory safe");
        assert_eq!(evidence[0].status, "resolved");
    }

    #[test]
    fn test_evidence_duplicate_ignored() {
        let store = test_store();
        upsert_item(&store, &SampleData::new().as_upsert()).unwrap();

        insert_evidence(
            &store, "ev_001", "abc123def456", "claim", "quote", "sha", "resolved",
            "{}", None, None, None, None, None, None, 0.9, "test",
        )
        .unwrap();

        let dup = insert_evidence(
            &store, "ev_001", "abc123def456", "claim", "quote", "sha", "resolved",
            "{}", None, None, None, None, None, None, 0.9, "test",
        )
        .unwrap();
        assert!(!dup); // duplicate ignored
    }

    // -- Capture triage query tests --

    fn insert_capture(store: &Store, id: &str, title: &str, metadata: serde_json::Value) {
        let tags: Vec<String> = vec![];
        let upsert = UpsertItem {
            id,
            item_type: "capture",
            title,
            source_url: None,
            content_type: None,
            tags: &tags,
            artifacts: &[],
            run_id: None,
            metadata: &metadata,
        };
        upsert_item(store, &upsert).unwrap();
    }

    #[test]
    fn test_list_active_captures_excludes_done() {
        let store = test_store();
        insert_capture(&store, "c1", "active todo", serde_json::json!({"status": "inbox"}));
        insert_capture(&store, "c2", "done task", serde_json::json!({"status": "done"}));
        insert_capture(&store, "c3", "triaged item", serde_json::json!({"status": "triaged"}));

        let active = list_active_captures(&store).unwrap();
        assert_eq!(active.len(), 2);
        assert!(active.iter().any(|i| i.id == "c1"));
        assert!(active.iter().any(|i| i.id == "c3"));
        assert!(active.iter().all(|i| i.id != "c2"));
    }

    #[test]
    fn test_list_active_captures_excludes_snoozed_future() {
        let store = test_store();
        insert_capture(
            &store, "c1", "snoozed item",
            serde_json::json!({"status": "snoozed", "snoozed_until": "2099-01-01T00:00:00+00:00"}),
        );
        insert_capture(&store, "c2", "active item", serde_json::json!({"status": "inbox"}));

        let active = list_active_captures(&store).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "c2");
    }

    #[test]
    fn test_list_active_captures_includes_expired_snooze() {
        let store = test_store();
        insert_capture(
            &store, "c1", "expired snooze",
            serde_json::json!({"status": "snoozed", "snoozed_until": "2020-01-01T00:00:00+00:00"}),
        );

        let active = list_active_captures(&store).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "c1");
    }

    #[test]
    fn test_list_active_captures_handles_missing_status() {
        let store = test_store();
        // Capture with empty metadata — COALESCE defaults to 'inbox'
        insert_capture(&store, "c1", "no status", serde_json::json!({}));

        let active = list_active_captures(&store).unwrap();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_update_capture_status_to_done() {
        let store = test_store();
        insert_capture(&store, "c1", "task to finish", serde_json::json!({"status": "inbox"}));

        let updated = update_capture_status(&store, "c1", "done", None).unwrap();
        assert!(updated);

        let item = get_item(&store, "c1").unwrap().unwrap();
        assert_eq!(item.metadata["status"], "done");
    }

    #[test]
    fn test_update_capture_status_to_snoozed() {
        let store = test_store();
        insert_capture(&store, "c1", "snooze me", serde_json::json!({"status": "inbox"}));

        let updated = update_capture_status(
            &store, "c1", "snoozed", Some("2099-12-01T00:00:00+00:00"),
        ).unwrap();
        assert!(updated);

        let item = get_item(&store, "c1").unwrap().unwrap();
        assert_eq!(item.metadata["status"], "snoozed");
        assert_eq!(item.metadata["snoozed_until"], "2099-12-01T00:00:00+00:00");
    }

    #[test]
    fn test_update_nonexistent_capture_returns_false() {
        let store = test_store();
        let updated = update_capture_status(&store, "nonexistent", "done", None).unwrap();
        assert!(!updated);
    }

    #[test]
    fn test_update_does_not_affect_content_items() {
        let store = test_store();
        // Insert a content item (not capture)
        let tags: Vec<String> = vec!["rust".into()];
        let upsert = UpsertItem {
            id: "content-item-01",
            item_type: "content",
            title: "Rust Guide",
            source_url: Some("https://example.com"),
            content_type: Some("web"),
            tags: &tags,
            artifacts: &[],
            run_id: None,
            metadata: &serde_json::json!({}),
        };
        upsert_item(&store, &upsert).unwrap();

        // update_capture_status should NOT touch non-capture items
        let updated = update_capture_status(&store, "content-item-01", "done", None).unwrap();
        assert!(!updated);
    }

    // -- Prefix-ID resolution tests --

    #[test]
    fn test_resolve_capture_exact_id() {
        let store = test_store();
        insert_capture(&store, "abc12345def67890", "a task", serde_json::json!({"status": "inbox"}));

        let resolved = resolve_capture_id(&store, "abc12345def67890").unwrap();
        assert_eq!(resolved, "abc12345def67890");
    }

    #[test]
    fn test_resolve_capture_unique_prefix() {
        let store = test_store();
        insert_capture(&store, "abc12345def67890", "task A", serde_json::json!({"status": "inbox"}));
        insert_capture(&store, "xyz98765fed43210", "task B", serde_json::json!({"status": "inbox"}));

        let resolved = resolve_capture_id(&store, "abc123").unwrap();
        assert_eq!(resolved, "abc12345def67890");
    }

    #[test]
    fn test_resolve_capture_ambiguous_prefix() {
        let store = test_store();
        insert_capture(&store, "abc12345aaa00000", "task A", serde_json::json!({"status": "inbox"}));
        insert_capture(&store, "abc12345bbb11111", "task B", serde_json::json!({"status": "inbox"}));

        let result = resolve_capture_id(&store, "abc123");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Ambiguous"), "Error should mention ambiguity: {}", err_msg);
    }

    #[test]
    fn test_resolve_capture_nonexistent() {
        let store = test_store();
        insert_capture(&store, "abc12345def67890", "a task", serde_json::json!({"status": "inbox"}));

        let result = resolve_capture_id(&store, "zzz999");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No capture found"), "Error should say not found: {}", err_msg);
    }

    #[test]
    fn test_resolve_ignores_non_capture_items() {
        let store = test_store();
        // Insert a content item (not capture) with matching prefix
        let tags: Vec<String> = vec![];
        upsert_item(&store, &UpsertItem {
            id: "abc12345content1",
            item_type: "content",
            title: "A Content Item",
            source_url: Some("https://example.com"),
            content_type: Some("web"),
            tags: &tags, artifacts: &[], run_id: None,
            metadata: &serde_json::json!({}),
        }).unwrap();

        // Prefix "abc123" should NOT match the content item
        let result = resolve_capture_id(&store, "abc123");
        assert!(result.is_err());
    }
}

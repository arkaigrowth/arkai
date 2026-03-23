//! Hybrid search module combining FTS5 (BM25) keyword search with vector cosine similarity.
//!
//! Uses Reciprocal Rank Fusion (RRF) to merge rankings from both retrieval methods
//! into a single combined score.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;

use super::embedding::cosine_similarity;

/// Result of a hybrid or vector search operation.
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    pub item_id: String,
    pub title: String,
    pub fts_rank: Option<f64>,
    pub vector_score: Option<f64>,
    pub combined_score: f64,
    /// Best matching chunk snippet (if result came from chunk-level search).
    pub chunk_text: Option<String>,
    /// Chunk ID (if result came from chunk-level search).
    pub chunk_id: Option<String>,
}

/// Reciprocal Rank Fusion constant. Standard value from the original RRF paper.
const RRF_K: f64 = 60.0;

/// Compute a single RRF contribution for a given rank (1-based).
fn rrf_score(rank: usize) -> f64 {
    1.0 / (RRF_K + rank as f64)
}

/// Run hybrid search combining FTS5 BM25 ranking with vector cosine similarity.
///
/// 1. FTS5 search returns the top 50 keyword-matched results ranked by BM25.
/// 2. All stored embeddings are scanned and ranked by cosine similarity to
///    `query_embedding`, keeping the top 50.
/// 3. Results from both lists are merged using Reciprocal Rank Fusion.
/// 4. The top `limit` results by combined RRF score are returned.
pub fn hybrid_search(
    conn: &Connection,
    query_embedding: &[f32],
    query_text: &str,
    limit: usize,
) -> Result<Vec<HybridSearchResult>> {
    let fts_results = fts_search(conn, query_text, 50)?;
    let vec_results = raw_vector_search(conn, query_embedding, 50)?;

    let merged = merge_rrf(&fts_results, &vec_results, conn, limit)?;
    Ok(merged)
}

/// Pure vector search -- brute-force cosine similarity scan over all embeddings.
///
/// Suitable for collections under ~10K items. Returns results sorted by
/// descending cosine similarity.
pub fn vector_search(
    conn: &Connection,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<HybridSearchResult>> {
    let scored = raw_vector_search(conn, query_embedding, limit)?;

    let mut results: Vec<HybridSearchResult> = scored
        .into_iter()
        .map(|(item_id, score)| {
            let title = item_title(conn, &item_id).unwrap_or_default();
            HybridSearchResult {
                item_id,
                title,
                fts_rank: None,
                vector_score: Some(score),
                combined_score: score,
                chunk_text: None,
                chunk_id: None,
            }
        })
        .collect();

    results.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(limit);
    Ok(results)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Sanitize user input for safe use in FTS5 MATCH expressions.
///
/// FTS5 treats hyphens as negation, bare numbers as column references, and
/// characters like `:`, `*`, `(`, `)` as operators. Wrapping the query in
/// double-quote phrase syntax disables all operator interpretation.
///
/// Returns empty string for blank input (caller should skip FTS in that case).
pub(crate) fn sanitize_fts_query(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // Escape internal double-quotes by doubling them (FTS5 spec), then wrap.
    let escaped = trimmed.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

/// FTS5 search returning (item_id, title, bm25_rank) tuples, ordered by BM25.
fn fts_search(conn: &Connection, query_text: &str, limit: usize) -> Result<Vec<(String, String, f64)>> {
    let safe_query = sanitize_fts_query(query_text);
    if safe_query.is_empty() {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT i.id, i.title, rank
             FROM items_fts fts
             JOIN items i ON i.rowid = fts.rowid
             WHERE items_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )
        .context("preparing FTS5 search statement")?;

    let rows = stmt
        .query_map(rusqlite::params![safe_query, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })
        .context("executing FTS5 search")?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Scan all embeddings, compute cosine similarity, return top `limit` as (item_id, score).
fn raw_vector_search(
    conn: &Connection,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<(String, f64)>> {
    let mut stmt = conn
        .prepare("SELECT item_id, vector FROM embeddings")
        .context("preparing embedding scan statement")?;

    let rows = stmt
        .query_map([], |row| {
            let item_id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((item_id, blob))
        })
        .context("scanning embeddings")?;

    let mut scored: Vec<(String, f64)> = Vec::new();
    for row in rows {
        let (item_id, blob) = row?;
        let vector: Vec<f32> = blob
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let sim = cosine_similarity(query_embedding, &vector) as f64;
        scored.push((item_id, sim));
    }

    // Sort descending by similarity.
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    Ok(scored)
}

/// Look up an item's title by id. Returns empty string on miss.
fn item_title(conn: &Connection, item_id: &str) -> Result<String> {
    conn.query_row("SELECT title FROM items WHERE id = ?1", [item_id], |row| {
        row.get(0)
    })
    .context("fetching item title")
}

/// Merge FTS and vector result lists using Reciprocal Rank Fusion.
///
/// Each result set contributes `1 / (k + rank)` per item it contains.
/// Items appearing in both lists receive the sum of their individual RRF scores.
fn merge_rrf(
    fts_results: &[(String, String, f64)],
    vec_results: &[(String, f64)],
    conn: &Connection,
    limit: usize,
) -> Result<Vec<HybridSearchResult>> {
    // Track per-item data: (title, fts_rank_value, vector_score, rrf_sum).
    let mut items: HashMap<String, (String, Option<f64>, Option<f64>, f64)> = HashMap::new();

    // FTS contributions (already ordered by BM25 rank).
    for (rank_idx, (item_id, title, bm25)) in fts_results.iter().enumerate() {
        let rank = rank_idx + 1; // 1-based
        let entry = items
            .entry(item_id.clone())
            .or_insert_with(|| (title.clone(), None, None, 0.0));
        entry.1 = Some(*bm25);
        entry.3 += rrf_score(rank);
    }

    // Vector contributions (already ordered by cosine similarity descending).
    for (rank_idx, (item_id, sim)) in vec_results.iter().enumerate() {
        let rank = rank_idx + 1; // 1-based
        let entry = items
            .entry(item_id.clone())
            .or_insert_with(|| {
                let title = item_title(conn, item_id).unwrap_or_default();
                (title, None, None, 0.0)
            });
        entry.2 = Some(*sim);
        entry.3 += rrf_score(rank);
    }

    let mut results: Vec<HybridSearchResult> = items
        .into_iter()
        .map(|(item_id, (title, fts_rank, vector_score, rrf_sum))| HybridSearchResult {
            item_id,
            title,
            fts_rank,
            vector_score,
            combined_score: rrf_sum,
            chunk_text: None,
            chunk_id: None,
        })
        .collect();

    results.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(limit);
    Ok(results)
}

// ---------------------------------------------------------------------------
// Multi-level search (items + chunks merged)
// ---------------------------------------------------------------------------

/// Scaling factor for chunk-level RRF contributions relative to item-level.
///
/// At 0.5, the maximum chunk boost (rank-1) is ~0.008 while a strong title
/// match scores ~0.033 from hybrid_search. This ensures chunks enrich results
/// with snippets but cannot override a strong title/FTS match.
const CHUNK_WEIGHT: f64 = 0.5;

/// Search both item-level and chunk-level embeddings, then merge results.
///
/// Item-level results come from hybrid_search (FTS5 + item embeddings).
/// Chunk-level results come from chunk_vector_search (chunk embeddings).
///
/// Merging strategy:
/// 1. Run both searches independently
/// 2. For chunk results, group by parent item_id, keep highest-scoring chunk
/// 3. Preserve item-level hybrid scores (includes FTS title-match boost)
/// 4. Add scaled chunk RRF boost; attach best chunk snippet for display
pub fn multi_level_search(
    conn: &Connection,
    query_embedding: &[f32],
    query_text: &str,
    limit: usize,
) -> Result<Vec<HybridSearchResult>> {
    // Item-level hybrid search
    let item_results = hybrid_search(conn, query_embedding, query_text, 50)?;

    // Chunk-level vector search
    let chunk_results = chunk_vector_search(conn, query_embedding, 50)?;

    // If no chunk results, just return item results
    if chunk_results.is_empty() {
        let mut results = item_results;
        results.truncate(limit);
        return Ok(results);
    }

    // Build a map of item_id -> best chunk result (highest score)
    let mut best_chunk_per_item: HashMap<String, HybridSearchResult> = HashMap::new();
    for cr in chunk_results {
        let entry = best_chunk_per_item.entry(cr.item_id.clone());
        entry
            .and_modify(|existing| {
                if cr.combined_score > existing.combined_score {
                    *existing = cr.clone();
                }
            })
            .or_insert(cr);
    }

    // Merge: start with item results, preserve their hybrid scores
    let mut merged: HashMap<String, HybridSearchResult> = HashMap::new();

    // Add item-level results — keep original combined_score from hybrid_search
    for r in item_results.iter() {
        merged.insert(
            r.item_id.clone(),
            HybridSearchResult {
                item_id: r.item_id.clone(),
                title: r.title.clone(),
                fts_rank: r.fts_rank,
                vector_score: r.vector_score,
                combined_score: r.combined_score,
                chunk_text: None,
                chunk_id: None,
            },
        );
    }

    // Sort chunks by score DESC then item_id ASC for deterministic ranking
    let mut sorted_chunks: Vec<(&String, &HybridSearchResult)> =
        best_chunk_per_item.iter().collect();
    sorted_chunks.sort_by(|a, b| {
        b.1.combined_score
            .partial_cmp(&a.1.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });

    // Add/boost with chunk results (scaled by CHUNK_WEIGHT)
    for (rank, (item_id, chunk_result)) in sorted_chunks.iter().enumerate() {
        let chunk_rrf = rrf_score(rank + 1) * CHUNK_WEIGHT;
        let entry = merged.entry((*item_id).clone());
        entry
            .and_modify(|existing| {
                existing.combined_score += chunk_rrf;
                existing.chunk_text = chunk_result.chunk_text.clone();
                existing.chunk_id = chunk_result.chunk_id.clone();
            })
            .or_insert_with(|| {
                let title = item_title(conn, item_id).unwrap_or_default();
                HybridSearchResult {
                    item_id: (*item_id).clone(),
                    title,
                    fts_rank: None,
                    vector_score: chunk_result.vector_score,
                    combined_score: chunk_rrf,
                    chunk_text: chunk_result.chunk_text.clone(),
                    chunk_id: chunk_result.chunk_id.clone(),
                }
            });
    }

    let mut results: Vec<HybridSearchResult> = merged.into_values().collect();
    results.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(limit);
    Ok(results)
}

// ---------------------------------------------------------------------------
// Chunk-level vector search
// ---------------------------------------------------------------------------

/// Scan all chunk embeddings, compute cosine similarity, return top `limit`
/// results with chunk text and parent item info.
pub fn chunk_vector_search(
    conn: &Connection,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<HybridSearchResult>> {
    let mut stmt = conn
        .prepare(
            "SELECT ce.chunk_id, c.item_id, c.text, ce.vector
             FROM chunk_embeddings ce
             JOIN chunks c ON c.id = ce.chunk_id",
        )
        .context("preparing chunk embedding scan statement")?;

    let rows = stmt
        .query_map([], |row| {
            let chunk_id: String = row.get(0)?;
            let item_id: String = row.get(1)?;
            let chunk_text: String = row.get(2)?;
            let blob: Vec<u8> = row.get(3)?;
            Ok((chunk_id, item_id, chunk_text, blob))
        })
        .context("scanning chunk embeddings")?;

    let mut scored: Vec<(String, String, String, f64)> = Vec::new();
    for row in rows {
        let (chunk_id, item_id, chunk_text, blob) = row?;
        let vector: Vec<f32> = blob
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let sim = cosine_similarity(query_embedding, &vector) as f64;
        scored.push((chunk_id, item_id, chunk_text, sim));
    }

    scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    let results: Vec<HybridSearchResult> = scored
        .into_iter()
        .map(|(chunk_id, item_id, chunk_text, sim)| {
            let title = item_title(conn, &item_id).unwrap_or_default();
            HybridSearchResult {
                item_id,
                title,
                fts_rank: None,
                vector_score: Some(sim),
                combined_score: sim,
                chunk_text: Some(chunk_text),
                chunk_id: Some(chunk_id),
            }
        })
        .collect();

    Ok(results)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: set up an in-memory SQLite database with the required schema.
    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE items (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                tags TEXT DEFAULT '',
                metadata TEXT DEFAULT ''
            );
            CREATE VIRTUAL TABLE items_fts USING fts5(
                title, tags, metadata, content=items, content_rowid=rowid
            );
            -- Triggers to keep FTS in sync (matches real migration)
            CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
                INSERT INTO items_fts(rowid, title, tags, metadata)
                VALUES (new.rowid, new.title, new.tags, new.metadata);
            END;
            CREATE TABLE embeddings (
                item_id TEXT PRIMARY KEY,
                model TEXT NOT NULL,
                dimensions INTEGER NOT NULL,
                vector BLOB NOT NULL
            );",
        )
        .unwrap();
        conn
    }

    /// Helper: insert an item (FTS auto-synced via trigger).
    fn insert_item(conn: &Connection, id: &str, title: &str, tags: &str) {
        conn.execute(
            "INSERT INTO items (id, title, tags, metadata) VALUES (?1, ?2, ?3, '')",
            rusqlite::params![id, title, tags],
        )
        .unwrap();
    }

    /// Helper: insert an embedding for an item.
    fn insert_embedding(conn: &Connection, item_id: &str, vector: &[f32]) {
        let blob: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
        conn.execute(
            "INSERT INTO embeddings (item_id, model, dimensions, vector) VALUES (?1, 'test', ?2, ?3)",
            rusqlite::params![item_id, vector.len() as i64, blob],
        )
        .unwrap();
    }

    #[test]
    fn test_rrf_score_known_values() {
        // rank=1, k=60 => 1/61
        let s1 = rrf_score(1);
        assert!((s1 - 1.0 / 61.0).abs() < 1e-10);

        // rank=2, k=60 => 1/62
        let s2 = rrf_score(2);
        assert!((s2 - 1.0 / 62.0).abs() < 1e-10);

        // rank=10, k=60 => 1/70
        let s10 = rrf_score(10);
        assert!((s10 - 1.0 / 70.0).abs() < 1e-10);

        // Scores must be monotonically decreasing.
        assert!(s1 > s2);
        assert!(s2 > s10);
    }

    #[test]
    fn test_rrf_merge_both_lists() {
        // Item appears in both FTS rank 1 and vector rank 1 => RRF = 2/61.
        // Item appears in only FTS rank 2 => RRF = 1/62.
        let conn = setup_db();
        insert_item(&conn, "a", "Alpha", "rust");
        insert_item(&conn, "b", "Beta", "python");

        let fts = vec![
            ("a".to_string(), "Alpha".to_string(), -1.0),
            ("b".to_string(), "Beta".to_string(), -2.0),
        ];
        let vec_results = vec![("a".to_string(), 0.95)];

        let merged = merge_rrf(&fts, &vec_results, &conn, 10).unwrap();

        assert_eq!(merged.len(), 2);
        // "a" should be first (appears in both lists).
        assert_eq!(merged[0].item_id, "a");
        let expected_a = rrf_score(1) + rrf_score(1); // FTS rank 1 + vec rank 1
        assert!((merged[0].combined_score - expected_a).abs() < 1e-10);

        // "b" only in FTS rank 2.
        assert_eq!(merged[1].item_id, "b");
        let expected_b = rrf_score(2);
        assert!((merged[1].combined_score - expected_b).abs() < 1e-10);
    }

    #[test]
    fn test_hybrid_search_returns_results() {
        let conn = setup_db();
        insert_item(&conn, "item1", "Rust programming", "rust systems");
        insert_item(&conn, "item2", "Python scripting", "python automation");
        insert_item(&conn, "item3", "Go concurrency", "go goroutines");

        // Embeddings: item1 and item3 have embeddings, item2 does not.
        insert_embedding(&conn, "item1", &[1.0, 0.0, 0.0]);
        insert_embedding(&conn, "item3", &[0.0, 1.0, 0.0]);

        // Query: text matches "rust", embedding is close to item1.
        let query_vec = [0.9_f32, 0.1, 0.0];
        let results = hybrid_search(&conn, &query_vec, "rust", 10).unwrap();

        // item1 should appear (matches both FTS and vector).
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.item_id == "item1"));

        // item1 should have both scores populated.
        let item1 = results.iter().find(|r| r.item_id == "item1").unwrap();
        assert!(item1.fts_rank.is_some());
        assert!(item1.vector_score.is_some());
    }

    #[test]
    fn test_vector_only_search() {
        let conn = setup_db();
        insert_item(&conn, "v1", "Doc A", "");
        insert_item(&conn, "v2", "Doc B", "");
        insert_item(&conn, "v3", "Doc C", "");

        insert_embedding(&conn, "v1", &[1.0, 0.0, 0.0]);
        insert_embedding(&conn, "v2", &[0.0, 1.0, 0.0]);
        insert_embedding(&conn, "v3", &[0.707, 0.707, 0.0]);

        // Query close to v1.
        let query = [0.95_f32, 0.05, 0.0];
        let results = vector_search(&conn, &query, 10).unwrap();

        assert_eq!(results.len(), 3);
        // v1 should be the top result (closest to query).
        assert_eq!(results[0].item_id, "v1");
        assert!(results[0].vector_score.unwrap() > results[1].vector_score.unwrap());
        // All should have no fts_rank.
        for r in &results {
            assert!(r.fts_rank.is_none());
        }
    }

    #[test]
    fn test_empty_results() {
        let conn = setup_db();

        // No items at all.
        let query_vec = [1.0_f32, 0.0, 0.0];
        let results = hybrid_search(&conn, &query_vec, "nonexistent", 10).unwrap();
        assert!(results.is_empty());

        let results = vector_search(&conn, &query_vec, 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_query_text() {
        let conn = setup_db();
        insert_item(&conn, "e1", "Something", "tag");
        insert_embedding(&conn, "e1", &[1.0, 0.0]);

        // Empty text query should still return vector results.
        let query_vec = [0.9_f32, 0.1];
        let results = hybrid_search(&conn, &query_vec, "", 10).unwrap();
        assert!(!results.is_empty());
        // Only vector score should be present since FTS was skipped.
        assert!(results[0].fts_rank.is_none());
        assert!(results[0].vector_score.is_some());
    }

    /// Helper: set up a database with chunks + chunk_embeddings tables.
    fn setup_db_with_chunks() -> Connection {
        let conn = setup_db();
        conn.execute_batch(
            "CREATE TABLE chunks (
                id TEXT PRIMARY KEY,
                item_id TEXT NOT NULL REFERENCES items(id),
                chunk_index INTEGER NOT NULL,
                text TEXT NOT NULL,
                byte_start INTEGER NOT NULL,
                byte_end INTEGER NOT NULL,
                word_count INTEGER NOT NULL,
                metadata TEXT DEFAULT '{}',
                UNIQUE(item_id, chunk_index)
            );
            CREATE TABLE chunk_embeddings (
                chunk_id TEXT PRIMARY KEY REFERENCES chunks(id),
                model TEXT NOT NULL,
                dimensions INTEGER NOT NULL,
                vector BLOB NOT NULL,
                created_at TEXT NOT NULL
            );",
        )
        .unwrap();
        conn
    }

    /// Helper: insert a chunk with embedding.
    fn insert_chunk_with_embedding(
        conn: &Connection,
        chunk_id: &str,
        item_id: &str,
        chunk_index: i64,
        text: &str,
        vector: &[f32],
    ) {
        conn.execute(
            "INSERT INTO chunks (id, item_id, chunk_index, text, byte_start, byte_end, word_count)
             VALUES (?1, ?2, ?3, ?4, 0, 100, 50)",
            rusqlite::params![chunk_id, item_id, chunk_index, text],
        )
        .unwrap();
        let blob: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
        conn.execute(
            "INSERT INTO chunk_embeddings (chunk_id, model, dimensions, vector, created_at)
             VALUES (?1, 'test', ?2, ?3, '2026-01-01')",
            rusqlite::params![chunk_id, vector.len() as i64, blob],
        )
        .unwrap();
    }

    #[test]
    fn test_multi_level_search_combines_items_and_chunks() {
        let conn = setup_db_with_chunks();

        // Item with title-level embedding only
        insert_item(&conn, "title-only", "Rust Programming Guide", "rust");
        insert_embedding(&conn, "title-only", &[1.0, 0.0, 0.0]);

        // Item with both title embedding and chunk about pizza
        insert_item(&conn, "has-chunk", "Some Video Title", "video");
        insert_embedding(&conn, "has-chunk", &[0.0, 1.0, 0.0]);
        insert_chunk_with_embedding(
            &conn,
            "chunk-pizza",
            "has-chunk",
            0,
            "The pizza shop owner told an amazing story about automation",
            &[0.9, 0.1, 0.0], // close to query
        );

        // Query vector close to the pizza chunk
        let query_vec = [0.85_f32, 0.15, 0.0];
        let results =
            multi_level_search(&conn, &query_vec, "pizza", 10).unwrap();

        // The item with the pizza chunk should appear
        assert!(!results.is_empty());
        let pizza_result = results.iter().find(|r| r.item_id == "has-chunk");
        assert!(
            pizza_result.is_some(),
            "Item with matching chunk should appear"
        );
        let pizza = pizza_result.unwrap();
        assert!(pizza.chunk_text.is_some(), "Should have chunk snippet");
        assert!(
            pizza
                .chunk_text
                .as_ref()
                .unwrap()
                .contains("pizza shop"),
            "Chunk text should contain 'pizza shop'"
        );
    }

    #[test]
    fn test_multi_level_search_without_chunks_falls_back() {
        let conn = setup_db_with_chunks();

        insert_item(&conn, "item1", "Rust Guide", "rust");
        insert_embedding(&conn, "item1", &[1.0, 0.0, 0.0]);

        // No chunks at all — should fall back to item-level
        let query_vec = [0.9_f32, 0.1, 0.0];
        let results = multi_level_search(&conn, &query_vec, "rust", 10).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].item_id, "item1");
        assert!(results[0].chunk_text.is_none());
    }

    #[test]
    fn test_multi_level_search_boosts_chunk_match() {
        let conn = setup_db_with_chunks();

        // Item A: high item-level vector match, no chunks
        insert_item(&conn, "item-a", "High Match Title", "");
        insert_embedding(&conn, "item-a", &[0.9, 0.1, 0.0]);

        // Item B: lower item-level match, but chunk is very close
        insert_item(&conn, "item-b", "Low Match Title", "");
        insert_embedding(&conn, "item-b", &[0.1, 0.9, 0.0]);
        insert_chunk_with_embedding(
            &conn,
            "chunk-b1",
            "item-b",
            0,
            "Exact topic match content here",
            &[0.95, 0.05, 0.0], // very close to query
        );

        let query_vec = [0.9_f32, 0.1, 0.0];
        let results = multi_level_search(&conn, &query_vec, "", 10).unwrap();

        // Item B should be boosted by its chunk match
        assert!(results.len() >= 2);
        // Item B should appear in results (boosted by chunk)
        assert!(results.iter().any(|r| r.item_id == "item-b"));
    }

    #[test]
    fn test_multi_level_preserves_exact_title_match() {
        let conn = setup_db_with_chunks();

        // Item with exact title match + strong embedding
        insert_item(&conn, "target", "Building AI Assistants", "ai llm");
        insert_embedding(&conn, "target", &[1.0, 0.0, 0.0]);

        // Noise items with high-cosine chunks that should NOT outrank title match
        insert_item(&conn, "noise1", "Unrelated Topic A", "");
        insert_embedding(&conn, "noise1", &[0.5, 0.5, 0.0]);
        insert_chunk_with_embedding(
            &conn, "cn1a", "noise1", 0,
            "something about building assistants tangentially",
            &[0.95, 0.05, 0.0],
        );
        insert_chunk_with_embedding(
            &conn, "cn1b", "noise1", 1,
            "more tangentially related content here",
            &[0.92, 0.08, 0.0],
        );

        insert_item(&conn, "noise2", "Unrelated Topic B", "");
        insert_embedding(&conn, "noise2", &[0.4, 0.6, 0.0]);
        insert_chunk_with_embedding(
            &conn, "cn2a", "noise2", 0,
            "yet another chunk close to the query vector",
            &[0.90, 0.10, 0.0],
        );

        let query_vec = [1.0_f32, 0.0, 0.0];
        let results = multi_level_search(&conn, &query_vec, "Building AI Assistants", 10).unwrap();

        assert!(!results.is_empty());
        assert_eq!(
            results[0].item_id, "target",
            "Exact title match must be top-1, got: {:?}",
            results.iter().map(|r| (&r.item_id, r.combined_score)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_multi_level_relevant_title_not_buried() {
        // Models the "job displacement" failure: a clearly relevant title item
        // should not get buried by chunk-only distractors.
        let conn = setup_db_with_chunks();

        // Relevant item — title clearly matches query
        insert_item(&conn, "relevant", "85 Percent Unemployable AI Job Loss", "ai automation");
        insert_embedding(&conn, "relevant", &[0.8, 0.2, 0.0]);

        // Distractor with lots of chunks that are somewhat related
        insert_item(&conn, "distractor", "Random Tech Talk", "tech");
        insert_embedding(&conn, "distractor", &[0.3, 0.7, 0.0]);
        for i in 0..5 {
            let cid = format!("dc{}", i);
            insert_chunk_with_embedding(
                &conn, &cid, "distractor", i,
                "automation and job market discussion content filler text",
                &[0.75, 0.25, 0.0],
            );
        }

        let query_vec = [0.8_f32, 0.2, 0.0];
        let results = multi_level_search(
            &conn, &query_vec, "unemployable AI job displacement", 10,
        ).unwrap();

        assert!(!results.is_empty());
        // The relevant-title item must be in top 2 (not buried)
        let relevant_pos = results.iter().position(|r| r.item_id == "relevant");
        assert!(
            relevant_pos.is_some() && relevant_pos.unwrap() < 2,
            "Relevant title item should be in top 2, pos={:?}, results: {:?}",
            relevant_pos,
            results.iter().map(|r| (&r.item_id, r.combined_score)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_limit_respected() {
        let conn = setup_db();
        for i in 0..20 {
            let id = format!("item{i}");
            let title = format!("Document {i}");
            insert_item(&conn, &id, &title, "common");
            insert_embedding(&conn, &id, &[i as f32, 0.0, 1.0]);
        }

        let query_vec = [10.0_f32, 0.0, 1.0];
        let results = hybrid_search(&conn, &query_vec, "common", 5).unwrap();
        assert!(results.len() <= 5);

        let results = vector_search(&conn, &query_vec, 3).unwrap();
        assert!(results.len() <= 3);
    }

    // -- FTS sanitization tests --

    #[test]
    fn test_sanitize_fts_query_wraps_simple_term() {
        assert_eq!(sanitize_fts_query("rust"), "\"rust\"");
    }

    #[test]
    fn test_sanitize_fts_query_handles_date() {
        assert_eq!(sanitize_fts_query("2026-03-15"), "\"2026-03-15\"");
    }

    #[test]
    fn test_sanitize_fts_query_escapes_double_quotes() {
        assert_eq!(
            sanitize_fts_query("say \"hello\""),
            "\"say \"\"hello\"\"\""
        );
    }

    #[test]
    fn test_sanitize_fts_query_trims_and_empty() {
        assert_eq!(sanitize_fts_query(""), "");
        assert_eq!(sanitize_fts_query("   "), "");
        assert_eq!(sanitize_fts_query("  rust  "), "\"rust\"");
    }

    #[test]
    fn test_fts_query_with_date_text() {
        let conn = setup_db();
        insert_item(&conn, "d1", "Meeting notes 2026-03-15", "");
        // Must not error — date-like text previously caused "no such column: 03"
        let results = fts_search(&conn, "2026-03-15", 10);
        assert!(results.is_ok());
    }

    #[test]
    fn test_fts_query_with_special_chars() {
        let conn = setup_db();
        insert_item(&conn, "s1", "config key value important", "");
        // Colons, parens, asterisks must not crash FTS5
        assert!(fts_search(&conn, "key: value", 10).is_ok());
        assert!(fts_search(&conn, "(important)", 10).is_ok());
        assert!(fts_search(&conn, "important*", 10).is_ok());
        assert!(fts_search(&conn, "cold-outreach", 10).is_ok());
    }
}

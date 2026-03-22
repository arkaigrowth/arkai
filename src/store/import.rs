//! Catalog-to-store import module.
//!
//! One-time import of existing arkai data (catalog.json and library metadata)
//! into the SQLite store. Idempotent via upsert semantics.

use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{info, warn};

use super::db::Store;
use super::queries::{upsert_item, UpsertItem};

// ─────────────────────────────────────────────────────────────────
// Import stats
// ─────────────────────────────────────────────────────────────────

/// Statistics from an import operation.
#[derive(Debug, Clone, Default)]
pub struct ImportStats {
    /// Items successfully imported (inserted or updated).
    pub imported: usize,
    /// Items skipped (e.g. missing required fields).
    pub skipped: usize,
    /// Items that failed to import.
    pub errors: usize,
}

impl ImportStats {
    /// Merge another stats batch into this one.
    pub fn merge(&mut self, other: &ImportStats) {
        self.imported += other.imported;
        self.skipped += other.skipped;
        self.errors += other.errors;
    }

    /// Total items processed.
    pub fn total(&self) -> usize {
        self.imported + self.skipped + self.errors
    }
}

impl fmt::Display for ImportStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "imported: {}, skipped: {}, errors: {} (total: {})",
            self.imported,
            self.skipped,
            self.errors,
            self.total(),
        )
    }
}

// ─────────────────────────────────────────────────────────────────
// Catalog JSON format (deserialization only)
// ─────────────────────────────────────────────────────────────────

/// Top-level catalog.json structure.
#[derive(Debug, Deserialize)]
struct CatalogFile {
    #[allow(dead_code)]
    version: u32,
    items: Vec<CatalogEntry>,
}

/// A single entry in catalog.json.
#[derive(Debug, Deserialize)]
struct CatalogEntry {
    id: String,
    title: String,
    url: String,
    content_type: String,
    #[allow(dead_code)]
    processed_at: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    artifacts: Vec<String>,
    run_id: Option<String>,
}

// ─────────────────────────────────────────────────────────────────
// Library metadata.json format (deserialization only)
// ─────────────────────────────────────────────────────────────────

/// metadata.json — handles both arkai catalog format and Whisper pipeline format.
///
/// Arkai format:  { id, title, url, content_type, processed_at, tags }
/// Whisper format: { id, title, url, source, duration, transcription_model, ... }
#[derive(Debug, Deserialize)]
struct LibraryMetadata {
    id: String,
    title: String,
    url: String,
    /// Arkai catalog format uses "content_type"
    content_type: Option<String>,
    /// Whisper pipeline format uses "source" (e.g., "youtube")
    source: Option<String>,
    #[allow(dead_code)]
    processed_at: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    /// Whisper pipeline metadata
    duration: Option<String>,
    word_count: Option<u64>,
    transcription_model: Option<String>,
}

/// claims.json top-level structure.
#[derive(Debug, Deserialize)]
struct ClaimsFile {
    #[serde(default)]
    claims: Vec<serde_json::Value>,
}

// ─────────────────────────────────────────────────────────────────
// import_catalog
// ─────────────────────────────────────────────────────────────────

/// Import items from an existing catalog.json file into the store.
///
/// Each catalog entry is upserted with `item_type = "content"`.
/// Entries with empty `id` or `title` are skipped.
/// Returns aggregate statistics for the import run.
pub fn import_catalog(store: &Store, catalog_path: &Path) -> Result<ImportStats> {
    let mut stats = ImportStats::default();

    let raw = fs::read_to_string(catalog_path)
        .with_context(|| format!("Failed to read catalog: {}", catalog_path.display()))?;

    let catalog: CatalogFile =
        serde_json::from_str(&raw).context("Failed to parse catalog.json")?;

    info!(
        items = catalog.items.len(),
        "importing catalog entries into store"
    );

    for entry in &catalog.items {
        if entry.id.is_empty() || entry.title.is_empty() {
            warn!(id = %entry.id, title = %entry.title, "skipping entry with empty id or title");
            stats.skipped += 1;
            continue;
        }

        let content_type_normalized = normalize_content_type(&entry.content_type);
        let metadata = serde_json::json!({});

        let upsert = UpsertItem {
            id: &entry.id,
            item_type: "content",
            title: &entry.title,
            source_url: Some(entry.url.as_str()),
            content_type: Some(content_type_normalized.as_str()),
            tags: &entry.tags,
            artifacts: &entry.artifacts,
            run_id: entry.run_id.as_deref(),
            metadata: &metadata,
        };

        match upsert_item(store, &upsert) {
            Ok(_) => {
                stats.imported += 1;
            }
            Err(e) => {
                warn!(id = %entry.id, error = %e, "failed to import catalog entry");
                stats.errors += 1;
            }
        }
    }

    info!(%stats, "catalog import complete");
    Ok(stats)
}

// ─────────────────────────────────────────────────────────────────
// import_library_metadata
// ─────────────────────────────────────────────────────────────────

/// Import metadata from library/youtube/*/ directories into the store.
///
/// For each subdirectory containing a metadata.json, the item is upserted
/// with `item_type = "content"`. Discovered artifacts (summary.md, wisdom.md,
/// etc.) are listed. If claims.json exists, the claim count is stored in
/// the item metadata.
pub fn import_library_metadata(store: &Store, library_path: &Path) -> Result<ImportStats> {
    let mut stats = ImportStats::default();

    let youtube_dir = library_path.join("youtube");
    if !youtube_dir.is_dir() {
        info!(path = %youtube_dir.display(), "youtube directory not found, nothing to import");
        return Ok(stats);
    }

    let entries: Vec<_> = fs::read_dir(&youtube_dir)
        .with_context(|| format!("Failed to read directory: {}", youtube_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    info!(dirs = entries.len(), "scanning library youtube directories");

    for entry in &entries {
        let dir = entry.path();
        let metadata_path = dir.join("metadata.json");

        if !metadata_path.is_file() {
            warn!(dir = %dir.display(), "no metadata.json found, skipping");
            stats.skipped += 1;
            continue;
        }

        match import_single_library_dir(store, &dir, &metadata_path) {
            Ok(()) => {
                stats.imported += 1;
            }
            Err(e) => {
                warn!(dir = %dir.display(), error = %e, "failed to import library entry");
                stats.errors += 1;
            }
        }
    }

    info!(%stats, "library metadata import complete");
    Ok(stats)
}

/// Import a single library directory into the store.
fn import_single_library_dir(store: &Store, dir: &Path, metadata_path: &Path) -> Result<()> {
    let raw = fs::read_to_string(metadata_path)
        .with_context(|| format!("Failed to read: {}", metadata_path.display()))?;

    let meta: LibraryMetadata = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse: {}", metadata_path.display()))?;

    // Discover artifacts present on disk
    let artifacts = discover_artifacts(dir);

    // Build metadata object; include claims count if claims.json exists
    let mut item_metadata = serde_json::json!({
        "library_path": dir.to_string_lossy()
    });
    let claims_path = dir.join("claims.json");
    if claims_path.is_file() {
        if let Ok(claims_raw) = fs::read_to_string(&claims_path) {
            if let Ok(claims_file) = serde_json::from_str::<ClaimsFile>(&claims_raw) {
                item_metadata["claims_count"] = serde_json::json!(claims_file.claims.len());
            }
        }
    }

    // Resolve content_type from either format
    let raw_type = meta
        .content_type
        .as_deref()
        .or(meta.source.as_deref())
        .unwrap_or("other");
    let content_type_normalized = normalize_content_type(raw_type);

    // Include Whisper pipeline metadata if present
    if let Some(duration) = &meta.duration {
        item_metadata["duration"] = serde_json::json!(duration);
    }
    if let Some(wc) = meta.word_count {
        item_metadata["word_count"] = serde_json::json!(wc);
    }
    if let Some(model) = &meta.transcription_model {
        item_metadata["transcription_model"] = serde_json::json!(model);
    }

    let upsert = UpsertItem {
        id: &meta.id,
        item_type: "content",
        title: &meta.title,
        source_url: Some(meta.url.as_str()),
        content_type: Some(content_type_normalized.as_str()),
        tags: &meta.tags,
        artifacts: &artifacts,
        run_id: None,
        metadata: &item_metadata,
    };

    upsert_item(store, &upsert)
        .with_context(|| format!("Failed to upsert library item: {}", meta.id))?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────

/// Discover artifact files in a library content directory.
///
/// Scans for known artifact extensions (.md, .json, .jsonl, .txt),
/// excludes metadata.json, and returns sorted filenames.
fn discover_artifacts(dir: &Path) -> Vec<String> {
    let mut artifacts = Vec::new();

    let Ok(entries) = fs::read_dir(dir) else {
        return artifacts;
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        // Skip metadata.json itself — it's structural, not an artifact
        if name == "metadata.json" {
            continue;
        }

        if name.ends_with(".md")
            || name.ends_with(".json")
            || name.ends_with(".jsonl")
            || name.ends_with(".txt")
        {
            artifacts.push(name.to_string());
        }
    }

    artifacts.sort();
    artifacts
}

/// Normalize content_type strings from various serialization formats.
///
/// Handles serde snake_case ("you_tube"), display form ("YouTube"),
/// and shorthand ("yt").
fn normalize_content_type(raw: &str) -> String {
    match raw.to_lowercase().as_str() {
        "youtube" | "you_tube" | "yt" => "youtube".to_string(),
        "web" | "webpage" | "article" => "web".to_string(),
        other => other.to_string(),
    }
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::queries;
    use tempfile::TempDir;

    fn test_store() -> Store {
        Store::open_memory().expect("failed to open in-memory store")
    }

    // -- ImportStats tests --

    #[test]
    fn test_import_stats_display() {
        let stats = ImportStats {
            imported: 10,
            skipped: 2,
            errors: 1,
        };
        assert_eq!(
            format!("{}", stats),
            "imported: 10, skipped: 2, errors: 1 (total: 13)"
        );
    }

    #[test]
    fn test_import_stats_default() {
        let stats = ImportStats::default();
        assert_eq!(stats.imported, 0);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.errors, 0);
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn test_import_stats_merge() {
        let mut a = ImportStats {
            imported: 5,
            skipped: 1,
            errors: 0,
        };
        let b = ImportStats {
            imported: 3,
            skipped: 2,
            errors: 1,
        };
        a.merge(&b);
        assert_eq!(a.imported, 8);
        assert_eq!(a.skipped, 3);
        assert_eq!(a.errors, 1);
        assert_eq!(a.total(), 12);
    }

    // -- import_catalog tests --

    #[test]
    fn test_import_catalog_basic() {
        let store = test_store();
        let dir = TempDir::new().unwrap();
        let catalog_path = dir.path().join("catalog.json");

        let catalog_json = r#"{
            "version": 1,
            "items": [
                {
                    "id": "abc123def456ab",
                    "title": "Test Video",
                    "url": "https://youtube.com/watch?v=test",
                    "content_type": "YouTube",
                    "processed_at": "2026-01-15T10:00:00Z",
                    "tags": ["test", "video"],
                    "artifacts": ["transcript.md"],
                    "run_id": "run-001"
                },
                {
                    "id": "xyz789xyz789xy",
                    "title": "Test Article",
                    "url": "https://example.com/article",
                    "content_type": "Web",
                    "processed_at": "2026-01-16T12:00:00Z",
                    "tags": [],
                    "artifacts": []
                }
            ]
        }"#;

        fs::write(&catalog_path, catalog_json).unwrap();

        let stats = import_catalog(&store, &catalog_path).unwrap();
        assert_eq!(stats.imported, 2);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.errors, 0);

        // Verify items are in store
        let item = queries::get_item(&store, "abc123def456ab")
            .unwrap()
            .unwrap();
        assert_eq!(item.title, "Test Video");
        assert_eq!(item.tags, vec!["test", "video"]);
        assert_eq!(item.content_type, Some("youtube".to_string()));
    }

    #[test]
    fn test_import_catalog_skips_empty_id() {
        let store = test_store();
        let dir = TempDir::new().unwrap();
        let catalog_path = dir.path().join("catalog.json");

        let catalog_json = r#"{
            "version": 1,
            "items": [
                {
                    "id": "",
                    "title": "No ID",
                    "url": "https://example.com",
                    "content_type": "web",
                    "processed_at": "2026-01-15T10:00:00Z"
                },
                {
                    "id": "valid123",
                    "title": "Valid Item",
                    "url": "https://example.com/valid",
                    "content_type": "web",
                    "processed_at": "2026-01-15T10:00:00Z"
                }
            ]
        }"#;

        fs::write(&catalog_path, catalog_json).unwrap();
        let stats = import_catalog(&store, &catalog_path).unwrap();

        assert_eq!(stats.imported, 1);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn test_import_catalog_skips_empty_title() {
        let store = test_store();
        let dir = TempDir::new().unwrap();
        let catalog_path = dir.path().join("catalog.json");

        let catalog_json = r#"{
            "version": 1,
            "items": [{
                "id": "has_id_no_title",
                "title": "",
                "url": "https://example.com",
                "content_type": "web",
                "processed_at": "2026-01-15T10:00:00Z"
            }]
        }"#;

        fs::write(&catalog_path, catalog_json).unwrap();
        let stats = import_catalog(&store, &catalog_path).unwrap();

        assert_eq!(stats.imported, 0);
        assert_eq!(stats.skipped, 1);
    }

    #[test]
    fn test_import_catalog_empty_items() {
        let store = test_store();
        let dir = TempDir::new().unwrap();
        let catalog_path = dir.path().join("catalog.json");

        fs::write(&catalog_path, r#"{"version": 1, "items": []}"#).unwrap();
        let stats = import_catalog(&store, &catalog_path).unwrap();

        assert_eq!(stats.imported, 0);
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn test_import_catalog_idempotent() {
        let store = test_store();
        let dir = TempDir::new().unwrap();
        let catalog_path = dir.path().join("catalog.json");

        let catalog_json = r#"{
            "version": 1,
            "items": [{
                "id": "idempotent01",
                "title": "Same Item",
                "url": "https://example.com/same",
                "content_type": "youtube",
                "processed_at": "2026-01-15T10:00:00Z",
                "tags": ["test"],
                "artifacts": []
            }]
        }"#;

        fs::write(&catalog_path, catalog_json).unwrap();

        let stats1 = import_catalog(&store, &catalog_path).unwrap();
        assert_eq!(stats1.imported, 1);

        // Import again -- upsert should succeed without error
        let stats2 = import_catalog(&store, &catalog_path).unwrap();
        assert_eq!(stats2.imported, 1);
        assert_eq!(stats2.errors, 0);
    }

    #[test]
    fn test_import_catalog_missing_file() {
        let store = test_store();
        let result = import_catalog(&store, Path::new("/nonexistent/catalog.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_import_catalog_normalizes_you_tube() {
        let store = test_store();
        let dir = TempDir::new().unwrap();
        let catalog_path = dir.path().join("catalog.json");

        // Uses "you_tube" (serde snake_case) instead of "youtube"
        let catalog_json = r#"{
            "version": 1,
            "items": [{
                "id": "serde_compat01",
                "title": "Serde Format Video",
                "url": "https://youtube.com/watch?v=serde",
                "content_type": "you_tube",
                "processed_at": "2026-01-15T10:00:00Z"
            }]
        }"#;

        fs::write(&catalog_path, catalog_json).unwrap();
        let stats = import_catalog(&store, &catalog_path).unwrap();

        assert_eq!(stats.imported, 1);
        assert_eq!(stats.errors, 0);

        let item = queries::get_item(&store, "serde_compat01")
            .unwrap()
            .unwrap();
        assert_eq!(item.content_type, Some("youtube".to_string()));
    }

    // -- import_library_metadata tests --

    #[test]
    fn test_import_library_metadata_basic() {
        let store = test_store();
        let dir = TempDir::new().unwrap();

        let video_dir = dir.path().join("youtube").join("Some Video (abc123)");
        fs::create_dir_all(&video_dir).unwrap();

        let metadata = r#"{
            "id": "lib_meta_001",
            "title": "Some Video",
            "url": "https://youtu.be/abc123",
            "content_type": "you_tube",
            "processed_at": "2026-01-20T08:00:00Z",
            "tags": ["ai"]
        }"#;
        fs::write(video_dir.join("metadata.json"), metadata).unwrap();
        fs::write(video_dir.join("summary.md"), "# Summary").unwrap();
        fs::write(video_dir.join("wisdom.md"), "# Wisdom").unwrap();
        fs::write(video_dir.join("transcript.txt"), "hello world").unwrap();

        let stats = import_library_metadata(&store, dir.path()).unwrap();

        assert_eq!(stats.imported, 1);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.errors, 0);

        let item = queries::get_item(&store, "lib_meta_001")
            .unwrap()
            .unwrap();
        assert_eq!(item.title, "Some Video");
        assert!(item.artifacts.contains(&"summary.md".to_string()));
        assert!(item.artifacts.contains(&"wisdom.md".to_string()));
        assert!(item.artifacts.contains(&"transcript.txt".to_string()));
    }

    #[test]
    fn test_import_library_metadata_with_claims() {
        let store = test_store();
        let dir = TempDir::new().unwrap();

        let video_dir = dir.path().join("youtube").join("Claims Video (xyz)");
        fs::create_dir_all(&video_dir).unwrap();

        fs::write(
            video_dir.join("metadata.json"),
            r#"{
                "id": "claims_test_01",
                "title": "Claims Video",
                "url": "https://youtu.be/xyz",
                "content_type": "youtube",
                "processed_at": "2026-02-01T00:00:00Z",
                "tags": []
            }"#,
        )
        .unwrap();

        fs::write(
            video_dir.join("claims.json"),
            r#"{"claims": [{"claim": "A"}, {"claim": "B"}, {"claim": "C"}]}"#,
        )
        .unwrap();

        let stats = import_library_metadata(&store, dir.path()).unwrap();

        assert_eq!(stats.imported, 1);
        assert_eq!(stats.errors, 0);

        // Verify claims_count in metadata
        let item = queries::get_item(&store, "claims_test_01")
            .unwrap()
            .unwrap();
        assert_eq!(item.metadata["claims_count"], 3);
    }

    #[test]
    fn test_import_library_metadata_skips_no_metadata() {
        let store = test_store();
        let dir = TempDir::new().unwrap();

        // Directory exists but has no metadata.json
        let video_dir = dir.path().join("youtube").join("Empty Dir");
        fs::create_dir_all(&video_dir).unwrap();
        fs::write(video_dir.join("notes.txt"), "just notes").unwrap();

        let stats = import_library_metadata(&store, dir.path()).unwrap();

        assert_eq!(stats.imported, 0);
        assert_eq!(stats.skipped, 1);
    }

    #[test]
    fn test_import_library_metadata_no_youtube_dir() {
        let store = test_store();
        let dir = TempDir::new().unwrap();

        // No youtube/ subdirectory at all
        let stats = import_library_metadata(&store, dir.path()).unwrap();

        assert_eq!(stats.imported, 0);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.errors, 0);
    }

    // -- Helper tests --

    #[test]
    fn test_normalize_content_type() {
        assert_eq!(normalize_content_type("youtube"), "youtube");
        assert_eq!(normalize_content_type("YouTube"), "youtube");
        assert_eq!(normalize_content_type("you_tube"), "youtube");
        assert_eq!(normalize_content_type("yt"), "youtube");
        assert_eq!(normalize_content_type("web"), "web");
        assert_eq!(normalize_content_type("webpage"), "web");
        assert_eq!(normalize_content_type("article"), "web");
        assert_eq!(normalize_content_type("other"), "other");
        assert_eq!(normalize_content_type("podcast"), "podcast");
    }

    #[test]
    fn test_discover_artifacts() {
        let dir = TempDir::new().unwrap();
        let base = dir.path();

        fs::write(base.join("metadata.json"), "{}").unwrap();
        fs::write(base.join("summary.md"), "").unwrap();
        fs::write(base.join("wisdom.md"), "").unwrap();
        fs::write(base.join("claims.json"), "{}").unwrap();
        fs::write(base.join("evidence.jsonl"), "").unwrap();
        fs::write(base.join("transcript.txt"), "").unwrap();
        fs::write(base.join("video.mp3"), "").unwrap(); // excluded

        let artifacts = discover_artifacts(base);

        assert!(artifacts.contains(&"summary.md".to_string()));
        assert!(artifacts.contains(&"wisdom.md".to_string()));
        assert!(artifacts.contains(&"claims.json".to_string()));
        assert!(artifacts.contains(&"evidence.jsonl".to_string()));
        assert!(artifacts.contains(&"transcript.txt".to_string()));
        // metadata.json and non-artifact files excluded
        assert!(!artifacts.contains(&"metadata.json".to_string()));
        assert!(!artifacts.contains(&"video.mp3".to_string()));
        // Must be sorted
        let mut sorted = artifacts.clone();
        sorted.sort();
        assert_eq!(artifacts, sorted);
    }
}

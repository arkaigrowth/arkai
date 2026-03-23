//! Capture CLI command handler.
//!
//! Implements `arkai capture "text"` for quick thought/reminder/todo capture.
//! Auto-classifies input text and stores it as an item in the SQLite store.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::store::capture::{auto_classify, build_capture_metadata, CaptureKind};
use crate::store::embedding::{EmbeddingConfig, EmbeddingProvider, OllamaProvider};
use crate::store::queries::{self, upsert_item, UpsertItem};
use crate::store::{Store, StoreConfig};

/// Execute the capture command: classify text and store it.
pub async fn execute_capture(
    text: String,
    kind: Option<String>,
    tags: Vec<String>,
    due: Option<String>,
) -> Result<()> {
    let db_path = StoreConfig::default_path()?;
    let store = Store::open(&db_path)?;

    // Auto-classify
    let mut classification = auto_classify(&text);

    // Override kind if explicitly provided
    if let Some(k) = &kind {
        classification.kind = match k.as_str() {
            "note" => CaptureKind::Note,
            "reminder" => CaptureKind::Reminder,
            "todo" => CaptureKind::Todo,
            "link" => CaptureKind::Link,
            "voice-memo" => CaptureKind::VoiceMemo,
            "reference" => CaptureKind::Reference,
            _ => classification.kind,
        };
    }

    // Override due date if provided
    if let Some(d) = &due {
        classification.due_date = Some(d.clone());
    }

    // Generate content-addressed ID: SHA256 of text + timestamp
    let id = generate_capture_id(&text);

    // Build metadata
    let metadata = build_capture_metadata(&classification, "cli", &tags);

    // Upsert into store
    let upsert = UpsertItem {
        id: &id,
        item_type: "capture",
        title: &text,
        source_url: None,
        content_type: None,
        tags: &tags,
        artifacts: &[],
        run_id: None,
        metadata: &metadata,
    };

    upsert_item(&store, &upsert)?;

    // Embed immediately so the capture is vector-searchable
    match embed_capture(&store, &id, &text, &tags).await {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Warning: embedding failed (capture still saved): {}", e);
        }
    }

    // Print confirmation
    let kind_str = serde_json::to_string(&classification.kind).unwrap_or_default();
    let kind_display = kind_str.trim_matches('"');
    println!(
        "Captured as: {} (horizon: {:?})",
        kind_display, classification.horizon
    );
    if let Some(due) = &classification.due_date {
        println!("Due: {}", due);
    }
    println!("ID: {}", id);

    Ok(())
}

/// Embed a capture immediately after storing it.
async fn embed_capture(store: &Store, id: &str, text: &str, tags: &[String]) -> Result<()> {
    // Load embedding config from store
    let keys = [
        ("embedding_provider", "embedding.provider"),
        ("embedding_model", "embedding.model"),
        ("embedding_dimensions", "embedding.dimensions"),
    ];
    let mut pairs = Vec::new();
    for (store_key, config_key) in keys {
        if let Some(value) = store.get_config(store_key)? {
            pairs.push((config_key.to_string(), value));
        }
    }

    let config = EmbeddingConfig::from_store_config(&pairs)
        .context("Failed to load embedding config")?;
    let dims = config.dimensions;
    let model_name = config.model.clone();
    let provider = OllamaProvider::new(config);

    // Embed: title + tags (same strategy as store import)
    let embed_text = format!("{} {}", text, tags.join(" "));
    let vector: Vec<f32> = provider.embed(&embed_text).await?;

    queries::store_embedding(store, id, &model_name, dims as i32, &vector)?;
    Ok(())
}

/// Generate a content-addressed ID from text + current timestamp.
fn generate_capture_id(text: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let input = format!("{}{}", text, timestamp);
    let hash = Sha256::digest(input.as_bytes());
    hex::encode(&hash[..8]) // 16 hex chars
}

//! Evidence CLI subcommands for inspecting and validating evidence.
//!
//! Provides commands to:
//! - `show`: Display evidence details with source snippet
//! - `open`: Open the evidence location in VS Code
//! - `validate`: Verify evidence integrity against transcripts

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Subcommand;
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::evidence::{
    compute_slice_hash, offset_to_line_col, Evidence, EvidenceEvent,
};
use crate::library::{ContentId, ContentType, LibraryContent};

/// Evidence-related subcommands
#[derive(Subcommand, Debug)]
pub enum EvidenceCommands {
    /// Show details of an evidence entry
    Show {
        /// Evidence ID to display
        evidence_id: String,
    },

    /// Open evidence location in VS Code
    Open {
        /// Evidence ID to open
        evidence_id: String,
    },

    /// Validate all evidence for a content item
    Validate {
        /// Content ID to validate
        content_id: String,
    },
}

/// Metadata with artifact_digests for fast-path validation
#[derive(Debug, Deserialize)]
struct MetadataWithDigests {
    #[serde(default)]
    artifact_digests: HashMap<String, String>,
}

/// Find the content directory for a content ID
async fn find_content_directory(content_id: &str) -> Result<PathBuf> {
    let id = ContentId::from_url(content_id);

    // Try to find by ID prefix match across all content types
    for content_type in [ContentType::YouTube, ContentType::Web, ContentType::Other] {
        if let Some(dir) = LibraryContent::find_content_dir(&id, content_type).await? {
            return Ok(dir);
        }

        // Also try direct ID match for cases where content_id is the actual hash
        let type_dir = crate::config::content_type_dir(content_type)?;
        let mut entries = tokio::fs::read_dir(&type_dir).await.ok();

        if let Some(ref mut entries) = entries {
            while let Some(entry) = entries.next_entry().await? {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();

                // Match by content_id prefix in folder name
                if name_str.contains(&format!("({})", &content_id[..content_id.len().min(8)]))
                    || name_str.starts_with(&content_id[..content_id.len().min(16)])
                {
                    return Ok(entry.path());
                }
            }
        }
    }

    anyhow::bail!("Content not found: {}", content_id)
}

/// Find evidence by ID in evidence.jsonl
fn find_evidence(evidence_path: &PathBuf, evidence_id: &str) -> Result<Option<Evidence>> {
    if !evidence_path.exists() {
        return Ok(None);
    }

    let file = File::open(evidence_path)
        .with_context(|| format!("Failed to open evidence file: {}", evidence_path.display()))?;

    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let evidence: Evidence = serde_json::from_str(&line)
            .with_context(|| format!("Failed to parse evidence line: {}", line))?;

        // Match by ID prefix
        if evidence.id.starts_with(evidence_id) || evidence_id.starts_with(&evidence.id) {
            return Ok(Some(evidence));
        }
    }

    Ok(None)
}

/// Load all evidence for a content ID
fn load_all_evidence(evidence_path: &PathBuf) -> Result<Vec<Evidence>> {
    if !evidence_path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(evidence_path)
        .with_context(|| format!("Failed to open evidence file: {}", evidence_path.display()))?;

    let reader = BufReader::new(file);
    let mut evidence_list = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let evidence: Evidence = serde_json::from_str(&line)
            .with_context(|| format!("Failed to parse evidence line: {}", line))?;

        evidence_list.push(evidence);
    }

    Ok(evidence_list)
}

/// Append an event to events.jsonl with file locking
fn append_event(events_path: &PathBuf, event: &EvidenceEvent) -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(events_path)
        .with_context(|| format!("Failed to open events file: {}", events_path.display()))?;

    // Acquire exclusive lock
    file.lock_exclusive()
        .context("Failed to acquire file lock on events.jsonl")?;

    // Create a wrapper struct for serialization with timestamp
    #[derive(Serialize)]
    struct EventWrapper<'a> {
        ts: String,
        #[serde(flatten)]
        event: &'a EvidenceEvent,
    }

    let wrapper = EventWrapper {
        ts: Utc::now().to_rfc3339(),
        event,
    };

    let json = serde_json::to_string(&wrapper).context("Failed to serialize event")?;

    let mut file = file;
    writeln!(file, "{}", json).context("Failed to write event")?;
    file.flush().context("Failed to flush event")?;

    // Lock is released when file is dropped
    Ok(())
}

/// Execute the `evidence show` command
pub async fn execute_show(evidence_id: &str) -> Result<()> {
    // Search through all content directories for evidence.jsonl files
    for content_type in [ContentType::YouTube, ContentType::Web, ContentType::Other] {
        let type_dir = crate::config::content_type_dir(content_type)?;

        if !type_dir.exists() {
            continue;
        }

        let mut entries = tokio::fs::read_dir(&type_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let content_dir = entry.path();
            let evidence_path = content_dir.join("evidence.jsonl");

            if let Some(evidence) = find_evidence(&evidence_path, evidence_id)? {
                // Found the evidence, now display it
                return display_evidence(&evidence, &content_dir).await;
            }
        }
    }

    anyhow::bail!("Evidence not found: {}", evidence_id)
}

/// Display evidence details
async fn display_evidence(evidence: &Evidence, content_dir: &PathBuf) -> Result<()> {
    println!("Evidence ID: {}", evidence.id);
    println!("Content ID:  {}", evidence.content_id);
    println!("Status:      {:?}", evidence.status);
    println!("Confidence:  {:.2}", evidence.confidence);
    println!("Extractor:   {}", evidence.extractor);
    println!("Timestamp:   {}", evidence.ts);
    println!();
    println!("Claim:");
    println!("  {}", evidence.claim);
    println!();
    println!("Quote:");
    println!("  \"{}\"", evidence.quote);
    println!("  (SHA256: {})", evidence.quote_sha256);

    if let Some(span) = &evidence.span {
        let artifact_path = content_dir.join(&span.artifact);
        println!();
        println!("Source Location:");
        println!("  File: {}", artifact_path.display());
        println!(
            "  Bytes: {} - {}",
            span.utf8_byte_offset[0], span.utf8_byte_offset[1]
        );

        // Load the transcript and compute line:col
        if artifact_path.exists() {
            let transcript = tokio::fs::read_to_string(&artifact_path).await?;
            let line_col = offset_to_line_col(&transcript, span.utf8_byte_offset[0]);
            println!("  Position: line {}, col {}", line_col.line, line_col.col);

            // Extract and display snippet
            let start = span.utf8_byte_offset[0];
            let end = span.utf8_byte_offset[1].min(transcript.len());

            if start < transcript.len() {
                let snippet = &transcript[start..end];
                println!();
                println!("Snippet:");
                println!("  ---");
                for line in snippet.lines().take(5) {
                    println!("  {}", line);
                }
                if snippet.lines().count() > 5 {
                    println!("  ...");
                }
                println!("  ---");
            }
        } else {
            println!("  (artifact file not found)");
        }

        if let Some(anchor) = &span.anchor_text {
            println!();
            println!("Anchor text: {}", anchor);
        }

        if let Some(ts) = &span.video_timestamp {
            println!("Video timestamp: {}", ts);
        }
    } else {
        println!();
        println!("(No span - evidence is unresolved)");
        if let Some(reason) = &evidence.resolution.reason {
            println!("Reason: {:?}", reason);
        }
    }

    Ok(())
}

/// Execute the `evidence open` command
pub async fn execute_open(evidence_id: &str) -> Result<()> {
    // Search through all content directories for evidence.jsonl files
    for content_type in [ContentType::YouTube, ContentType::Web, ContentType::Other] {
        let type_dir = crate::config::content_type_dir(content_type)?;

        if !type_dir.exists() {
            continue;
        }

        let mut entries = tokio::fs::read_dir(&type_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let content_dir = entry.path();
            let evidence_path = content_dir.join("evidence.jsonl");

            if let Some(evidence) = find_evidence(&evidence_path, evidence_id)? {
                // Found the evidence, now open it
                return open_evidence(&evidence, &content_dir).await;
            }
        }
    }

    anyhow::bail!("Evidence not found: {}", evidence_id)
}

/// Open evidence in VS Code
async fn open_evidence(evidence: &Evidence, content_dir: &PathBuf) -> Result<()> {
    let span = evidence.span.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Evidence {} is unresolved - no source location available",
            evidence.id
        )
    })?;

    let artifact_path = content_dir.join(&span.artifact);

    if !artifact_path.exists() {
        anyhow::bail!(
            "Artifact file not found: {}\nThe transcript may have been deleted or moved.",
            artifact_path.display()
        );
    }

    // Load transcript and compute line:col
    let transcript = tokio::fs::read_to_string(&artifact_path).await?;
    let line_col = offset_to_line_col(&transcript, span.utf8_byte_offset[0]);

    // Try to open in VS Code
    let vscode_arg = format!(
        "{}:{}:{}",
        artifact_path.display(),
        line_col.line,
        line_col.col
    );

    println!("Opening in VS Code: {}", vscode_arg);

    let result = Command::new("code").args(["-g", &vscode_arg]).status();

    match result {
        Ok(status) if status.success() => {
            println!("Opened successfully.");
            Ok(())
        }
        Ok(_) => {
            println!();
            println!("VS Code command failed. You can manually open:");
            println!("  File: {}", artifact_path.display());
            println!("  Line: {}, Column: {}", line_col.line, line_col.col);
            Ok(())
        }
        Err(_) => {
            println!();
            println!("VS Code ('code' command) not found in PATH.");
            println!();
            println!("To open manually:");
            println!("  File: {}", artifact_path.display());
            println!("  Line: {}, Column: {}", line_col.line, line_col.col);
            println!();
            println!("Or run:");
            println!("  code -g \"{}\"", vscode_arg);
            Ok(())
        }
    }
}

/// Execute the `evidence validate` command
pub async fn execute_validate(content_id: &str) -> Result<()> {
    let content_dir = find_content_directory(content_id).await?;

    println!("Validating evidence for: {}", content_dir.display());
    println!();

    let evidence_path = content_dir.join("evidence.jsonl");
    let metadata_path = content_dir.join("metadata.json");
    let events_path = content_dir.join("events.jsonl");

    // Load metadata with artifact_digests if available
    let metadata: Option<MetadataWithDigests> = if metadata_path.exists() {
        let content = tokio::fs::read_to_string(&metadata_path).await?;
        serde_json::from_str(&content).ok()
    } else {
        None
    };

    // Load all evidence
    let evidence_list = load_all_evidence(&evidence_path)?;

    if evidence_list.is_empty() {
        println!("No evidence found in evidence.jsonl");

        // Still emit event
        let event = EvidenceEvent::EvidenceValidated {
            content_id: content_id.to_string(),
            artifact: "transcript.md".to_string(),
            digest_ok: true,
            valid_count: 0,
            stale_count: 0,
            unresolved_count: 0,
        };
        append_event(&events_path, &event)?;

        return Ok(());
    }

    // Group evidence by artifact
    let mut by_artifact: HashMap<String, Vec<&Evidence>> = HashMap::new();
    let mut unresolved_count = 0;

    for evidence in &evidence_list {
        if let Some(span) = &evidence.span {
            by_artifact
                .entry(span.artifact.clone())
                .or_default()
                .push(evidence);
        } else {
            unresolved_count += 1;
        }
    }

    let mut total_valid = 0;
    let mut total_stale = 0;
    let mut artifact_missing_count = 0;

    // Validate each artifact group
    for (artifact_name, evidence_group) in &by_artifact {
        let artifact_path = content_dir.join(artifact_name);

        println!("Artifact: {}", artifact_name);

        if !artifact_path.exists() {
            println!("  Status: MISSING");
            println!("  Evidence count: {} (all marked artifact_missing)", evidence_group.len());
            artifact_missing_count += evidence_group.len();

            // Emit event for missing artifact
            let event = EvidenceEvent::EvidenceValidated {
                content_id: content_id.to_string(),
                artifact: artifact_name.clone(),
                digest_ok: false,
                valid_count: 0,
                stale_count: 0,
                unresolved_count: evidence_group.len(),
            };
            append_event(&events_path, &event)?;

            continue;
        }

        // Load transcript for validation
        let transcript = tokio::fs::read_to_string(&artifact_path).await?;
        let transcript_bytes = transcript.as_bytes();

        // Check for digest fast-path
        let mut use_fast_path = false;
        if let Some(ref meta) = metadata {
            if let Some(stored_digest) = meta.artifact_digests.get(artifact_name) {
                let current_digest = crate::evidence::compute_hash(transcript_bytes);
                if &current_digest == stored_digest {
                    use_fast_path = true;
                    println!("  Digest: OK (fast-path - skipping per-span checks)");
                } else {
                    println!("  Digest: CHANGED (checking individual spans)");
                }
            }
        }

        if use_fast_path {
            // All evidence for this artifact is valid
            total_valid += evidence_group.len();
            println!("  Valid: {}", evidence_group.len());

            let event = EvidenceEvent::EvidenceValidated {
                content_id: content_id.to_string(),
                artifact: artifact_name.clone(),
                digest_ok: true,
                valid_count: evidence_group.len(),
                stale_count: 0,
                unresolved_count: 0,
            };
            append_event(&events_path, &event)?;
        } else {
            // Validate each span individually
            let mut valid = 0;
            let mut stale = 0;

            for evidence in evidence_group {
                if let Some(span) = &evidence.span {
                    let start = span.utf8_byte_offset[0];
                    let end = span.utf8_byte_offset[1];

                    if end <= transcript_bytes.len() {
                        let current_hash = compute_slice_hash(transcript_bytes, start, end);
                        if current_hash == span.slice_sha256 {
                            valid += 1;
                        } else {
                            stale += 1;
                            println!(
                                "    STALE: {} (hash mismatch at {}:{})",
                                evidence.id, start, end
                            );
                        }
                    } else {
                        stale += 1;
                        println!(
                            "    STALE: {} (offset {} out of bounds, file size {})",
                            evidence.id, end, transcript_bytes.len()
                        );
                    }
                }
            }

            total_valid += valid;
            total_stale += stale;

            println!("  Valid: {}, Stale: {}", valid, stale);

            let event = EvidenceEvent::EvidenceValidated {
                content_id: content_id.to_string(),
                artifact: artifact_name.clone(),
                digest_ok: false,
                valid_count: valid,
                stale_count: stale,
                unresolved_count: 0,
            };
            append_event(&events_path, &event)?;
        }
    }

    // Print summary
    println!();
    println!("Summary:");
    println!("  Total evidence: {}", evidence_list.len());
    println!("  Valid:          {}", total_valid);
    println!("  Stale:          {}", total_stale);
    println!("  Unresolved:     {}", unresolved_count);
    if artifact_missing_count > 0 {
        println!("  Artifact missing: {}", artifact_missing_count);
    }

    if total_stale > 0 || artifact_missing_count > 0 {
        println!();
        println!("Some evidence needs re-extraction due to transcript changes.");
    }

    Ok(())
}

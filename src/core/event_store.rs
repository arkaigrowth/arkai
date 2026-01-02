//! Append-only event store with file-based persistence.
//!
//! Events are stored as newline-delimited JSON (JSONL) for simplicity
//! and easy debugging/inspection.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

use crate::domain::{Event, EventType};

/// File-based event store using JSONL format
pub struct EventStore {
    /// Directory containing the run
    run_dir: PathBuf,

    /// Path to the events.jsonl file
    events_path: PathBuf,

    /// Path to artifacts directory
    artifacts_dir: PathBuf,
}

impl EventStore {
    /// Create or open an event store for a run
    pub async fn open(run_id: Uuid) -> Result<Self> {
        let base_dir = Self::base_directory()?;
        let run_dir = base_dir.join(run_id.to_string());
        let artifacts_dir = run_dir.join("artifacts");

        // Create directory structure including artifacts
        fs::create_dir_all(&artifacts_dir)
            .await
            .with_context(|| format!("Failed to create artifacts directory: {}", artifacts_dir.display()))?;

        let events_path = run_dir.join("events.jsonl");

        Ok(Self {
            run_dir,
            events_path,
            artifacts_dir,
        })
    }

    /// Get the base directory for all runs (~/.arkai/runs or $ARKAI_HOME/runs)
    pub fn base_directory() -> Result<PathBuf> {
        crate::config::runs_dir()
    }

    /// Get the path to the events file
    pub fn events_path(&self) -> &Path {
        &self.events_path
    }

    /// Get the run directory
    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    /// Get the artifacts directory
    pub fn artifacts_dir(&self) -> &Path {
        &self.artifacts_dir
    }

    /// Store an artifact to disk
    pub async fn store_artifact(&self, step_name: &str, content: &str) -> Result<PathBuf> {
        let artifact_path = self.artifacts_dir.join(format!("{}.md", step_name));

        fs::write(&artifact_path, content)
            .await
            .with_context(|| format!("Failed to write artifact: {}", artifact_path.display()))?;

        Ok(artifact_path)
    }

    /// Load an artifact from disk
    pub async fn load_artifact(&self, step_name: &str) -> Result<Option<String>> {
        let artifact_path = self.artifacts_dir.join(format!("{}.md", step_name));

        if !artifact_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&artifact_path)
            .await
            .with_context(|| format!("Failed to read artifact: {}", artifact_path.display()))?;

        Ok(Some(content))
    }

    /// List all artifacts in this run
    pub async fn list_artifacts(&self) -> Result<Vec<String>> {
        let mut artifacts = Vec::new();

        if !self.artifacts_dir.exists() {
            return Ok(artifacts);
        }

        let mut entries = fs::read_dir(&self.artifacts_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".md") {
                    artifacts.push(name.trim_end_matches(".md").to_string());
                }
            }
        }

        Ok(artifacts)
    }

    /// Append an event to the log
    pub async fn append(&self, event: &Event) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to open events file: {}",
                    self.events_path.display()
                )
            })?;

        let json = serde_json::to_string(event).context("Failed to serialize event")?;
        file.write_all(format!("{}\n", json).as_bytes())
            .await
            .context("Failed to write event")?;
        file.flush().await.context("Failed to flush event")?;

        Ok(())
    }

    /// Replay all events in order
    pub async fn replay(&self) -> Result<Vec<Event>> {
        if !self.events_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.events_path)
            .await
            .with_context(|| format!("Failed to open events file: {}", self.events_path.display()))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut events = Vec::new();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let event: Event = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse event: {}", line))?;
            events.push(event);
        }

        Ok(events)
    }

    /// Check if a step is already completed (idempotency check)
    pub async fn is_step_completed(&self, idempotency_key: &str) -> Result<bool> {
        let events = self.replay().await?;

        let completed = events.iter().any(|e| {
            e.idempotency_key == idempotency_key
                && matches!(e.event_type, EventType::StepCompleted)
        });

        Ok(completed)
    }

    /// Find events matching a predicate
    pub async fn find_events<F>(&self, predicate: F) -> Result<Vec<Event>>
    where
        F: Fn(&Event) -> bool,
    {
        let events = self.replay().await?;
        Ok(events.into_iter().filter(predicate).collect())
    }

    /// Get the last event of a specific type
    pub async fn last_event_of_type(&self, event_type: EventType) -> Result<Option<Event>> {
        let events = self.replay().await?;
        Ok(events.into_iter().rev().find(|e| e.event_type == event_type))
    }

    /// List all run IDs in the base directory
    pub async fn list_runs() -> Result<Vec<Uuid>> {
        let base_dir = Self::base_directory()?;

        if !base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut runs = Vec::new();
        let mut entries = fs::read_dir(&base_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Ok(uuid) = Uuid::parse_str(name) {
                        runs.push(uuid);
                    }
                }
            }
        }

        Ok(runs)
    }
}

/// Generate an idempotency key for a step
pub fn generate_idempotency_key(run_id: Uuid, step_name: &str, input: &str) -> String {
    let input_hash = hash_input(input);
    format!("{}:{}:{}", run_id, step_name, input_hash)
}

/// Hash input content (first 16 chars of SHA256)
pub fn hash_input(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8]) // First 16 hex chars (8 bytes)
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::StepStatus;
    use tempfile::TempDir;

    // Helper to create a test event store in a temp directory
    async fn create_test_store() -> (EventStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let run_id = Uuid::new_v4();

        // Override the base directory for testing
        let run_dir = temp_dir.path().join(run_id.to_string());
        let artifacts_dir = run_dir.join("artifacts");
        std::fs::create_dir_all(&artifacts_dir).unwrap();

        let store = EventStore {
            run_dir: run_dir.clone(),
            events_path: run_dir.join("events.jsonl"),
            artifacts_dir,
        };

        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_event_append_and_replay() {
        let (store, _temp) = create_test_store().await;
        let run_id = Uuid::new_v4();

        // Append events
        let event1 = Event::new(
            run_id,
            None,
            EventType::RunStarted,
            format!("{}:start", run_id),
            "Run started".to_string(),
            StepStatus::Running,
        );

        let event2 = Event::new(
            run_id,
            Some("step1".to_string()),
            EventType::StepStarted,
            format!("{}:step1:abc", run_id),
            "Step started".to_string(),
            StepStatus::Running,
        );

        store.append(&event1).await.unwrap();
        store.append(&event2).await.unwrap();

        // Replay
        let events = store.replay().await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, EventType::RunStarted);
        assert_eq!(events[1].event_type, EventType::StepStarted);
    }

    #[tokio::test]
    async fn test_event_replay_order() {
        let (store, _temp) = create_test_store().await;
        let run_id = Uuid::new_v4();

        // Append 5 events
        for i in 0..5 {
            let event = Event::new(
                run_id,
                Some(format!("step{}", i)),
                EventType::StepStarted,
                format!("{}:step{}:abc", run_id, i),
                format!("Step {} started", i),
                StepStatus::Running,
            );
            store.append(&event).await.unwrap();
        }

        // Replay and verify order
        let events = store.replay().await.unwrap();
        assert_eq!(events.len(), 5);

        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.step_id, Some(format!("step{}", i)));
        }
    }

    #[tokio::test]
    async fn test_idempotency_check() {
        let (store, _temp) = create_test_store().await;
        let run_id = Uuid::new_v4();
        let idem_key = format!("{}:step1:abc123", run_id);

        // Initially not completed
        assert!(!store.is_step_completed(&idem_key).await.unwrap());

        // Add a StepStarted event (not complete)
        let started = Event::new(
            run_id,
            Some("step1".to_string()),
            EventType::StepStarted,
            idem_key.clone(),
            "Step started".to_string(),
            StepStatus::Running,
        );
        store.append(&started).await.unwrap();

        // Still not completed
        assert!(!store.is_step_completed(&idem_key).await.unwrap());

        // Add StepCompleted event
        let completed = Event::new(
            run_id,
            Some("step1".to_string()),
            EventType::StepCompleted,
            idem_key.clone(),
            "Step completed".to_string(),
            StepStatus::Completed,
        );
        store.append(&completed).await.unwrap();

        // Now completed
        assert!(store.is_step_completed(&idem_key).await.unwrap());
    }

    #[test]
    fn test_idempotency_key_format() {
        let run_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let key = generate_idempotency_key(run_id, "summarize", "test input");

        // Format: {run_id}:{step}:{hash16}
        assert!(key.starts_with("550e8400-e29b-41d4-a716-446655440000:summarize:"));

        // Hash should be 16 hex chars
        let parts: Vec<&str> = key.split(':').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[2].len(), 16);
    }

    #[test]
    fn test_input_hash_consistency() {
        let hash1 = hash_input("test input");
        let hash2 = hash_input("test input");
        let hash3 = hash_input("different input");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 16); // 8 bytes = 16 hex chars
    }
}

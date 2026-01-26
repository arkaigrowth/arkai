//! JSONL-based voice queue for idempotent processing.
//!
//! Follows the EventStore pattern: append-only JSONL with state derived from replay.
//! Each queue item is stored as a JSON line, and state changes are appended as new entries.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::domain::VoiceQueueStatus;

/// Errors that can occur with the voice queue
#[derive(Debug, Error)]
pub enum VoiceQueueError {
    #[error("Queue item not found: {0}")]
    NotFound(String),

    #[error("Item already exists: {0}")]
    AlreadyExists(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid state transition: {from:?} → {to:?}")]
    InvalidTransition {
        from: VoiceQueueStatus,
        to: VoiceQueueStatus,
    },
}

/// An event in the queue log (append-only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEvent {
    /// When this event occurred
    pub timestamp: DateTime<Utc>,

    /// The queue item ID (content hash)
    pub item_id: String,

    /// Type of queue event
    pub event_type: QueueEventType,

    /// Additional data (depends on event type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Types of queue events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueEventType {
    /// Item added to queue
    Enqueued,

    /// Processing started
    ProcessingStarted,

    /// Processing completed successfully
    Completed,

    /// Processing failed
    Failed,

    /// Reset for retry
    ResetForRetry,
}

/// Metadata for a queued audio file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemData {
    /// Original file path
    pub file_path: PathBuf,

    /// File name only
    pub file_name: String,

    /// File size in bytes
    pub file_size: u64,

    /// When the file was detected
    pub detected_at: DateTime<Utc>,
}

/// A queue item with current state (derived from replaying events)
#[derive(Debug, Clone)]
pub struct QueueItem {
    /// Unique ID (SHA256 hash, 12 chars)
    pub id: String,

    /// Current status
    pub status: VoiceQueueStatus,

    /// Item metadata
    pub data: QueueItemData,

    /// When processing started (if applicable)
    pub started_at: Option<DateTime<Utc>>,

    /// When processing completed (if applicable)
    pub completed_at: Option<DateTime<Utc>>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Number of retry attempts
    pub retry_count: u32,
}

/// JSONL-based voice queue
pub struct VoiceQueue {
    /// Path to the queue JSONL file
    queue_path: PathBuf,
}

impl VoiceQueue {
    /// Create a new voice queue
    pub fn new(queue_path: PathBuf) -> Self {
        Self { queue_path }
    }

    /// Create a queue in the default location (~/.arkai/voice_queue.jsonl)
    pub fn default_path() -> Result<PathBuf> {
        let home = crate::config::arkai_home()?;
        Ok(home.join("voice_queue.jsonl"))
    }

    /// Open the default queue
    pub async fn open_default() -> Result<Self> {
        let path = Self::default_path()?;

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        Ok(Self::new(path))
    }

    /// Append an event to the queue log
    async fn append_event(&self, event: &QueueEvent) -> Result<(), VoiceQueueError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.queue_path)
            .await?;

        let json = serde_json::to_string(event)?;
        file.write_all(format!("{}\n", json).as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }

    /// Replay all events to build current state
    pub async fn replay(&self) -> Result<HashMap<String, QueueItem>, VoiceQueueError> {
        let mut items: HashMap<String, QueueItem> = HashMap::new();

        if !self.queue_path.exists() {
            return Ok(items);
        }

        let file = File::open(&self.queue_path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            let event: QueueEvent = serde_json::from_str(&line)?;
            Self::apply_event(&mut items, event);
        }

        Ok(items)
    }

    /// Apply a single event to the state
    fn apply_event(items: &mut HashMap<String, QueueItem>, event: QueueEvent) {
        match event.event_type {
            QueueEventType::Enqueued => {
                if let Some(data) = event.data {
                    if let Ok(item_data) = serde_json::from_value::<QueueItemData>(data) {
                        items.insert(
                            event.item_id.clone(),
                            QueueItem {
                                id: event.item_id,
                                status: VoiceQueueStatus::Pending,
                                data: item_data,
                                started_at: None,
                                completed_at: None,
                                error: None,
                                retry_count: 0,
                            },
                        );
                    }
                }
            }
            QueueEventType::ProcessingStarted => {
                if let Some(item) = items.get_mut(&event.item_id) {
                    item.status = VoiceQueueStatus::Processing;
                    item.started_at = Some(event.timestamp);
                }
            }
            QueueEventType::Completed => {
                if let Some(item) = items.get_mut(&event.item_id) {
                    item.status = VoiceQueueStatus::Done;
                    item.completed_at = Some(event.timestamp);
                }
            }
            QueueEventType::Failed => {
                if let Some(item) = items.get_mut(&event.item_id) {
                    item.status = VoiceQueueStatus::Failed;
                    item.completed_at = Some(event.timestamp);
                    if let Some(data) = event.data {
                        if let Some(error) = data.get("error").and_then(|e| e.as_str()) {
                            item.error = Some(error.to_string());
                        }
                    }
                }
            }
            QueueEventType::ResetForRetry => {
                if let Some(item) = items.get_mut(&event.item_id) {
                    item.status = VoiceQueueStatus::Pending;
                    item.retry_count += 1;
                    item.error = None;
                    item.started_at = None;
                    item.completed_at = None;
                }
            }
        }
    }

    /// Enqueue a new audio file (idempotent - returns existing if already queued)
    pub async fn enqueue(
        &self,
        file_path: &Path,
        file_size: u64,
        detected_at: DateTime<Utc>,
    ) -> Result<EnqueueResult, VoiceQueueError> {
        // Compute content hash
        let hash = compute_file_hash(file_path).await?;

        // Check if already exists
        let items = self.replay().await?;
        if let Some(existing) = items.get(&hash) {
            match existing.status {
                VoiceQueueStatus::Done => {
                    return Ok(EnqueueResult::AlreadyProcessed(hash));
                }
                VoiceQueueStatus::Failed => {
                    // Reset for retry
                    let event = QueueEvent {
                        timestamp: Utc::now(),
                        item_id: hash.clone(),
                        event_type: QueueEventType::ResetForRetry,
                        data: None,
                    };
                    self.append_event(&event).await?;
                    return Ok(EnqueueResult::ResetForRetry(hash));
                }
                _ => {
                    return Ok(EnqueueResult::AlreadyQueued(hash));
                }
            }
        }

        // Create queue item data
        let item_data = QueueItemData {
            file_path: file_path.to_path_buf(),
            file_name: file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            file_size,
            detected_at,
        };

        // Append enqueue event
        let event = QueueEvent {
            timestamp: Utc::now(),
            item_id: hash.clone(),
            event_type: QueueEventType::Enqueued,
            data: Some(serde_json::to_value(&item_data)?),
        };
        self.append_event(&event).await?;

        Ok(EnqueueResult::Queued(hash))
    }

    /// Get all pending items (ready for processing)
    pub async fn get_pending(&self) -> Result<Vec<QueueItem>, VoiceQueueError> {
        let items = self.replay().await?;
        let mut pending: Vec<QueueItem> = items
            .into_values()
            .filter(|item| item.status == VoiceQueueStatus::Pending)
            .collect();

        // Sort by detected_at (oldest first)
        pending.sort_by(|a, b| a.data.detected_at.cmp(&b.data.detected_at));

        Ok(pending)
    }

    /// Mark an item as processing
    pub async fn mark_processing(&self, id: &str) -> Result<(), VoiceQueueError> {
        let items = self.replay().await?;
        let item = items.get(id).ok_or_else(|| VoiceQueueError::NotFound(id.to_string()))?;

        if item.status != VoiceQueueStatus::Pending {
            return Err(VoiceQueueError::InvalidTransition {
                from: item.status,
                to: VoiceQueueStatus::Processing,
            });
        }

        let event = QueueEvent {
            timestamp: Utc::now(),
            item_id: id.to_string(),
            event_type: QueueEventType::ProcessingStarted,
            data: None,
        };
        self.append_event(&event).await?;

        Ok(())
    }

    /// Mark an item as done
    pub async fn mark_done(&self, id: &str) -> Result<(), VoiceQueueError> {
        let event = QueueEvent {
            timestamp: Utc::now(),
            item_id: id.to_string(),
            event_type: QueueEventType::Completed,
            data: None,
        };
        self.append_event(&event).await?;

        Ok(())
    }

    /// Mark an item as failed
    pub async fn mark_failed(&self, id: &str, error: &str) -> Result<(), VoiceQueueError> {
        let event = QueueEvent {
            timestamp: Utc::now(),
            item_id: id.to_string(),
            event_type: QueueEventType::Failed,
            data: Some(serde_json::json!({ "error": error })),
        };
        self.append_event(&event).await?;

        Ok(())
    }

    /// Get queue status summary
    pub async fn status(&self) -> Result<QueueStatus, VoiceQueueError> {
        let items = self.replay().await?;

        let mut status = QueueStatus::default();
        for item in items.values() {
            match item.status {
                VoiceQueueStatus::Pending => status.pending += 1,
                VoiceQueueStatus::Processing => status.processing += 1,
                VoiceQueueStatus::Done => status.done += 1,
                VoiceQueueStatus::Failed => status.failed += 1,
            }
        }

        // Get recent items (last 5)
        let mut all_items: Vec<&QueueItem> = items.values().collect();
        all_items.sort_by(|a, b| b.data.detected_at.cmp(&a.data.detected_at));
        status.recent = all_items
            .into_iter()
            .take(5)
            .cloned()
            .collect();

        Ok(status)
    }

    /// Get a specific item by ID
    pub async fn get(&self, id: &str) -> Result<Option<QueueItem>, VoiceQueueError> {
        let items = self.replay().await?;
        Ok(items.get(id).cloned())
    }
}

/// Result of enqueueing an item
#[derive(Debug, Clone)]
pub enum EnqueueResult {
    /// Successfully queued (new item)
    Queued(String),

    /// Already queued and pending/processing
    AlreadyQueued(String),

    /// Already processed (done)
    AlreadyProcessed(String),

    /// Reset from failed state for retry
    ResetForRetry(String),
}

impl EnqueueResult {
    /// Get the item ID regardless of result type
    pub fn id(&self) -> &str {
        match self {
            Self::Queued(id)
            | Self::AlreadyQueued(id)
            | Self::AlreadyProcessed(id)
            | Self::ResetForRetry(id) => id,
        }
    }

    /// Check if this was a new enqueue
    pub fn is_new(&self) -> bool {
        matches!(self, Self::Queued(_))
    }
}

/// Queue status summary
#[derive(Debug, Clone, Default)]
pub struct QueueStatus {
    pub pending: usize,
    pub processing: usize,
    pub done: usize,
    pub failed: usize,
    pub recent: Vec<QueueItem>,
}

impl QueueStatus {
    /// Total items in queue
    pub fn total(&self) -> usize {
        self.pending + self.processing + self.done + self.failed
    }
}

/// Compute SHA256 hash of file content (first 12 chars)
pub async fn compute_file_hash(path: &Path) -> Result<String, std::io::Error> {
    let content = tokio::fs::read(path).await?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();

    // Return first 12 hex characters
    Ok(format!("{:x}", result)[..12].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_queue() -> (VoiceQueue, TempDir) {
        let temp = TempDir::new().unwrap();
        let queue_path = temp.path().join("test_queue.jsonl");
        (VoiceQueue::new(queue_path), temp)
    }

    #[tokio::test]
    async fn test_enqueue_new_item() {
        let (queue, temp) = create_test_queue().await;

        // Create a test audio file
        let audio_path = temp.path().join("test.m4a");
        tokio::fs::write(&audio_path, b"fake audio content").await.unwrap();

        let result = queue
            .enqueue(&audio_path, 18, Utc::now())
            .await
            .unwrap();

        assert!(result.is_new());

        // Verify it's in pending state
        let status = queue.status().await.unwrap();
        assert_eq!(status.pending, 1);
        assert_eq!(status.done, 0);
    }

    #[tokio::test]
    async fn test_idempotent_enqueue() {
        let (queue, temp) = create_test_queue().await;

        let audio_path = temp.path().join("test.m4a");
        tokio::fs::write(&audio_path, b"fake audio content").await.unwrap();

        // Enqueue twice
        let result1 = queue.enqueue(&audio_path, 18, Utc::now()).await.unwrap();
        let result2 = queue.enqueue(&audio_path, 18, Utc::now()).await.unwrap();

        assert!(result1.is_new());
        assert!(!result2.is_new());
        assert_eq!(result1.id(), result2.id());

        // Should still only have 1 pending
        let status = queue.status().await.unwrap();
        assert_eq!(status.pending, 1);
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let (queue, temp) = create_test_queue().await;

        let audio_path = temp.path().join("test.m4a");
        tokio::fs::write(&audio_path, b"fake audio content").await.unwrap();

        let result = queue.enqueue(&audio_path, 18, Utc::now()).await.unwrap();
        let id = result.id().to_string();

        // Pending → Processing
        queue.mark_processing(&id).await.unwrap();
        let item = queue.get(&id).await.unwrap().unwrap();
        assert_eq!(item.status, VoiceQueueStatus::Processing);

        // Processing → Done
        queue.mark_done(&id).await.unwrap();
        let item = queue.get(&id).await.unwrap().unwrap();
        assert_eq!(item.status, VoiceQueueStatus::Done);
    }

    #[tokio::test]
    async fn test_retry_failed_item() {
        let (queue, temp) = create_test_queue().await;

        let audio_path = temp.path().join("test.m4a");
        tokio::fs::write(&audio_path, b"fake audio content").await.unwrap();

        let result = queue.enqueue(&audio_path, 18, Utc::now()).await.unwrap();
        let id = result.id().to_string();

        // Mark as failed
        queue.mark_processing(&id).await.unwrap();
        queue.mark_failed(&id, "test error").await.unwrap();

        let item = queue.get(&id).await.unwrap().unwrap();
        assert_eq!(item.status, VoiceQueueStatus::Failed);
        assert_eq!(item.error, Some("test error".to_string()));

        // Re-enqueue should reset for retry
        let result2 = queue.enqueue(&audio_path, 18, Utc::now()).await.unwrap();
        assert!(matches!(result2, EnqueueResult::ResetForRetry(_)));

        let item = queue.get(&id).await.unwrap().unwrap();
        assert_eq!(item.status, VoiceQueueStatus::Pending);
        assert_eq!(item.retry_count, 1);
    }
}

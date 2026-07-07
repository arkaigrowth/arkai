//! JSONL-based voice queue for idempotent processing.
//!
//! Follows the EventStore pattern: append-only JSONL with state derived from replay.
//! Each queue item is stored as a JSON line, and state changes are appended as new entries.

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex, OnceLock, Weak};
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex as AsyncMutex;

use crate::domain::VoiceQueueStatus;

const AUTO_RETRY_CAP: u32 = 1;
const STALE_PROCESSING_BACKSTOP_SECS: i64 = 24 * 60 * 60;
const PID_REUSE_GRACE_SECS: i64 = 5;
const CACHE_RETENTION_SECS: u64 = 30 * 24 * 60 * 60;

/// Errors that can occur with the voice queue
#[derive(Debug, Error)]
pub enum VoiceQueueError {
    #[error("Queue item not found: {0}")]
    NotFound(String),

    #[error("Queue item ID prefix is ambiguous: {0}")]
    AmbiguousId(String),

    #[error("Item already exists: {0}")]
    AlreadyExists(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cache copy failed from {from} to {to}: {source}")]
    CacheCopy {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },

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

    /// Human approved item for processing
    Approved,

    /// Human skipped item
    Skipped,

    /// Processing started
    ProcessingStarted,

    /// Processing completed successfully
    Completed,

    /// Processing failed
    Failed,

    /// Reset for retry
    ResetForRetry,

    /// Unknown future event type (safe no-op for forward compatibility)
    #[serde(other)]
    Unknown,
}

/// Human approval decision payload for an approved queue item
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApprovalDecision {
    #[serde(default)]
    pub engine: Option<String>,

    #[serde(default)]
    pub speakers: Option<u32>,

    #[serde(default)]
    pub names: Option<String>,

    #[serde(default)]
    pub category: Option<String>,

    #[serde(default)]
    pub allow_large: bool,

    #[serde(default)]
    pub hint: Option<String>,

    #[serde(default = "Utc::now")]
    pub decided_at: DateTime<Utc>,
}

/// Processing metadata recorded with `ProcessingStarted` in M3.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessingStartedData {
    #[serde(default)]
    pub pid: Option<u32>,

    #[serde(default)]
    pub proc_start_epoch: Option<i64>,
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

    /// Audio duration in seconds (populated via ffprobe)
    #[serde(default)]
    pub duration_seconds: Option<f32>,

    /// Original recording timestamp, populated by later source scanners.
    #[serde(default)]
    pub recorded_at: Option<DateTime<Utc>>,

    /// Source config name that discovered this item.
    #[serde(default)]
    pub source: Option<String>,

    /// Processing kind: "audio" or "prediarized".
    #[serde(default)]
    pub kind: Option<String>,

    /// Whether remote notifications must redact human-readable details.
    #[serde(default)]
    pub private: bool,
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

    /// Whether this item has ever cleared the human approval gate
    pub approved_once: bool,

    /// Engine selected during approval, if any
    pub chosen_engine: Option<String>,

    /// Human-provided processing overrides
    pub overrides: Option<serde_json::Value>,

    /// Whether the human allowed large-file processing
    pub allow_large: bool,

    /// When a human approval decision was recorded
    pub decided_at: Option<DateTime<Utc>>,

    /// PID recorded when the current processing attempt started.
    pub pid: Option<u32>,

    /// Process start epoch recorded with `pid`.
    pub proc_start_epoch: Option<i64>,

    /// Count of `ProcessingStarted` events derived during replay.
    pub attempt_count: u32,

    /// Count of automatic reset events derived during replay.
    pub auto_reset_count: u32,
}

/// Additional replay counters for operational alerts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplayStats {
    pub skipped_lines: usize,
}

/// Result of a stale-processing reaper pass.
#[derive(Debug, Clone, Default)]
pub struct ReapReport {
    pub reset_for_retry: Vec<String>,
    pub quarantined: Vec<QueueItem>,
}

#[derive(Debug, Clone)]
pub struct EnqueueMetadata {
    pub source: Option<String>,
    pub kind: Option<String>,
    pub private: bool,
    pub recorded_at: Option<DateTime<Utc>>,
}

impl Default for EnqueueMetadata {
    fn default() -> Self {
        Self {
            source: None,
            kind: Some("audio".to_string()),
            private: false,
            recorded_at: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CacheSweepReport {
    pub backfilled: usize,
    pub retained: usize,
    pub errors: usize,
}

/// JSONL-based voice queue
pub struct VoiceQueue {
    /// Path to the queue JSONL file
    queue_path: PathBuf,

    /// Sibling advisory lock file path
    lock_path: PathBuf,

    /// Cache directory paired with this queue.
    cache_dir: PathBuf,

    /// Process-local mutex shared by handles for the same lock path
    process_lock: Arc<AsyncMutex<()>>,
}

fn process_lock_for(lock_path: &Path) -> Arc<AsyncMutex<()>> {
    static PROCESS_LOCKS: OnceLock<StdMutex<HashMap<PathBuf, Weak<AsyncMutex<()>>>>> =
        OnceLock::new();

    let locks = PROCESS_LOCKS.get_or_init(|| StdMutex::new(HashMap::new()));
    let mut locks = locks.lock().expect("voice queue process lock poisoned");

    if let Some(existing) = locks.get(lock_path).and_then(Weak::upgrade) {
        return existing;
    }

    let process_lock = Arc::new(AsyncMutex::new(()));
    locks.insert(lock_path.to_path_buf(), Arc::downgrade(&process_lock));
    process_lock
}

fn join_error_to_io(error: tokio::task::JoinError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, error.to_string())
}

fn approval_overrides(decision: &ApprovalDecision) -> Option<serde_json::Value> {
    let mut overrides = serde_json::Map::new();

    if let Some(speakers) = decision.speakers {
        overrides.insert("speakers".to_string(), serde_json::json!(speakers));
    }
    if let Some(names) = &decision.names {
        overrides.insert("names".to_string(), serde_json::json!(names));
    }
    if let Some(category) = &decision.category {
        overrides.insert("category".to_string(), serde_json::json!(category));
    }
    if let Some(hint) = &decision.hint {
        overrides.insert("hint".to_string(), serde_json::json!(hint));
    }

    if overrides.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(overrides))
    }
}

fn reset_trigger_is_auto(data: Option<&serde_json::Value>) -> bool {
    data.and_then(|value| value.get("trigger"))
        .and_then(|trigger| trigger.as_str())
        == Some("auto")
}

impl VoiceQueue {
    /// Create a new voice queue
    pub fn new(queue_path: PathBuf) -> Self {
        let lock_path = queue_path.with_extension("lock");
        let cache_dir = queue_path
            .parent()
            .map(|parent| parent.join("voice_cache"))
            .unwrap_or_else(|| PathBuf::from("voice_cache"));
        let process_lock = process_lock_for(&lock_path);
        Self {
            queue_path,
            lock_path,
            cache_dir,
            process_lock,
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
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

    /// Run a short mutation critical section under process-local and advisory file locks.
    pub async fn with_queue_lock<F, Fut, T>(&self, f: F) -> Result<T, VoiceQueueError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, VoiceQueueError>>,
    {
        let _process_guard = self.process_lock.lock().await;
        let lock_path = self.lock_path.clone();

        let lock_file = tokio::task::spawn_blocking(move || -> std::io::Result<std::fs::File> {
            if let Some(parent) = lock_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file = std::fs::OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .open(&lock_path)?;
            file.lock_exclusive()?;
            Ok(file)
        })
        .await
        .map_err(join_error_to_io)??;

        let result = f().await;

        let unlock_result = tokio::task::spawn_blocking(move || lock_file.unlock())
            .await
            .map_err(join_error_to_io)?;

        if let Err(error) = unlock_result {
            if result.is_ok() {
                return Err(VoiceQueueError::Io(error));
            }
        }

        result
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
        let (items, _) = self.replay_with_stats().await?;
        Ok(items)
    }

    /// Replay all events and return operational stats.
    ///
    /// No compaction exists yet; when compaction is added, preserve a `.bak`
    /// before rewriting so skipped-line evidence is not silently destroyed.
    pub async fn replay_with_stats(
        &self,
    ) -> Result<(HashMap<String, QueueItem>, ReplayStats), VoiceQueueError> {
        let mut items: HashMap<String, QueueItem> = HashMap::new();
        let mut stats = ReplayStats::default();

        if !self.queue_path.exists() {
            return Ok((items, stats));
        }

        let file = File::open(&self.queue_path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<QueueEvent>(&line) {
                Ok(event) => Self::apply_event(&mut items, event),
                Err(error) => {
                    stats.skipped_lines += 1;
                    tracing::warn!(
                        "Skipping malformed voice queue line in {}: {}",
                        self.queue_path.display(),
                        error
                    );
                }
            }
        }

        Ok((items, stats))
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
                                status: VoiceQueueStatus::AwaitingApproval,
                                data: item_data,
                                started_at: None,
                                completed_at: None,
                                error: None,
                                retry_count: 0,
                                approved_once: false,
                                chosen_engine: None,
                                overrides: None,
                                allow_large: false,
                                decided_at: None,
                                pid: None,
                                proc_start_epoch: None,
                                attempt_count: 0,
                                auto_reset_count: 0,
                            },
                        );
                    }
                }
            }
            QueueEventType::Approved => {
                if let Some(item) = items.get_mut(&event.item_id) {
                    let decision = event
                        .data
                        .and_then(|data| serde_json::from_value::<ApprovalDecision>(data).ok())
                        .unwrap_or_default();

                    item.status = VoiceQueueStatus::Pending;
                    item.approved_once = true;
                    item.decided_at = Some(decision.decided_at);
                    item.chosen_engine = decision.engine.clone();
                    item.allow_large = decision.allow_large;
                    item.overrides = approval_overrides(&decision);
                    item.completed_at = None;
                    item.error = None;
                }
            }
            QueueEventType::Skipped => {
                if let Some(item) = items.get_mut(&event.item_id) {
                    item.status = VoiceQueueStatus::Skipped;
                    item.completed_at = Some(event.timestamp);
                }
            }
            QueueEventType::ProcessingStarted => {
                if let Some(item) = items.get_mut(&event.item_id) {
                    let data = event
                        .data
                        .and_then(|data| serde_json::from_value::<ProcessingStartedData>(data).ok())
                        .unwrap_or_default();
                    item.status = VoiceQueueStatus::Processing;
                    item.started_at = Some(event.timestamp);
                    item.pid = data.pid;
                    item.proc_start_epoch = data.proc_start_epoch;
                    item.attempt_count += 1;
                }
            }
            QueueEventType::Completed => {
                if let Some(item) = items.get_mut(&event.item_id) {
                    item.status = VoiceQueueStatus::Done;
                    item.completed_at = Some(event.timestamp);
                    item.pid = None;
                    item.proc_start_epoch = None;
                }
            }
            QueueEventType::Failed => {
                if let Some(item) = items.get_mut(&event.item_id) {
                    item.status = VoiceQueueStatus::Failed;
                    item.completed_at = Some(event.timestamp);
                    item.pid = None;
                    item.proc_start_epoch = None;
                    if let Some(data) = event.data {
                        if let Some(error) = data.get("error").and_then(|e| e.as_str()) {
                            item.error = Some(error.to_string());
                        }
                    }
                }
            }
            QueueEventType::ResetForRetry => {
                if let Some(item) = items.get_mut(&event.item_id) {
                    if reset_trigger_is_auto(event.data.as_ref()) {
                        item.auto_reset_count += 1;
                    }
                    item.status = if item.approved_once {
                        VoiceQueueStatus::Pending
                    } else {
                        VoiceQueueStatus::AwaitingApproval
                    };
                    item.retry_count += 1;
                    item.error = None;
                    item.started_at = None;
                    item.completed_at = None;
                    item.pid = None;
                    item.proc_start_epoch = None;
                }
            }
            QueueEventType::Unknown => {}
        }
    }

    fn resolve_item_id(
        items: &HashMap<String, QueueItem>,
        id_or_prefix: &str,
    ) -> Result<String, VoiceQueueError> {
        if items.contains_key(id_or_prefix) {
            return Ok(id_or_prefix.to_string());
        }

        let matches: Vec<&String> = items
            .keys()
            .filter(|id| id.starts_with(id_or_prefix))
            .collect();

        match matches.as_slice() {
            [] => Err(VoiceQueueError::NotFound(id_or_prefix.to_string())),
            [id] => Ok((*id).clone()),
            _ => Err(VoiceQueueError::AmbiguousId(id_or_prefix.to_string())),
        }
    }

    /// Enqueue a new audio file (idempotent - returns existing if already queued)
    pub async fn enqueue(
        &self,
        file_path: &Path,
        file_size: u64,
        detected_at: DateTime<Utc>,
    ) -> Result<EnqueueResult, VoiceQueueError> {
        self.enqueue_with_metadata(
            file_path,
            file_size,
            detected_at,
            EnqueueMetadata::default(),
        )
        .await
    }

    pub async fn enqueue_with_metadata(
        &self,
        file_path: &Path,
        file_size: u64,
        detected_at: DateTime<Utc>,
        metadata: EnqueueMetadata,
    ) -> Result<EnqueueResult, VoiceQueueError> {
        // Compute content hash
        let hash = compute_file_hash(file_path).await?;

        let kind = metadata.kind.unwrap_or_else(|| "audio".to_string());
        let duration_seconds = if kind == "prediarized" {
            None
        } else {
            probe_duration(file_path).await
        };

        let cache_path = self.cache_path_for_id_and_source(&hash, file_path);
        // Skip when the copy already exists: item_id is a content hash, so an
        // existing cache file is byte-identical — without this check every scan
        // tick would re-copy the entire awaiting backlog (~142 items today).
        if !path_is_file(&cache_path).await && self.should_precopy_for_hash(&hash).await? {
            copy_to_cache(file_path, &cache_path).await?;
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
            duration_seconds,
            recorded_at: metadata.recorded_at,
            source: metadata.source,
            kind: Some(kind),
            private: metadata.private,
        };

        self.with_queue_lock(|| async {
            // Check if already exists
            let items = self.replay().await?;
            if let Some(existing) = items.get(&hash) {
                match existing.status {
                    VoiceQueueStatus::Done => {
                        return Ok(EnqueueResult::AlreadyProcessed(hash.clone()));
                    }
                    VoiceQueueStatus::Failed => {
                        if existing.auto_reset_count >= AUTO_RETRY_CAP {
                            return Ok(EnqueueResult::Quarantined(hash.clone()));
                        }

                        // Reset for one automatic retry
                        let event = QueueEvent {
                            timestamp: Utc::now(),
                            item_id: hash.clone(),
                            event_type: QueueEventType::ResetForRetry,
                            data: Some(serde_json::json!({ "trigger": "auto" })),
                        };
                        self.append_event(&event).await?;
                        return Ok(EnqueueResult::ResetForRetry(hash.clone()));
                    }
                    VoiceQueueStatus::Skipped => {
                        return Ok(EnqueueResult::AlreadySkipped(hash.clone()));
                    }
                    _ => {
                        return Ok(EnqueueResult::AlreadyQueued(hash.clone()));
                    }
                }
            }

            // Append enqueue event
            let event = QueueEvent {
                timestamp: Utc::now(),
                item_id: hash.clone(),
                event_type: QueueEventType::Enqueued,
                data: Some(serde_json::to_value(&item_data)?),
            };
            self.append_event(&event).await?;

            Ok(EnqueueResult::Queued(hash.clone()))
        })
        .await
    }

    pub fn cache_path_for_id_and_source(&self, item_id: &str, source_path: &Path) -> PathBuf {
        let ext = source_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!(".{ext}"))
            .unwrap_or_default();
        self.cache_dir.join(format!("{item_id}{ext}"))
    }

    pub fn cache_path_for_item(&self, item: &QueueItem) -> PathBuf {
        self.cache_path_for_id_and_source(&item.id, &item.data.file_path)
    }

    async fn should_precopy_for_hash(&self, hash: &str) -> Result<bool, VoiceQueueError> {
        let items = self.replay().await?;
        let Some(existing) = items.get(hash) else {
            return Ok(true);
        };

        Ok(match existing.status {
            VoiceQueueStatus::Done | VoiceQueueStatus::Skipped => false,
            VoiceQueueStatus::Failed if existing.auto_reset_count >= AUTO_RETRY_CAP => false,
            _ => true,
        })
    }

    pub async fn cache_sweep(&self) -> Result<CacheSweepReport, VoiceQueueError> {
        let items = self.replay().await?;
        let mut report = CacheSweepReport::default();

        for item in items.values() {
            if item.status != VoiceQueueStatus::Pending {
                continue;
            }

            let cache_path = self.cache_path_for_item(item);
            if path_is_file(&cache_path).await {
                continue;
            }

            if !path_is_file(&item.data.file_path).await {
                continue;
            }

            match copy_to_cache(&item.data.file_path, &cache_path).await {
                Ok(()) => report.backfilled += 1,
                Err(error) => {
                    report.errors += 1;
                    tracing::warn!(
                        item_id = %item.id,
                        error = %error,
                        "failed to backfill voice cache"
                    );
                }
            }
        }

        if let Err(error) = fs::create_dir_all(&self.cache_dir).await {
            report.errors += 1;
            tracing::warn!(
                path = %self.cache_dir.display(),
                error = %error,
                "failed to prepare voice cache for retention sweep"
            );
            return Ok(report);
        }

        let mut entries = match fs::read_dir(&self.cache_dir).await {
            Ok(entries) => entries,
            Err(error) => {
                report.errors += 1;
                tracing::warn!(
                    path = %self.cache_dir.display(),
                    error = %error,
                    "failed to read voice cache for retention sweep"
                );
                return Ok(report);
            }
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let Some(item_id) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let Some(item) = items.get(item_id) else {
                continue;
            };
            if !is_terminal_status(item.status) {
                continue;
            }
            if !cache_file_is_older_than(&path, CACHE_RETENTION_SECS).await {
                continue;
            }

            match fs::remove_file(&path).await {
                Ok(()) => report.retained += 1,
                Err(error) => {
                    report.errors += 1;
                    tracing::warn!(
                        path = %path.display(),
                        item_id = %item.id,
                        error = %error,
                        "failed to delete expired voice cache file"
                    );
                }
            }
        }

        Ok(report)
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

    /// Get all items waiting for human approval
    pub async fn get_awaiting(&self) -> Result<Vec<QueueItem>, VoiceQueueError> {
        let items = self.replay().await?;
        let mut awaiting: Vec<QueueItem> = items
            .into_values()
            .filter(|item| item.status == VoiceQueueStatus::AwaitingApproval)
            .collect();

        // Sort by detected_at (oldest first)
        awaiting.sort_by(|a, b| a.data.detected_at.cmp(&b.data.detected_at));

        Ok(awaiting)
    }

    /// Mark an item as processing
    pub async fn mark_processing(&self, id: &str) -> Result<(), VoiceQueueError> {
        self.with_queue_lock(|| async {
            let items = self.replay().await?;
            let item = items
                .get(id)
                .ok_or_else(|| VoiceQueueError::NotFound(id.to_string()))?;

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
        })
        .await
    }

    /// Mark an item as processing and record M3 reaper metadata.
    pub async fn mark_processing_with_attempt(&self, id: &str) -> Result<(), VoiceQueueError> {
        self.with_queue_lock(|| async {
            let items = self.replay().await?;
            let item = items
                .get(id)
                .ok_or_else(|| VoiceQueueError::NotFound(id.to_string()))?;

            if item.status != VoiceQueueStatus::Pending {
                return Err(VoiceQueueError::InvalidTransition {
                    from: item.status,
                    to: VoiceQueueStatus::Processing,
                });
            }

            let now = Utc::now();
            let data = ProcessingStartedData {
                pid: Some(std::process::id()),
                proc_start_epoch: Some(now.timestamp()),
            };
            let event = QueueEvent {
                timestamp: now,
                item_id: id.to_string(),
                event_type: QueueEventType::ProcessingStarted,
                data: Some(serde_json::to_value(data)?),
            };
            self.append_event(&event).await?;

            Ok(())
        })
        .await
    }

    /// Mark an item as done
    pub async fn mark_done(&self, id: &str) -> Result<(), VoiceQueueError> {
        self.with_queue_lock(|| async {
            let items = self.replay().await?;
            items
                .get(id)
                .ok_or_else(|| VoiceQueueError::NotFound(id.to_string()))?;

            let event = QueueEvent {
                timestamp: Utc::now(),
                item_id: id.to_string(),
                event_type: QueueEventType::Completed,
                data: None,
            };
            self.append_event(&event).await?;

            Ok(())
        })
        .await
    }

    /// Mark an item as failed
    pub async fn mark_failed(&self, id: &str, error: &str) -> Result<(), VoiceQueueError> {
        self.with_queue_lock(|| async {
            let items = self.replay().await?;
            let item = items
                .get(id)
                .ok_or_else(|| VoiceQueueError::NotFound(id.to_string()))?;

            if item.status != VoiceQueueStatus::Processing
                && item.status != VoiceQueueStatus::Pending
            {
                return Err(VoiceQueueError::InvalidTransition {
                    from: item.status,
                    to: VoiceQueueStatus::Failed,
                });
            }

            let event = QueueEvent {
                timestamp: Utc::now(),
                item_id: id.to_string(),
                event_type: QueueEventType::Failed,
                data: Some(serde_json::json!({ "error": error })),
            };
            self.append_event(&event).await?;

            Ok(())
        })
        .await
    }

    /// Approve an item for processing.
    pub async fn approve(
        &self,
        id: &str,
        decision: ApprovalDecision,
    ) -> Result<(), VoiceQueueError> {
        self.with_queue_lock(|| async {
            let items = self.replay().await?;
            let item_id = Self::resolve_item_id(&items, id)?;
            let item = items
                .get(&item_id)
                .ok_or_else(|| VoiceQueueError::NotFound(id.to_string()))?;
            let from = item.status;

            if from != VoiceQueueStatus::AwaitingApproval && from != VoiceQueueStatus::Skipped {
                return Err(VoiceQueueError::InvalidTransition {
                    from,
                    to: VoiceQueueStatus::Pending,
                });
            }

            let event = QueueEvent {
                timestamp: Utc::now(),
                item_id,
                event_type: QueueEventType::Approved,
                data: Some(serde_json::to_value(&decision)?),
            };
            self.append_event(&event).await?;

            Ok(())
        })
        .await
    }

    /// Skip an awaiting item.
    pub async fn skip(&self, id: &str, reason: Option<String>) -> Result<(), VoiceQueueError> {
        let data = reason.map(|reason| serde_json::json!({ "reason": reason }));

        self.with_queue_lock(|| async {
            let items = self.replay().await?;
            let item_id = Self::resolve_item_id(&items, id)?;
            let item = items
                .get(&item_id)
                .ok_or_else(|| VoiceQueueError::NotFound(id.to_string()))?;
            let from = item.status;

            if from != VoiceQueueStatus::AwaitingApproval {
                return Err(VoiceQueueError::InvalidTransition {
                    from,
                    to: VoiceQueueStatus::Skipped,
                });
            }

            let event = QueueEvent {
                timestamp: Utc::now(),
                item_id,
                event_type: QueueEventType::Skipped,
                data: data.clone(),
            };
            self.append_event(&event).await?;

            Ok(())
        })
        .await
    }

    /// Retry a failed or skipped item.
    pub async fn retry(&self, id: &str) -> Result<(), VoiceQueueError> {
        self.with_queue_lock(|| async {
            let items = self.replay().await?;
            let item_id = Self::resolve_item_id(&items, id)?;
            let item = items
                .get(&item_id)
                .ok_or_else(|| VoiceQueueError::NotFound(id.to_string()))?;
            let from = item.status;
            let to = if item.approved_once {
                VoiceQueueStatus::Pending
            } else {
                VoiceQueueStatus::AwaitingApproval
            };

            if from != VoiceQueueStatus::Failed && from != VoiceQueueStatus::Skipped {
                return Err(VoiceQueueError::InvalidTransition { from, to });
            }

            let event = QueueEvent {
                timestamp: Utc::now(),
                item_id,
                event_type: QueueEventType::ResetForRetry,
                data: Some(serde_json::json!({ "trigger": "human" })),
            };
            self.append_event(&event).await?;

            Ok(())
        })
        .await
    }

    /// Reap stale `Processing` items and either reset once or quarantine at the cap.
    pub async fn reap_stale_processing(&self) -> Result<ReapReport, VoiceQueueError> {
        self.with_queue_lock(|| async {
            let items = self.replay().await?;
            let now = Utc::now();
            let mut report = ReapReport::default();
            let processing_items: Vec<QueueItem> = items
                .values()
                .filter(|item| item.status == VoiceQueueStatus::Processing)
                .cloned()
                .collect();

            for item in processing_items {
                if !is_stale_processing_item(&item, now).await {
                    continue;
                }

                if item.auto_reset_count >= AUTO_RETRY_CAP {
                    let event = QueueEvent {
                        timestamp: Utc::now(),
                        item_id: item.id.clone(),
                        event_type: QueueEventType::Failed,
                        data: Some(serde_json::json!({ "error": "reaped at retry cap" })),
                    };
                    self.append_event(&event).await?;
                    report.quarantined.push(item);
                } else {
                    let event = QueueEvent {
                        timestamp: Utc::now(),
                        item_id: item.id.clone(),
                        event_type: QueueEventType::ResetForRetry,
                        data: Some(serde_json::json!({ "trigger": "auto" })),
                    };
                    self.append_event(&event).await?;
                    report.reset_for_retry.push(item.id);
                }
            }

            Ok(report)
        })
        .await
    }

    /// Get queue status summary
    pub async fn status(&self) -> Result<QueueStatus, VoiceQueueError> {
        let items = self.replay().await?;

        let mut status = QueueStatus::default();
        for item in items.values() {
            match item.status {
                VoiceQueueStatus::Pending => status.pending += 1,
                VoiceQueueStatus::AwaitingApproval => status.awaiting += 1,
                VoiceQueueStatus::Processing => status.processing += 1,
                VoiceQueueStatus::Done => status.done += 1,
                VoiceQueueStatus::Failed => status.failed += 1,
                VoiceQueueStatus::Skipped => status.skipped += 1,
            }
        }

        // Get recent items (last 5)
        let mut all_items: Vec<&QueueItem> = items.values().collect();
        all_items.sort_by(|a, b| b.data.detected_at.cmp(&a.data.detected_at));
        status.recent = all_items.into_iter().take(5).cloned().collect();

        Ok(status)
    }

    /// Get a specific item by ID
    pub async fn get(&self, id: &str) -> Result<Option<QueueItem>, VoiceQueueError> {
        let items = self.replay().await?;
        Ok(items.get(id).cloned())
    }
}

async fn is_stale_processing_item(item: &QueueItem, now: DateTime<Utc>) -> bool {
    let started_too_old = item
        .started_at
        .map(|started| {
            now.signed_duration_since(started).num_seconds() > STALE_PROCESSING_BACKSTOP_SECS
        })
        .unwrap_or(false);

    if started_too_old {
        return true;
    }

    let Some(pid) = item.pid else {
        return started_too_old;
    };

    if process_is_esrch(pid) {
        return true;
    }

    if let Some(recorded_epoch) = item.proc_start_epoch {
        if let Some(elapsed) = process_elapsed(pid).await {
            let process_start_epoch = now.timestamp() - elapsed.as_secs() as i64;
            if process_start_epoch > recorded_epoch + PID_REUSE_GRACE_SECS {
                return true;
            }
        }
    }

    false
}

fn process_is_esrch(pid: u32) -> bool {
    let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if rc == 0 {
        return false;
    }

    std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

async fn process_elapsed(pid: u32) -> Option<Duration> {
    let output = tokio::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "etime="])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ps_etime(stdout.trim())
}

/// Parse `ps -o etime=` output: `[[dd-]hh:]mm:ss`, plus macOS's short `ss`.
pub fn parse_ps_etime(input: &str) -> Option<Duration> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (days, time_part) = if let Some((days, rest)) = trimmed.split_once('-') {
        (days.parse::<u64>().ok()?, rest)
    } else {
        (0, trimmed)
    };

    let parts: Vec<&str> = time_part.split(':').collect();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [seconds] => (0, 0, seconds.parse::<u64>().ok()?),
        [minutes, seconds] => (
            0,
            minutes.parse::<u64>().ok()?,
            seconds.parse::<u64>().ok()?,
        ),
        [hours, minutes, seconds] => (
            hours.parse::<u64>().ok()?,
            minutes.parse::<u64>().ok()?,
            seconds.parse::<u64>().ok()?,
        ),
        _ => return None,
    };

    Some(Duration::from_secs(
        days * 24 * 60 * 60 + hours * 60 * 60 + minutes * 60 + seconds,
    ))
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

    /// Already skipped by a human
    AlreadySkipped(String),

    /// Failed item has reached the automatic retry cap
    Quarantined(String),
}

impl EnqueueResult {
    /// Get the item ID regardless of result type
    pub fn id(&self) -> &str {
        match self {
            Self::Queued(id)
            | Self::AlreadyQueued(id)
            | Self::AlreadyProcessed(id)
            | Self::ResetForRetry(id)
            | Self::AlreadySkipped(id)
            | Self::Quarantined(id) => id,
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
    pub awaiting: usize,
    pub pending: usize,
    pub processing: usize,
    pub done: usize,
    pub failed: usize,
    pub skipped: usize,
    pub recent: Vec<QueueItem>,
}

impl QueueStatus {
    /// Total items in queue
    pub fn total(&self) -> usize {
        self.awaiting + self.pending + self.processing + self.done + self.failed + self.skipped
    }
}

async fn copy_to_cache(source: &Path, destination: &Path) -> Result<(), VoiceQueueError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::copy(source, destination)
        .await
        .map(|_| ())
        .map_err(|source_error| VoiceQueueError::CacheCopy {
            from: source.to_path_buf(),
            to: destination.to_path_buf(),
            source: source_error,
        })
}

async fn path_is_file(path: &Path) -> bool {
    fs::metadata(path)
        .await
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

fn is_terminal_status(status: VoiceQueueStatus) -> bool {
    matches!(
        status,
        VoiceQueueStatus::Done | VoiceQueueStatus::Skipped | VoiceQueueStatus::Failed
    )
}

async fn cache_file_is_older_than(path: &Path, max_age_secs: u64) -> bool {
    let Ok(metadata) = fs::metadata(path).await else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    modified
        .elapsed()
        .map(|age| age > Duration::from_secs(max_age_secs))
        .unwrap_or(false)
}

/// Compute SHA256 hash of file content using streaming (8KB chunks)
/// Returns first 12 hex characters of the hash.
/// Uses streaming to avoid loading entire file into memory.
pub async fn compute_file_hash(path: &Path) -> Result<String, std::io::Error> {
    use tokio::io::AsyncReadExt;

    let file = tokio::fs::File::open(path).await?;
    let mut reader = tokio::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192]; // 8KB chunks

    loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result)[..12].to_string())
}

/// Probe audio duration in seconds using ffprobe
pub async fn probe_duration(path: &Path) -> Option<f32> {
    let output = tokio::process::Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .await
        .ok()?;

    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Normalize audio file if needed (.qta → .m4a)
/// Returns the path to use for hashing/processing.
/// For .m4a files, returns the original path unchanged.
/// For .qta files, converts to .m4a and caches in voice_cache directory.
///
/// Security: ffmpeg args are hardcoded, no user input in command construction.
pub async fn normalize_audio(input: &Path) -> Result<PathBuf> {
    // If not .qta, return original path unchanged
    if input.extension().map(|e| e != "qta").unwrap_or(true) {
        return Ok(input.to_path_buf());
    }

    // Get cache directory
    let cache_dir = crate::config::voice_cache_dir()?;
    fs::create_dir_all(&cache_dir).await?;

    // Compute hash of input file to create cache filename
    let hash = compute_file_hash(input).await?;
    let output = cache_dir.join(format!("{}.m4a", hash));

    // If already cached, return cached path
    if output.exists() {
        tracing::debug!("Using cached normalized audio: {}", output.display());
        return Ok(output);
    }

    // Convert .qta → .m4a using ffmpeg with hardcoded args (security)
    tracing::info!("Normalizing .qta → .m4a: {}", input.display());
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-i",
            input.to_str().unwrap_or(""),
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-y", // Overwrite output
        ])
        .arg(&output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("ffmpeg normalization failed for {}", input.display());
    }

    Ok(output)
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

    async fn create_audio_file(temp: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
        let audio_path = temp.path().join(name);
        tokio::fs::write(&audio_path, bytes).await.unwrap();
        audio_path
    }

    fn test_item_data(audio_path: &Path, file_size: u64) -> QueueItemData {
        QueueItemData {
            file_path: audio_path.to_path_buf(),
            file_name: audio_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            file_size,
            detected_at: Utc::now(),
            duration_seconds: None,
            recorded_at: None,
            source: None,
            kind: Some("audio".to_string()),
            private: false,
        }
    }

    fn queue_event(
        item_id: &str,
        event_type: QueueEventType,
        data: Option<serde_json::Value>,
    ) -> QueueEvent {
        QueueEvent {
            timestamp: Utc::now(),
            item_id: item_id.to_string(),
            event_type,
            data,
        }
    }

    async fn append_jsonl_line(path: &Path, line: &str) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .unwrap();
        file.write_all(line.as_bytes()).await.unwrap();
        file.write_all(b"\n").await.unwrap();
        file.flush().await.unwrap();
    }

    async fn seed_enqueued(queue: &VoiceQueue, id: &str, audio_path: &Path, file_size: u64) {
        let event = queue_event(
            id,
            QueueEventType::Enqueued,
            Some(serde_json::to_value(test_item_data(audio_path, file_size)).unwrap()),
        );
        queue.append_event(&event).await.unwrap();
    }

    async fn seed_failed_without_approval(queue: &VoiceQueue, temp: &TempDir) -> (String, PathBuf) {
        let audio_path = create_audio_file(temp, "failed.m4a", b"failed audio").await;
        let id = compute_file_hash(&audio_path).await.unwrap();
        seed_enqueued(queue, &id, &audio_path, 12).await;
        queue
            .append_event(&queue_event(
                &id,
                QueueEventType::Failed,
                Some(serde_json::json!({ "error": "seeded failure" })),
            ))
            .await
            .unwrap();
        (id, audio_path)
    }

    async fn seed_processing_event(
        queue: &VoiceQueue,
        id: &str,
        audio_path: &Path,
        pid: Option<u32>,
        proc_start_epoch: i64,
    ) {
        seed_enqueued(queue, id, audio_path, 12).await;
        queue
            .append_event(&queue_event(id, QueueEventType::Approved, None))
            .await
            .unwrap();
        queue
            .append_event(&queue_event(
                id,
                QueueEventType::ProcessingStarted,
                Some(serde_json::json!({
                    "pid": pid,
                    "proc_start_epoch": proc_start_epoch
                })),
            ))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_enqueue_new_item() {
        let (queue, temp) = create_test_queue().await;

        // Create a test audio file
        let audio_path = create_audio_file(&temp, "test.m4a", b"fake audio content").await;

        let result = queue.enqueue(&audio_path, 18, Utc::now()).await.unwrap();

        assert!(result.is_new());

        // Verify it is held at the approval gate.
        let status = queue.status().await.unwrap();
        assert_eq!(status.awaiting, 1);
        assert_eq!(status.pending, 0);
        assert_eq!(status.done, 0);
    }

    #[tokio::test]
    async fn test_precopy_exists_after_enqueue() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "copy.m4a", b"copy me").await;

        let result = queue.enqueue(&audio_path, 7, Utc::now()).await.unwrap();
        let cache_path = queue.cache_path_for_id_and_source(result.id(), &audio_path);

        assert_eq!(tokio::fs::read(&cache_path).await.unwrap(), b"copy me");
    }

    #[tokio::test]
    async fn test_enqueue_metadata_replays_additive_fields() {
        let (queue, temp) = create_test_queue().await;
        let transcript = create_audio_file(&temp, "meeting.md", b"# Meeting").await;
        let recorded_at = Utc::now();

        let result = queue
            .enqueue_with_metadata(
                &transcript,
                9,
                Utc::now(),
                EnqueueMetadata {
                    source: Some("transcribex-exports".to_string()),
                    kind: Some("prediarized".to_string()),
                    private: true,
                    recorded_at: Some(recorded_at),
                },
            )
            .await
            .unwrap();
        let item = queue.get(result.id()).await.unwrap().unwrap();

        assert_eq!(item.data.source.as_deref(), Some("transcribex-exports"));
        assert_eq!(item.data.kind.as_deref(), Some("prediarized"));
        assert!(item.data.private);
        assert_eq!(item.data.recorded_at, Some(recorded_at));
        assert!(item.data.duration_seconds.is_none());
    }

    #[tokio::test]
    async fn test_backfill_copies_pending_item_missing_cache() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "backfill.m4a", b"backfill").await;
        let result = queue.enqueue(&audio_path, 8, Utc::now()).await.unwrap();
        let id = result.id().to_string();
        queue
            .approve(&id, ApprovalDecision::default())
            .await
            .unwrap();
        let cache_path = queue.cache_path_for_id_and_source(&id, &audio_path);
        tokio::fs::remove_file(&cache_path).await.unwrap();

        let report = queue.cache_sweep().await.unwrap();

        assert_eq!(report.backfilled, 1);
        assert_eq!(tokio::fs::read(&cache_path).await.unwrap(), b"backfill");
    }

    #[tokio::test]
    async fn test_retention_deletes_only_terminal_old_cache_files() {
        use std::time::SystemTime;

        use filetime::{set_file_mtime, FileTime};

        let (queue, temp) = create_test_queue().await;
        let terminal_path = create_audio_file(&temp, "terminal.m4a", b"terminal").await;
        let pending_path = create_audio_file(&temp, "pending.m4a", b"pending").await;
        let terminal_id = queue
            .enqueue(&terminal_path, 8, Utc::now())
            .await
            .unwrap()
            .id()
            .to_string();
        let pending_id = queue
            .enqueue(&pending_path, 7, Utc::now())
            .await
            .unwrap()
            .id()
            .to_string();
        queue.skip(&terminal_id, None).await.unwrap();
        queue
            .approve(&pending_id, ApprovalDecision::default())
            .await
            .unwrap();

        let terminal_cache = queue.cache_path_for_id_and_source(&terminal_id, &terminal_path);
        let pending_cache = queue.cache_path_for_id_and_source(&pending_id, &pending_path);
        let orphan_cache = queue.cache_dir().join("orphan123456.m4a");
        tokio::fs::write(&orphan_cache, b"orphan").await.unwrap();

        let old_time = FileTime::from_unix_time(
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                - (CACHE_RETENTION_SECS as i64)
                - 60,
            0,
        );
        set_file_mtime(&terminal_cache, old_time).unwrap();
        set_file_mtime(&pending_cache, old_time).unwrap();
        set_file_mtime(&orphan_cache, old_time).unwrap();

        let report = queue.cache_sweep().await.unwrap();

        assert_eq!(report.retained, 1);
        assert!(!terminal_cache.exists());
        assert!(pending_cache.exists());
        assert!(orphan_cache.exists());
    }

    #[tokio::test]
    async fn test_idempotent_enqueue() {
        let (queue, temp) = create_test_queue().await;

        let audio_path = create_audio_file(&temp, "test.m4a", b"fake audio content").await;

        // Enqueue twice
        let result1 = queue.enqueue(&audio_path, 18, Utc::now()).await.unwrap();
        let result2 = queue.enqueue(&audio_path, 18, Utc::now()).await.unwrap();

        assert!(result1.is_new());
        assert!(!result2.is_new());
        assert_eq!(result1.id(), result2.id());

        // Should still only have 1 awaiting approval
        let status = queue.status().await.unwrap();
        assert_eq!(status.awaiting, 1);
        assert_eq!(status.pending, 0);
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let (queue, temp) = create_test_queue().await;

        let audio_path = create_audio_file(&temp, "test.m4a", b"fake audio content").await;

        let result = queue.enqueue(&audio_path, 18, Utc::now()).await.unwrap();
        let id = result.id().to_string();

        queue
            .approve(&id, ApprovalDecision::default())
            .await
            .unwrap();

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

        let (id, audio_path) = seed_failed_without_approval(&queue, &temp).await;

        let item = queue.get(&id).await.unwrap().unwrap();
        assert_eq!(item.status, VoiceQueueStatus::Failed);
        assert_eq!(item.error, Some("seeded failure".to_string()));

        // Re-enqueue should reset for retry
        let result2 = queue.enqueue(&audio_path, 18, Utc::now()).await.unwrap();
        assert!(matches!(result2, EnqueueResult::ResetForRetry(_)));

        let item = queue.get(&id).await.unwrap().unwrap();
        assert_eq!(item.status, VoiceQueueStatus::AwaitingApproval);
        assert_eq!(item.retry_count, 1);
        assert_eq!(item.auto_reset_count, 1);
    }

    #[tokio::test]
    async fn test_enqueue_lands_awaiting_approval() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "new.m4a", b"new audio").await;

        let result = queue.enqueue(&audio_path, 9, Utc::now()).await.unwrap();
        let item = queue.get(result.id()).await.unwrap().unwrap();
        let status = queue.status().await.unwrap();

        assert_eq!(item.status, VoiceQueueStatus::AwaitingApproval);
        assert_eq!(status.awaiting, 1);
        assert_eq!(status.pending, 0);
    }

    #[tokio::test]
    async fn test_awaiting_item_excluded_from_get_pending() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "awaiting.m4a", b"awaiting audio").await;

        let result = queue.enqueue(&audio_path, 14, Utc::now()).await.unwrap();
        let pending = queue.get_pending().await.unwrap();
        let awaiting = queue.get_awaiting().await.unwrap();

        assert!(pending.is_empty());
        assert_eq!(awaiting.len(), 1);
        assert_eq!(awaiting[0].id, result.id());
    }

    #[tokio::test]
    async fn test_mark_processing_rejects_awaiting() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "reject.m4a", b"reject audio").await;

        let result = queue.enqueue(&audio_path, 12, Utc::now()).await.unwrap();
        let error = queue.mark_processing(result.id()).await.unwrap_err();

        assert!(matches!(
            error,
            VoiceQueueError::InvalidTransition {
                from: VoiceQueueStatus::AwaitingApproval,
                to: VoiceQueueStatus::Processing,
            }
        ));
    }

    #[tokio::test]
    async fn test_approve_transitions_awaiting_to_pending() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "approve.m4a", b"approve audio").await;

        let result = queue.enqueue(&audio_path, 13, Utc::now()).await.unwrap();
        let id = result.id().to_string();
        queue
            .approve(&id, ApprovalDecision::default())
            .await
            .unwrap();

        let item = queue.get(&id).await.unwrap().unwrap();
        let pending = queue.get_pending().await.unwrap();

        assert_eq!(item.status, VoiceQueueStatus::Pending);
        assert!(item.approved_once);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        queue.mark_processing(&id).await.unwrap();
    }

    #[tokio::test]
    async fn test_approve_stores_decision_hints() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "decision.m4a", b"decision audio").await;

        let result = queue.enqueue(&audio_path, 14, Utc::now()).await.unwrap();
        let id = result.id().to_string();
        queue
            .approve(
                &id,
                ApprovalDecision {
                    engine: Some("whisperx".to_string()),
                    speakers: Some(2),
                    allow_large: true,
                    ..ApprovalDecision::default()
                },
            )
            .await
            .unwrap();

        let item = queue.get(&id).await.unwrap().unwrap();
        let overrides = item.overrides.as_ref().unwrap();

        assert_eq!(item.chosen_engine.as_deref(), Some("whisperx"));
        assert!(item.allow_large);
        assert_eq!(overrides.get("speakers").and_then(|v| v.as_u64()), Some(2));
        assert!(item.decided_at.is_some());
    }

    #[tokio::test]
    async fn test_skip_transitions_awaiting_to_skipped() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "skip.m4a", b"skip audio").await;

        let result = queue.enqueue(&audio_path, 10, Utc::now()).await.unwrap();
        let id = result.id().to_string();
        queue
            .skip(&id, Some("not needed".to_string()))
            .await
            .unwrap();

        let item = queue.get(&id).await.unwrap().unwrap();

        assert_eq!(item.status, VoiceQueueStatus::Skipped);
        assert!(item.completed_at.is_some());
        assert!(queue.get_pending().await.unwrap().is_empty());
        assert!(queue.get_awaiting().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_skipped_item_not_reoffered_on_reenqueue() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "sticky-skip.m4a", b"sticky skip").await;

        let result = queue.enqueue(&audio_path, 11, Utc::now()).await.unwrap();
        let id = result.id().to_string();
        queue.skip(&id, None).await.unwrap();
        let second = queue.enqueue(&audio_path, 11, Utc::now()).await.unwrap();
        let item = queue.get(&id).await.unwrap().unwrap();
        let status = queue.status().await.unwrap();

        assert!(matches!(second, EnqueueResult::AlreadySkipped(_)));
        assert_eq!(item.status, VoiceQueueStatus::Skipped);
        assert_eq!(status.awaiting, 0);
    }

    #[tokio::test]
    async fn test_retry_failed_approved_item_skips_gate() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "approved-failed.m4a", b"approved failed").await;

        let result = queue.enqueue(&audio_path, 15, Utc::now()).await.unwrap();
        let id = result.id().to_string();
        queue
            .approve(&id, ApprovalDecision::default())
            .await
            .unwrap();
        queue.mark_processing(&id).await.unwrap();
        queue.mark_failed(&id, "boom").await.unwrap();
        queue.retry(&id).await.unwrap();

        let item = queue.get(&id).await.unwrap().unwrap();
        let pending = queue.get_pending().await.unwrap();

        assert_eq!(item.status, VoiceQueueStatus::Pending);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
    }

    #[tokio::test]
    async fn test_retry_failed_unapproved_item_reenters_gate() {
        let (queue, temp) = create_test_queue().await;
        let (id, _) = seed_failed_without_approval(&queue, &temp).await;

        queue.retry(&id).await.unwrap();
        let item = queue.get(&id).await.unwrap().unwrap();

        assert_eq!(item.status, VoiceQueueStatus::AwaitingApproval);
    }

    #[tokio::test]
    async fn test_enqueue_reset_unapproved_failed_reenters_gate() {
        let (queue, temp) = create_test_queue().await;
        let (id, audio_path) = seed_failed_without_approval(&queue, &temp).await;

        let result = queue.enqueue(&audio_path, 12, Utc::now()).await.unwrap();
        let item = queue.get(&id).await.unwrap().unwrap();
        let status = queue.status().await.unwrap();

        assert!(matches!(result, EnqueueResult::ResetForRetry(_)));
        assert_eq!(item.status, VoiceQueueStatus::AwaitingApproval);
        assert_eq!(status.awaiting, 1);
        assert_eq!(status.pending, 0);
    }

    #[tokio::test]
    async fn test_legacy_enqueued_replays_as_awaiting() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "legacy.m4a", b"legacy audio").await;
        let id = "legacy123456";

        seed_enqueued(&queue, id, &audio_path, 12).await;
        let item = queue.get(id).await.unwrap().unwrap();

        assert_eq!(item.status, VoiceQueueStatus::AwaitingApproval);
        assert_eq!(item.data.file_name, "legacy.m4a");
        assert!(!item.approved_once);
        assert!(item.chosen_engine.is_none());
        assert!(item.overrides.is_none());
        assert!(!item.allow_large);
    }

    #[tokio::test]
    async fn test_unknown_event_type_is_tolerated() {
        let (queue, temp) = create_test_queue().await;
        let audio1 = create_audio_file(&temp, "one.m4a", b"one").await;
        let audio2 = create_audio_file(&temp, "two.m4a", b"two").await;
        let first = queue_event(
            "one123456789",
            QueueEventType::Enqueued,
            Some(serde_json::to_value(test_item_data(&audio1, 3)).unwrap()),
        );
        let second = queue_event(
            "two123456789",
            QueueEventType::Enqueued,
            Some(serde_json::to_value(test_item_data(&audio2, 3)).unwrap()),
        );

        append_jsonl_line(&queue.queue_path, &serde_json::to_string(&first).unwrap()).await;
        append_jsonl_line(
            &queue.queue_path,
            &serde_json::json!({
                "timestamp": Utc::now(),
                "item_id": "one123456789",
                "event_type": "approval_expired"
            })
            .to_string(),
        )
        .await;
        append_jsonl_line(&queue.queue_path, &serde_json::to_string(&second).unwrap()).await;

        let items = queue.replay().await.unwrap();

        assert_eq!(items.len(), 2);
        assert!(items.contains_key("one123456789"));
        assert!(items.contains_key("two123456789"));
    }

    #[tokio::test]
    async fn test_torn_final_line_is_skipped_and_warned() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "valid.m4a", b"valid").await;
        let valid = queue_event(
            "valid123456",
            QueueEventType::Enqueued,
            Some(serde_json::to_value(test_item_data(&audio_path, 5)).unwrap()),
        );
        let contents = format!(
            "{}\n{{\"timestamp\":\"{}\",\"item_id\":\"broken\"",
            serde_json::to_string(&valid).unwrap(),
            Utc::now().to_rfc3339()
        );
        tokio::fs::write(&queue.queue_path, contents).await.unwrap();

        let items = queue.replay().await.unwrap();

        assert_eq!(items.len(), 1);
        assert!(items.contains_key("valid123456"));
    }

    #[tokio::test]
    async fn test_torn_interior_line_skipped_and_counted() {
        let (queue, temp) = create_test_queue().await;
        let audio1 = create_audio_file(&temp, "valid1.m4a", b"valid1").await;
        let audio2 = create_audio_file(&temp, "valid2.m4a", b"valid2").await;
        let first = queue_event(
            "valid111111",
            QueueEventType::Enqueued,
            Some(serde_json::to_value(test_item_data(&audio1, 6)).unwrap()),
        );
        let second = queue_event(
            "valid222222",
            QueueEventType::Enqueued,
            Some(serde_json::to_value(test_item_data(&audio2, 6)).unwrap()),
        );
        let contents = format!(
            "{}\n{{\"timestamp\":\"broken\"\n{}\n",
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        tokio::fs::write(&queue.queue_path, contents).await.unwrap();

        let (items, stats) = queue.replay_with_stats().await.unwrap();

        assert_eq!(items.len(), 2);
        assert!(items.contains_key("valid111111"));
        assert!(items.contains_key("valid222222"));
        assert_eq!(stats.skipped_lines, 1);
    }

    #[tokio::test]
    async fn test_with_queue_lock_serializes_concurrent_enqueue() {
        let temp = TempDir::new().unwrap();
        let queue_path = temp.path().join("test_queue.jsonl");
        let queue1 = VoiceQueue::new(queue_path.clone());
        let queue2 = VoiceQueue::new(queue_path.clone());
        let audio_path = create_audio_file(&temp, "same.m4a", b"same bytes").await;

        let (first, second) = tokio::join!(
            queue1.enqueue(&audio_path, 10, Utc::now()),
            queue2.enqueue(&audio_path, 10, Utc::now())
        );

        first.unwrap();
        second.unwrap();

        let contents = tokio::fs::read_to_string(&queue_path).await.unwrap();
        let enqueued_count = contents
            .lines()
            .filter(|line| {
                let value: serde_json::Value = serde_json::from_str(line).unwrap();
                value.get("event_type").and_then(|v| v.as_str()) == Some("enqueued")
            })
            .count();
        let status = queue1.status().await.unwrap();

        assert_eq!(enqueued_count, 1);
        assert_eq!(status.awaiting, 1);
    }

    #[test]
    fn test_parse_ps_etime() {
        assert_eq!(parse_ps_etime("03:07").unwrap().as_secs(), 187);
        assert_eq!(parse_ps_etime("1-02:03:04").unwrap().as_secs(), 93_784);
        assert_eq!(parse_ps_etime("59").unwrap().as_secs(), 59);
        assert!(parse_ps_etime("").is_none());
        assert!(parse_ps_etime("bad").is_none());
    }

    #[tokio::test]
    async fn test_auto_vs_human_trigger_counting() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "trigger.m4a", b"trigger").await;
        let id = "trigger12345";
        seed_enqueued(&queue, id, &audio_path, 7).await;

        queue
            .append_event(&queue_event(
                id,
                QueueEventType::ResetForRetry,
                Some(serde_json::json!({ "trigger": "human" })),
            ))
            .await
            .unwrap();
        queue
            .append_event(&queue_event(id, QueueEventType::ResetForRetry, None))
            .await
            .unwrap();
        queue
            .append_event(&queue_event(
                id,
                QueueEventType::ResetForRetry,
                Some(serde_json::json!({ "trigger": "auto" })),
            ))
            .await
            .unwrap();

        let item = queue.get(id).await.unwrap().unwrap();
        assert_eq!(item.retry_count, 3);
        assert_eq!(item.auto_reset_count, 1);
    }

    #[tokio::test]
    async fn test_enqueue_quarantined_at_auto_retry_cap() {
        let (queue, temp) = create_test_queue().await;
        let (id, audio_path) = seed_failed_without_approval(&queue, &temp).await;
        queue
            .append_event(&queue_event(
                &id,
                QueueEventType::ResetForRetry,
                Some(serde_json::json!({ "trigger": "auto" })),
            ))
            .await
            .unwrap();
        queue
            .append_event(&queue_event(
                &id,
                QueueEventType::Failed,
                Some(serde_json::json!({ "error": "failed again" })),
            ))
            .await
            .unwrap();

        let result = queue.enqueue(&audio_path, 12, Utc::now()).await.unwrap();
        let item = queue.get(&id).await.unwrap().unwrap();

        assert!(matches!(result, EnqueueResult::Quarantined(_)));
        assert_eq!(item.status, VoiceQueueStatus::Failed);
        assert_eq!(item.auto_reset_count, 1);
    }

    #[tokio::test]
    async fn test_mark_failed_guard_rejects_terminal_states() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "done.m4a", b"done").await;
        let result = queue.enqueue(&audio_path, 4, Utc::now()).await.unwrap();
        let id = result.id().to_string();
        queue
            .approve(&id, ApprovalDecision::default())
            .await
            .unwrap();
        queue.mark_processing(&id).await.unwrap();
        queue.mark_done(&id).await.unwrap();

        let error = queue.mark_failed(&id, "late failure").await.unwrap_err();

        assert!(matches!(
            error,
            VoiceQueueError::InvalidTransition {
                from: VoiceQueueStatus::Done,
                to: VoiceQueueStatus::Failed,
            }
        ));
    }

    #[tokio::test]
    async fn test_attempt_count_derives_from_processing_started_events() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "attempts.m4a", b"attempts").await;
        let id = "attempt12345";
        seed_enqueued(&queue, id, &audio_path, 8).await;
        queue
            .append_event(&queue_event(id, QueueEventType::Approved, None))
            .await
            .unwrap();
        queue
            .append_event(&queue_event(
                id,
                QueueEventType::ProcessingStarted,
                Some(serde_json::json!({
                    "pid": std::process::id(),
                    "proc_start_epoch": Utc::now().timestamp()
                })),
            ))
            .await
            .unwrap();
        queue
            .append_event(&queue_event(
                id,
                QueueEventType::ResetForRetry,
                Some(serde_json::json!({ "trigger": "auto" })),
            ))
            .await
            .unwrap();
        queue
            .append_event(&queue_event(
                id,
                QueueEventType::ProcessingStarted,
                Some(serde_json::json!({
                    "pid": std::process::id(),
                    "proc_start_epoch": Utc::now().timestamp()
                })),
            ))
            .await
            .unwrap();

        let item = queue.get(id).await.unwrap().unwrap();
        assert_eq!(item.status, VoiceQueueStatus::Processing);
        assert_eq!(item.attempt_count, 2);
        assert_eq!(item.auto_reset_count, 1);
    }

    #[tokio::test]
    async fn test_reaper_dead_pid_resets_for_retry() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "dead-pid.m4a", b"dead").await;
        let id = "deadpid12345";
        seed_processing_event(
            &queue,
            id,
            &audio_path,
            Some(999_999),
            Utc::now().timestamp(),
        )
        .await;

        let report = queue.reap_stale_processing().await.unwrap();
        let item = queue.get(id).await.unwrap().unwrap();

        assert_eq!(report.reset_for_retry, vec![id.to_string()]);
        assert_eq!(item.status, VoiceQueueStatus::Pending);
        assert_eq!(item.auto_reset_count, 1);
    }

    #[tokio::test]
    async fn test_reaper_alive_pid_untouched() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "alive-pid.m4a", b"alive").await;
        let id = "alivepid123";
        seed_processing_event(
            &queue,
            id,
            &audio_path,
            Some(std::process::id()),
            Utc::now().timestamp(),
        )
        .await;

        let report = queue.reap_stale_processing().await.unwrap();
        let item = queue.get(id).await.unwrap().unwrap();

        assert!(report.reset_for_retry.is_empty());
        assert!(report.quarantined.is_empty());
        assert_eq!(item.status, VoiceQueueStatus::Processing);
    }

    #[tokio::test]
    async fn test_reaper_pid_reuse_epoch_mismatch_resets() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "reuse-pid.m4a", b"reuse").await;
        let id = "reusepid123";
        seed_processing_event(&queue, id, &audio_path, Some(std::process::id()), 0).await;

        let report = queue.reap_stale_processing().await.unwrap();
        let item = queue.get(id).await.unwrap().unwrap();

        assert_eq!(report.reset_for_retry, vec![id.to_string()]);
        assert_eq!(item.status, VoiceQueueStatus::Pending);
    }

    #[tokio::test]
    async fn test_reaper_at_cap_fails_and_quarantines() {
        let (queue, temp) = create_test_queue().await;
        let audio_path = create_audio_file(&temp, "cap-pid.m4a", b"cap").await;
        let id = "cappid123456";
        seed_processing_event(
            &queue,
            id,
            &audio_path,
            Some(999_999),
            Utc::now().timestamp(),
        )
        .await;
        queue
            .append_event(&queue_event(
                id,
                QueueEventType::ResetForRetry,
                Some(serde_json::json!({ "trigger": "auto" })),
            ))
            .await
            .unwrap();
        queue
            .append_event(&queue_event(
                id,
                QueueEventType::ProcessingStarted,
                Some(serde_json::json!({
                    "pid": 999999u32,
                    "proc_start_epoch": Utc::now().timestamp()
                })),
            ))
            .await
            .unwrap();

        let report = queue.reap_stale_processing().await.unwrap();
        let item = queue.get(id).await.unwrap().unwrap();

        assert!(report.reset_for_retry.is_empty());
        assert_eq!(report.quarantined.len(), 1);
        assert_eq!(report.quarantined[0].id, id);
        assert_eq!(item.status, VoiceQueueStatus::Failed);
        assert_eq!(item.error.as_deref(), Some("reaped at retry cap"));
    }

    #[test]
    fn test_status_display_roundtrip() {
        assert_eq!(
            VoiceQueueStatus::AwaitingApproval.to_string(),
            "awaiting_approval"
        );
        assert_eq!(VoiceQueueStatus::Skipped.to_string(), "skipped");
    }
}

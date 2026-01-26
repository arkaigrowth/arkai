//! Voice Memos file watcher.
//!
//! Watches the Voice Memos directory for new .m4a files and emits events
//! when they are stable (iCloud sync complete).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{DateTime, Utc};
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

use super::queue::{compute_file_hash, EnqueueResult, VoiceQueue};

/// Errors that can occur with the watcher
#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("Watch directory does not exist: {0}")]
    DirectoryNotFound(PathBuf),

    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Queue error: {0}")]
    Queue(String),
}

/// Configuration for the watcher
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// Path to watch (Voice Memos directory)
    pub watch_path: PathBuf,

    /// How long a file must be stable before processing (seconds)
    pub stability_delay_secs: u64,

    /// File extensions to watch
    pub extensions: Vec<String>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            watch_path: Self::default_voice_memos_path(),
            stability_delay_secs: 5,
            extensions: vec!["m4a".to_string()],
        }
    }
}

impl WatcherConfig {
    /// Default Voice Memos path on macOS
    pub fn default_voice_memos_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join("Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings")
    }

    /// Check if the watch path exists
    pub fn validate(&self) -> Result<(), WatcherError> {
        if !self.watch_path.exists() {
            return Err(WatcherError::DirectoryNotFound(self.watch_path.clone()));
        }
        Ok(())
    }
}

/// Event emitted when an audio file is detected and stable
#[derive(Debug, Clone)]
pub struct AudioFileEvent {
    /// Path to the audio file
    pub path: PathBuf,

    /// SHA256 hash (12 chars)
    pub hash: String,

    /// File size in bytes
    pub size: u64,

    /// When the file was detected
    pub detected_at: DateTime<Utc>,
}

/// Voice Memo watcher with stability checking
pub struct VoiceMemoWatcher {
    config: WatcherConfig,
}

impl VoiceMemoWatcher {
    /// Create a new watcher with default configuration
    pub fn new() -> Self {
        Self {
            config: WatcherConfig::default(),
        }
    }

    /// Create a watcher with custom configuration
    pub fn with_config(config: WatcherConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    pub fn config(&self) -> &WatcherConfig {
        &self.config
    }

    /// Scan the directory once and enqueue any existing files
    /// Returns the number of new files queued
    pub async fn scan_once(&self, queue: &VoiceQueue) -> Result<ScanResult> {
        self.config.validate().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut result = ScanResult::default();

        let mut entries = tokio::fs::read_dir(&self.config.watch_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Check extension
            if !self.is_audio_file(&path) {
                continue;
            }

            // Get file metadata
            let metadata = match tokio::fs::metadata(&path).await {
                Ok(m) => m,
                Err(_) => continue,
            };

            if !metadata.is_file() {
                continue;
            }

            let file_size = metadata.len();

            // Enqueue the file
            match queue.enqueue(&path, file_size, Utc::now()).await {
                Ok(enqueue_result) => match enqueue_result {
                    EnqueueResult::Queued(_) => result.new_files += 1,
                    EnqueueResult::AlreadyQueued(_) => result.already_queued += 1,
                    EnqueueResult::AlreadyProcessed(_) => result.already_processed += 1,
                    EnqueueResult::ResetForRetry(_) => result.reset_for_retry += 1,
                },
                Err(e) => {
                    tracing::warn!("Failed to enqueue {}: {}", path.display(), e);
                    result.errors += 1;
                }
            }
        }

        Ok(result)
    }

    /// Watch the directory and emit events for new stable files
    /// This runs until cancelled via the returned channel
    pub async fn watch(
        &self,
        queue: Arc<VoiceQueue>,
    ) -> Result<(mpsc::Receiver<AudioFileEvent>, WatchHandle)> {
        self.config.validate().map_err(|e| anyhow::anyhow!("{}", e))?;

        let (event_tx, event_rx) = mpsc::channel::<AudioFileEvent>(100);
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        let config = self.config.clone();

        // Spawn watcher task
        let handle = tokio::spawn(async move {
            if let Err(e) = run_watcher(config, queue, event_tx, &mut stop_rx).await {
                tracing::error!("Watcher error: {}", e);
            }
        });

        Ok((
            event_rx,
            WatchHandle {
                stop_tx,
                task: handle,
            },
        ))
    }

    /// Check if a path is an audio file we care about
    fn is_audio_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| self.config.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
            .unwrap_or(false)
    }
}

impl Default for VoiceMemoWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to control the watcher
pub struct WatchHandle {
    stop_tx: mpsc::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

impl WatchHandle {
    /// Stop the watcher
    pub async fn stop(self) -> Result<()> {
        let _ = self.stop_tx.send(()).await;
        self.task.await?;
        Ok(())
    }
}

/// Result of a directory scan
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub new_files: usize,
    pub already_queued: usize,
    pub already_processed: usize,
    pub reset_for_retry: usize,
    pub errors: usize,
}

impl ScanResult {
    pub fn total_scanned(&self) -> usize {
        self.new_files + self.already_queued + self.already_processed + self.reset_for_retry
    }
}

/// Internal watcher loop
async fn run_watcher(
    config: WatcherConfig,
    queue: Arc<VoiceQueue>,
    event_tx: mpsc::Sender<AudioFileEvent>,
    stop_rx: &mut mpsc::Receiver<()>,
) -> Result<()> {
    // Track files being stabilized (path -> (size, last_seen))
    let mut pending: HashMap<PathBuf, (u64, Instant)> = HashMap::new();

    // Create debounced watcher
    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(
        Duration::from_secs(2), // Initial debounce
        tx,
    )?;

    debouncer.watcher().watch(&config.watch_path, RecursiveMode::NonRecursive)?;

    let stability_delay = Duration::from_secs(config.stability_delay_secs);

    tracing::info!("Watching {} for audio files", config.watch_path.display());

    loop {
        // Check for stop signal
        if stop_rx.try_recv().is_ok() {
            tracing::info!("Watcher stopping...");
            break;
        }

        // Check for file events (non-blocking with timeout)
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(events)) => {
                for event in events {
                    let path = event.path;

                    // Only care about audio files
                    if !path.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| config.extensions.iter().any(|ext| ext.eq_ignore_ascii_case(e)))
                        .unwrap_or(false)
                    {
                        continue;
                    }

                    // Get current file size
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        if metadata.is_file() {
                            let size = metadata.len();
                            pending.insert(path, (size, Instant::now()));
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("Watcher error: {:?}", e);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Expected - continue to stability check
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                tracing::error!("Watcher channel disconnected");
                break;
            }
        }

        // Check for stable files
        let now = Instant::now();
        let mut stable_files = Vec::new();

        for (path, (last_size, last_seen)) in pending.iter() {
            if now.duration_since(*last_seen) >= stability_delay {
                // Check current size
                if let Ok(metadata) = std::fs::metadata(path) {
                    let current_size = metadata.len();
                    if current_size == *last_size && current_size > 0 {
                        stable_files.push((path.clone(), current_size));
                    } else {
                        // Size changed, update tracking
                        // Note: We can't modify during iteration, handle after
                    }
                }
            }
        }

        // Process stable files
        for (path, size) in stable_files {
            pending.remove(&path);

            // Compute hash and create event
            match compute_file_hash(&path).await {
                Ok(hash) => {
                    let audio_event = AudioFileEvent {
                        path: path.clone(),
                        hash: hash.clone(),
                        size,
                        detected_at: Utc::now(),
                    };

                    // Enqueue to queue
                    match queue.enqueue(&path, size, Utc::now()).await {
                        Ok(result) => {
                            if result.is_new() {
                                tracing::info!("New audio file queued: {} ({})", path.display(), hash);
                                let _ = event_tx.send(audio_event).await;
                            } else {
                                tracing::debug!("Audio file already in queue: {}", path.display());
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to enqueue {}: {}", path.display(), e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to hash {}: {}", path.display(), e);
                }
            }
        }

        // Small sleep to prevent busy loop
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_voice_memos_path() {
        let path = WatcherConfig::default_voice_memos_path();
        assert!(path.to_string_lossy().contains("VoiceMemos"));
    }

    #[test]
    fn test_config_extensions() {
        let config = WatcherConfig::default();
        assert!(config.extensions.contains(&"m4a".to_string()));
    }

    #[tokio::test]
    async fn test_scan_once() {
        let temp = TempDir::new().unwrap();

        // Create test files
        let audio1 = temp.path().join("test1.m4a");
        let audio2 = temp.path().join("test2.m4a");
        let other = temp.path().join("test.txt");

        tokio::fs::write(&audio1, b"audio 1").await.unwrap();
        tokio::fs::write(&audio2, b"audio 2").await.unwrap();
        tokio::fs::write(&other, b"not audio").await.unwrap();

        // Create watcher and queue
        let config = WatcherConfig {
            watch_path: temp.path().to_path_buf(),
            stability_delay_secs: 1,
            extensions: vec!["m4a".to_string()],
        };
        let watcher = VoiceMemoWatcher::with_config(config);

        let queue_path = temp.path().join("queue.jsonl");
        let queue = VoiceQueue::new(queue_path);

        // Scan
        let result = watcher.scan_once(&queue).await.unwrap();

        assert_eq!(result.new_files, 2);
        assert_eq!(result.already_queued, 0);

        // Scan again - should be idempotent
        let result2 = watcher.scan_once(&queue).await.unwrap();

        assert_eq!(result2.new_files, 0);
        assert_eq!(result2.already_queued, 2);
    }
}

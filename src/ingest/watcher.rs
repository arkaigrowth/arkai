//! Voice Memos file watcher.
//!
//! Watches the Voice Memos directory for new .m4a files and emits events
//! when they are stable (iCloud sync complete).
//!
//! ## Stability Hardening (Phase 1.5)
//!
//! Files must pass multiple checks before processing:
//! - Size + mtime unchanged for stability_delay (10s default)
//! - Minimum age of 30 seconds since first seen
//! - At least 2 consecutive stable checks
//! - ffprobe validation for .qta files (pre-normalize)
//!
//! If normalization or validation fails, files are deferred (not errored)
//! and will be retried on the next stability window.

use std::collections::HashMap;

/// Minimum age before processing (hardening against iCloud sync)
/// Files modified in the last 30 seconds are considered potentially unstable.
const MIN_FILE_AGE_SECS: u64 = 30;
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

use super::queue::{compute_file_hash, normalize_audio, EnqueueResult, VoiceQueue};

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
            stability_delay_secs: 10, // Bumped from 5 for iPhone sync stability
            extensions: vec!["m4a".to_string(), "qta".to_string()], // Added .qta for iPhone sync
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

            // Check file age - skip files modified in last 30 seconds (likely still syncing)
            if let Ok(mtime) = metadata.modified() {
                if let Ok(age) = mtime.elapsed() {
                    if age < std::time::Duration::from_secs(MIN_FILE_AGE_SECS) {
                        tracing::debug!("Skipped (too recent, age={:.1}s): {}", age.as_secs_f32(), path.display());
                        result.deferred += 1;
                        continue;
                    }
                }
            }

            // Pre-validate with ffprobe for .qta files
            if is_qta_file(&path) {
                if !validate_audio_readable(&path).await {
                    tracing::info!("Deferred (ffprobe failed): {}", path.display());
                    result.deferred += 1;
                    continue;
                }
            }

            // Normalize .qta → .m4a if needed (before hashing/enqueueing)
            let normalized_path = match normalize_audio(&path).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::info!("Deferred (normalize failed): {} - {}", path.display(), e);
                    result.deferred += 1;
                    continue;
                }
            };

            // Get normalized file size (may differ after conversion)
            let normalized_size = match tokio::fs::metadata(&normalized_path).await {
                Ok(m) => m.len(),
                Err(_) => file_size, // Fallback to original size
            };

            // Enqueue the normalized file
            match queue.enqueue(&normalized_path, normalized_size, Utc::now()).await {
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
    pub deferred: usize,
    pub errors: usize,
}

impl ScanResult {
    pub fn total_scanned(&self) -> usize {
        self.new_files + self.already_queued + self.already_processed + self.reset_for_retry
    }
}

/// Stability tracking for a pending file
/// Implements Chad's hardening requirements:
/// - Size + mtime unchanged for stability_delay
/// - Minimum age since first seen
/// - Multiple consecutive stable checks
#[derive(Debug, Clone)]
struct FileStabilityState {
    /// File size at last check
    size: u64,
    /// File mtime at last check (as SystemTime)
    mtime: std::time::SystemTime,
    /// When we first saw this file
    first_seen: Instant,
    /// When size/mtime last changed
    last_changed: Instant,
    /// Number of consecutive stable checks passed
    stable_checks: u32,
}

impl FileStabilityState {
    fn new(size: u64, mtime: std::time::SystemTime) -> Self {
        let now = Instant::now();
        Self {
            size,
            mtime,
            first_seen: now,
            last_changed: now,
            stable_checks: 0,
        }
    }

    /// Update state with new file metadata
    /// Returns true if file changed (resets stability)
    fn update(&mut self, new_size: u64, new_mtime: std::time::SystemTime) -> bool {
        if new_size != self.size || new_mtime != self.mtime {
            self.size = new_size;
            self.mtime = new_mtime;
            self.last_changed = Instant::now();
            self.stable_checks = 0;
            true
        } else {
            false
        }
    }

    /// Check if file is stable enough for processing
    /// Requirements:
    /// - No changes for stability_delay
    /// - Minimum age of 30 seconds since first seen
    /// - At least 2 consecutive stable checks
    fn is_stable(&self, stability_delay: Duration, min_age: Duration) -> bool {
        let now = Instant::now();
        let time_since_change = now.duration_since(self.last_changed);
        let age = now.duration_since(self.first_seen);

        time_since_change >= stability_delay
            && age >= min_age
            && self.stable_checks >= 2
    }

    /// Record a stable check (size/mtime unchanged)
    fn record_stable_check(&mut self) {
        self.stable_checks += 1;
    }

    /// Reset for retry after a deferral
    fn reset_for_retry(&mut self) {
        self.last_changed = Instant::now();
        self.stable_checks = 0;
    }
}

/// Internal watcher loop
async fn run_watcher(
    config: WatcherConfig,
    queue: Arc<VoiceQueue>,
    event_tx: mpsc::Sender<AudioFileEvent>,
    stop_rx: &mut mpsc::Receiver<()>,
) -> Result<()> {
    // Track files being stabilized with enhanced state
    let mut pending: HashMap<PathBuf, FileStabilityState> = HashMap::new();

    // Create debounced watcher
    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(
        Duration::from_secs(2), // Initial debounce
        tx,
    )?;

    debouncer.watcher().watch(&config.watch_path, RecursiveMode::NonRecursive)?;

    let stability_delay = Duration::from_secs(config.stability_delay_secs);
    let min_age = Duration::from_secs(MIN_FILE_AGE_SECS);

    tracing::info!(
        "Watching {} for audio files (stability: {}s, min_age: {}s)",
        config.watch_path.display(),
        config.stability_delay_secs,
        MIN_FILE_AGE_SECS
    );

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

                    // Get current file metadata (size + mtime)
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        if metadata.is_file() {
                            let size = metadata.len();
                            let mtime = metadata.modified().unwrap_or(std::time::SystemTime::now());

                            // Update or create tracking state
                            if let Some(state) = pending.get_mut(&path) {
                                state.update(size, mtime);
                            } else {
                                pending.insert(path, FileStabilityState::new(size, mtime));
                            }
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

        // Check for stable files (two-phase: first update states, then collect stable ones)
        let mut stable_files = Vec::new();

        for (path, state) in pending.iter_mut() {
            // Get current metadata
            if let Ok(metadata) = std::fs::metadata(path) {
                let current_size = metadata.len();
                let current_mtime = metadata.modified().unwrap_or(std::time::SystemTime::now());

                // Check if file changed
                if !state.update(current_size, current_mtime) {
                    // File unchanged - record a stable check
                    state.record_stable_check();
                }

                // Check if fully stable (delay + min_age + 2 stable checks)
                if current_size > 0 && state.is_stable(stability_delay, min_age) {
                    stable_files.push((path.clone(), current_size));
                }
            }
        }

        // Process stable files
        for (path, size) in stable_files {
            // Pre-normalize validation: verify file is readable with ffprobe
            // If this fails, the file is likely still syncing despite passing stability checks
            if is_qta_file(&path) {
                if !validate_audio_readable(&path).await {
                    tracing::info!("Deferred (ffprobe failed, still syncing?): {}", path.display());
                    // Reset for retry - don't remove from pending
                    if let Some(state) = pending.get_mut(&path) {
                        state.reset_for_retry();
                    }
                    continue;
                }
            }

            // Normalize .qta → .m4a if needed (before hashing/enqueueing)
            let normalized_path = match normalize_audio(&path).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::info!("Deferred (normalize failed): {} - {}", path.display(), e);
                    // Reset for retry - don't remove from pending
                    if let Some(state) = pending.get_mut(&path) {
                        state.reset_for_retry();
                    }
                    continue;
                }
            };

            // Successfully normalized - NOW remove from pending
            pending.remove(&path);

            // Get normalized file size
            let normalized_size = match tokio::fs::metadata(&normalized_path).await {
                Ok(m) => m.len(),
                Err(_) => size, // Fallback to original size
            };

            // Compute hash and create event
            match compute_file_hash(&normalized_path).await {
                Ok(hash) => {
                    let audio_event = AudioFileEvent {
                        path: normalized_path.clone(),
                        hash: hash.clone(),
                        size: normalized_size,
                        detected_at: Utc::now(),
                    };

                    // Enqueue the normalized file
                    match queue.enqueue(&normalized_path, normalized_size, Utc::now()).await {
                        Ok(result) => {
                            if result.is_new() {
                                tracing::info!("New audio file queued: {} ({})", normalized_path.display(), hash);
                                let _ = event_tx.send(audio_event).await;
                            } else {
                                tracing::debug!("Audio file already in queue: {}", normalized_path.display());
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to enqueue {}: {}", normalized_path.display(), e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to hash {}: {}", normalized_path.display(), e);
                }
            }
        }

        // Small sleep to prevent busy loop
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

/// Check if a path is a .qta file
fn is_qta_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("qta"))
        .unwrap_or(false)
}

/// Validate that an audio file is readable using ffprobe
/// Returns false if ffprobe fails (file likely still syncing)
async fn validate_audio_readable(path: &Path) -> bool {
    let output = tokio::process::Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .await;

    match output {
        Ok(out) => {
            // ffprobe succeeded - check if we got a valid duration
            let duration_str = String::from_utf8_lossy(&out.stdout);
            duration_str.trim().parse::<f32>().is_ok()
        }
        Err(_) => false,
    }
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

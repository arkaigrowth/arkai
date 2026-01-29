# Phase 1 + 1.5 Code Review for Chad

**Purpose**: Quick review of hardening implementation before proceeding to parallel build tracks.

---

## 1. FileStabilityState (watcher.rs:290-360)

```rust
/// Stability tracking for a pending file
/// Implements Chad's hardening requirements:
/// - Size + mtime unchanged for stability_delay
/// - Minimum age since first seen
/// - Multiple consecutive stable checks
#[derive(Debug, Clone)]
struct FileStabilityState {
    size: u64,
    mtime: std::time::SystemTime,
    first_seen: Instant,
    last_changed: Instant,
    stable_checks: u32,
}

impl FileStabilityState {
    fn new(size: u64, mtime: std::time::SystemTime) -> Self {
        let now = Instant::now();
        Self { size, mtime, first_seen: now, last_changed: now, stable_checks: 0 }
    }

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
    /// - No changes for stability_delay (10s)
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

    fn record_stable_check(&mut self) { self.stable_checks += 1; }

    fn reset_for_retry(&mut self) {
        self.last_changed = Instant::now();
        self.stable_checks = 0;
    }
}
```

**Questions for Chad:**
- Is `stable_checks >= 2` sufficient, or should it be 3?
- Should we track mtime resolution issues (some filesystems have 1s granularity)?

---

## 2. Pre-Normalize Validation (watcher.rs:547-564)

```rust
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
```

**Note**: This runs BEFORE normalize_audio() for .qta files only.

---

## 3. Deferral Logic (watcher.rs:464-493)

```rust
// Process stable files
for (path, size) in stable_files {
    // Pre-normalize validation: verify file is readable with ffprobe
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

    // Normalize .qta → .m4a if needed
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

    // ... continue with hash + enqueue
}
```

**Key behavior**: On failure, file stays in `pending` map with reset state → retried on next stability window.

---

## 4. Streaming Hash (queue.rs:455-477)

```rust
/// Compute SHA256 hash of file content using streaming (8KB chunks)
pub async fn compute_file_hash(path: &Path) -> Result<String, std::io::Error> {
    use tokio::io::AsyncReadExt;

    let file = tokio::fs::File::open(path).await?;
    let mut reader = tokio::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192]; // 8KB chunks

    loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result)[..12].to_string())
}
```

**Fixed**: No longer loads entire file into memory.

---

## 5. Constants (watcher.rs:21-23)

```rust
/// Minimum age before processing (hardening against iCloud sync)
/// Files modified in the last 30 seconds are considered potentially unstable.
const MIN_FILE_AGE_SECS: u64 = 30;
```

Stability delay is 10s (in WatcherConfig), min age is 30s.

---

## Edge Cases Addressed

| Concern | Solution |
|---------|----------|
| Hash of incomplete .qta | ffprobe validation + stability checks BEFORE hash |
| Normalize fails mid-sync | Deferral with reset_for_retry(), stays in pending |
| 10s too short for big syncs | Added 30s min age + 2 stable checks |
| Double-enqueue race | Hash only happens AFTER normalize succeeds |
| Memory OOM on large files | Streaming 8KB hash |

---

## Potential Concerns (for Chad)

1. **scan_once age check**: Currently skips files < 30s old but doesn't track them for retry. Watcher handles this, but scan_once is fire-and-forget.

2. **ffprobe process spawning**: Each validation spawns a process. For large directories with many .qta files, could be slow. Consider: batch validation or rate limiting?

3. **Cache growth**: `~/.arkai/voice_cache/` grows unbounded. Added cleanup policy note but no implementation.

---

**Verdict needed**: Is this sufficient hardening to proceed with Phase 2+4 parallel tracks?

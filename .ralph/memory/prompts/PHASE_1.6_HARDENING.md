---
created: 2026-01-29
purpose: Chad's Phase 1.6 hardening requirements (do BEFORE parallel tracks)
status: DONE
completed: 2026-01-29
---

# Phase 1.6 Hardening (Chad's Feedback)

**Do this BEFORE starting Phase 2+3 or Phase 4.**

## Requirements

### 1. scan_once should report/defer young files, not silently skip

**Current behavior:** Files < 30s old are skipped with `continue` and counted as `deferred`.

**Fix needed:** Log which files were deferred and why, so user knows to retry.

```rust
// In scan_once, instead of just incrementing deferred:
tracing::info!("Deferred (too recent, age={:.1}s): {}", age.as_secs_f32(), path.display());
result.deferred += 1;
```

### 2. stable_checks needs either >=3 OR minimum time between checks

**Current behavior:** `stable_checks >= 2` with no time constraint between checks.

**Problem:** Two checks could happen 100ms apart.

**Fix options:**
- A) Require `stable_checks >= 3`
- B) Track `last_stable_check` timestamp, require 2s between checks
- C) Both

**Recommended:** Option C - require 3 checks with 2s minimum between each.

```rust
struct FileStabilityState {
    // ... existing fields
    last_stable_check: Option<Instant>,  // NEW
}

fn record_stable_check(&mut self) -> bool {
    let now = Instant::now();
    if let Some(last) = self.last_stable_check {
        if now.duration_since(last) < Duration::from_secs(2) {
            return false;  // Too soon, don't count
        }
    }
    self.last_stable_check = Some(now);
    self.stable_checks += 1;
    true
}

fn is_stable(...) -> bool {
    // ... existing checks
    && self.stable_checks >= 3  // Changed from 2
}
```

### 3. ffprobe missing should hard-error, not defer forever

**Current behavior:** If ffprobe binary is missing, validation fails → defer → retry → defer → infinite loop.

**Fix needed:** Check if ffprobe exists at startup. Hard-error if missing.

```rust
// At watcher startup or first use:
fn check_ffprobe_available() -> Result<()> {
    match std::process::Command::new("ffprobe").arg("-version").output() {
        Ok(out) if out.status.success() => Ok(()),
        _ => anyhow::bail!("ffprobe not found. Install ffmpeg to process .qta files.")
    }
}
```

## Acceptance Criteria

- [x] scan_once logs each deferred file with reason
- [x] stable_checks >= 3 with 2s minimum between checks
- [x] ffprobe missing causes startup error, not infinite defer loop

## Test Results (2026-01-29)

### Test 1: Deferred file logging
```bash
$ echo "test" > /tmp/arkai-test-voice/fresh-test.m4a
$ RUST_LOG=info arkai voice scan --path /tmp/arkai-test-voice
📂 Scanning: /tmp/arkai-test-voice
INFO Deferred (too recent, age=0.3s): /tmp/arkai-test-voice/fresh-test.m4a
  Deferred (syncing):  1
```
✅ **PASS**: Deferred files now logged with reason

### Test 2: ffprobe missing error
```bash
$ PATH=/bin:/usr/bin arkai voice scan --path /tmp/arkai-test-voice
Error: ffprobe not found: No such file or directory (os error 2). Install ffmpeg to process .qta files: brew install ffmpeg
```
✅ **PASS**: Hard startup error instead of infinite defer loop

### Test 3: Unit tests
```bash
$ cargo test watcher
running 4 tests
test ingest::watcher::tests::test_default_voice_memos_path ... ok
test ingest::watcher::tests::test_config_extensions ... ok
test ingest::watcher::tests::test_scan_once_defers_fresh_files ... ok
test ingest::watcher::tests::test_scan_once_processes_old_files ... ok
```
✅ **PASS**: All watcher tests pass

---

**Phase 1.6 COMPLETE. Ready for parallel tracks (Phase 2+3 and Phase 4).**

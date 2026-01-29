---
created: 2026-01-29
purpose: Chad's Phase 1.6 hardening requirements (do BEFORE parallel tracks)
status: PENDING
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

- [ ] scan_once logs each deferred file with reason
- [ ] stable_checks >= 3 with 2s minimum between checks
- [ ] ffprobe missing causes startup error, not infinite defer loop

## Test

```bash
# Test 1: Create a "young" file, run scan, verify it's reported as deferred
touch /tmp/test.qta
arkai voice scan --path /tmp  # Should show "Deferred (too recent)"

# Test 2: Remove ffprobe temporarily
sudo mv /opt/homebrew/bin/ffprobe /opt/homebrew/bin/ffprobe.bak
arkai voice watch  # Should error: "ffprobe not found"
sudo mv /opt/homebrew/bin/ffprobe.bak /opt/homebrew/bin/ffprobe
```

---

**After completing Phase 1.6, proceed with parallel tracks.**

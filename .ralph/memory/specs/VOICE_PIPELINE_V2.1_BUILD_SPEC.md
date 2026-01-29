---
created: 2026-01-28T09:30:00Z
status: FROZEN
authors: [claude-opus, chad, alex]
purpose: Canonical build spec for Voice Pipeline v2.1
---

# 🔒 VOICE PIPELINE v2.1 — CANONICAL BUILD SPEC

> **Status: FROZEN**
> If anything conflicts with this spec, this spec wins.
> Ask questions only if something is impossible, not just unclear.

---

## Purpose

Zero-friction voice memo capture, transcription, and optional diarization with strong security boundaries and graceful degradation.

---

## 1. System Roles & Ownership

### Mac (Execution + Library)

| Owns | Writes | NEVER Touches |
|------|--------|---------------|
| `~/.arkai/` (engine state, voice_queue.jsonl) | audio → transcript | `~/clawd/artifacts/` |
| `~/.arkai/voice_cache/` (normalized audio cache) | | VPS state |
| `~/AI/library/voice/` (final transcripts) | | |

**Watch path (canonical):**
```
/Users/alexkamysz/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings
```

**Supported formats:**
- `.m4a` - Standard Voice Memo format
- `.qta` - iPhone sync intermediate format (MUST normalize before processing)
- `.ogg` - Telegram inbound (VPS only)

**Runs:**
- Voice watcher (existing, needs format update)
- Audio normalizer (NEW - ffmpeg .qta → .m4a)
- Local Whisper (existing)
- Optional diarization (pyannote, new)

### VPS (Orchestration + UX)

| Owns | Writes | NEVER Touches |
|------|--------|---------------|
| `~/clawd/artifacts/voice/requests/` | request/result JSONs | `~/.arkai/` |
| `~/clawd/artifacts/voice/results/` | | Mac filesystem |
| `~/clawd/artifacts/voice/audio-cache/` | | |

**Runs:**
- vps-voice-runner daemon (new)
- Whisper API transcription (Groq/OpenAI - NOT local Whisper)
- Speaker detection (new)

> ⚠️ **VPS Hardware Limitation**: CAX21 has 4 ARM64 cores, 7.5GB RAM, NO GPU.
> Local Whisper would be too slow and could lag Claudia. Use API instead.

### Clawdbot/Moltbot

**Used for:**
- Telegram intake
- Node execution (`clawdbot node run`)
- Approval flows (`exec-approvals.json`)

**NOT used as:**
- Artifact store
- Pipeline state store

---

## 2. Default Behavior

- All voice memos auto-transcribe (zero friction)
- Safety caps are **mandatory**:
  - `--limit N` (max items per batch)
  - `--max-hours H` (max audio duration per batch)
- Diarization default: `auto` (only when multi-speaker detected)

---

## 3. Existing Components (DO NOT REBUILD)

| Component | Location | Status |
|-----------|----------|--------|
| VoiceQueue (event-sourced) | `src/ingest/queue.rs` | ✅ Working |
| Local Whisper transcriber | `src/ingest/transcriber.rs` | ✅ Working |
| ClawdbotClient webhook | `src/adapters/clawdbot.rs` | ✅ Working |
| Voice watcher | `src/ingest/watcher.rs` | ✅ Working |
| Telegram inbound audio | `~/.clawdbot/media/inbound/*.ogg` | ✅ Available |
| Clawdbot approvals | `~/.clawdbot/exec-approvals.json` | ✅ Available |

---

## 4. What To Build (In Order)

### Phase 1 — CLI Caps + Watcher Fixes (HIGH PRIORITY)

**Files to modify:**
- `src/cli/voice.rs` - Add CLI flags
- `src/ingest/watcher.rs` - Add .qta support, bump stability delay
- `src/ingest/queue.rs` - Streaming hash, duration capture

#### 1a. CLI Flags (`src/cli/voice.rs`)

```rust
Process {
    #[arg(long)]
    once: bool,
    #[arg(long, default_value = "telegram")]
    route: String,
    #[arg(long, default_value = "base")]
    model: String,
    // NEW FLAGS:
    #[arg(long)]
    limit: Option<u32>,          // Stop after N items
    #[arg(long)]
    max_hours: Option<f32>,      // Stop after H hours of audio (uses sum of duration_seconds)
    #[arg(long)]
    dry_run: bool,               // Show what would process
}
```

#### 1b. Watcher Fixes (`src/ingest/watcher.rs`)

```rust
// Update WatcherConfig defaults:
WatcherConfig {
    watch_path: "/Users/alexkamysz/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings".into(),
    stability_delay_secs: 10,  // BUMP from 5 to 10 for iPhone sync
    extensions: vec!["m4a".to_string(), "qta".to_string()],  // ADD .qta
}
```

> ⚠️ **Why .qta matters**: iPhone recordings sync as .qta initially (temporary container).
> If enqueued too early, hash will be garbage. Stability delay + normalization handles this.

#### 1c. Streaming Hash (`src/ingest/queue.rs`) — CRITICAL FIX

**Current (BAD - loads entire file into memory):**
```rust
let content = tokio::fs::read(path).await?;  // DON'T DO THIS
```

**Fixed (streaming, 8KB chunks):**
```rust
pub async fn compute_file_hash(path: &Path) -> Result<String, std::io::Error> {
    use tokio::io::AsyncReadExt;
    let file = tokio::fs::File::open(path).await?;
    let mut reader = tokio::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result)[..12].to_string())
}
```

#### 1d. Duration Capture at Enqueue

**Add to QueueItemData struct:**
```rust
pub duration_seconds: Option<f32>,  // NEW - populated via ffprobe
```

**Duration probe helper:**
```rust
async fn probe_duration(path: &Path) -> Option<f32> {
    let output = tokio::process::Command::new("ffprobe")
        .args(["-v", "quiet", "-show_entries", "format=duration",
               "-of", "default=noprint_wrappers=1:nokey=1"])
        .arg(path)
        .output()
        .await
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}
```

#### 1e. Audio Normalization (Mac-side only)

**For .qta files, normalize BEFORE hashing:**
```rust
async fn normalize_audio(input: &Path, cache_dir: &Path) -> Result<PathBuf> {
    if input.extension().map(|e| e != "qta").unwrap_or(true) {
        return Ok(input.to_path_buf());  // Already .m4a, no conversion needed
    }
    let hash = compute_file_hash(input).await?;
    let output = cache_dir.join(format!("{}.m4a", hash));
    if output.exists() { return Ok(output); }  // Already cached

    tokio::process::Command::new("ffmpeg")
        .args(["-i", input.to_str().unwrap(),
               "-c:a", "aac", "-b:a", "128k", "-y",
               output.to_str().unwrap()])
        .output()
        .await?;
    Ok(output)
}
```

> ⚠️ **Security**: ffmpeg args are HARDCODED. No user input in command construction.
> Cache dir: `~/.arkai/voice_cache/`

**Acceptance Criteria:**
- [ ] `arkai voice process --limit 5` stops after 5 items
- [ ] `arkai voice process --max-hours 1.0` stops after 1 hour (sum of `duration_seconds`)
- [ ] `arkai voice process --dry-run` prints: `{id, file_name, ext, duration_seconds, size}` + totals
- [ ] `.qta` files detected and normalized before hashing
- [ ] Hash computed via streaming (NOT full file load)
- [ ] `duration_seconds` captured at enqueue time via ffprobe
- [ ] Stability delay is 10 seconds (not 5)

---

### Phase 2 — Path Authority Module

**Location:** `src/config/paths.rs` (Rust) AND `services/voice/paths.py` (Python)

**Purpose:** Single source of truth for all canonical paths.

```rust
// src/config/paths.rs
pub mod paths {
    // Mac paths
    pub const ARKAI_HOME: &str = "~/.arkai";
    pub const VOICE_QUEUE: &str = "~/.arkai/voice_queue.jsonl";
    pub const LIBRARY_VOICE: &str = "~/AI/library/voice";

    // VPS paths
    pub const VPS_ARTIFACTS: &str = "~/clawd/artifacts/voice";
    pub const VPS_REQUESTS: &str = "~/clawd/artifacts/voice/requests";
    pub const VPS_RESULTS: &str = "~/clawd/artifacts/voice/results";
    pub const VPS_AUDIO_CACHE: &str = "~/clawd/artifacts/voice/audio-cache";

    // Clawdbot paths (read-only for pipeline)
    pub const TELEGRAM_INBOUND: &str = "~/.clawdbot/media/inbound";
}
```

**Acceptance:**
- [ ] All new code imports paths from this module
- [ ] No hardcoded paths anywhere else

---

### Phase 3 — Schemas + Validators

**Location:** `contracts/`

**Create:**

#### `contracts/voice_request.schema.json`
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["id", "action", "params", "requested_by", "requested_at"],
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "action": { "type": "string", "enum": ["process", "status", "cancel"] },
    "params": {
      "type": "object",
      "properties": {
        "limit": { "type": "integer", "minimum": 1, "maximum": 50 },
        "max_hours": { "type": "number", "minimum": 0.1, "maximum": 3.0 },
        "diarize": { "type": "string", "enum": ["auto", "always", "never"], "default": "auto" },
        "model": { "type": "string", "enum": ["base", "small", "medium"], "default": "base" }
      }
    },
    "requested_by": { "type": "string", "enum": ["claudia", "alex", "system"] },
    "requested_at": { "type": "string", "format": "date-time" }
  }
}
```

#### `contracts/voice_result.schema.json`
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["id", "status", "completed_at"],
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "status": { "type": "string", "enum": ["completed", "partial", "failed"] },
    "processed_count": { "type": "integer" },
    "total_duration_seconds": { "type": "number" },
    "baseline": {
      "type": "object",
      "properties": {
        "transcript_refs": { "type": "array", "items": { "type": "string" } }
      }
    },
    "speaker_detection": {
      "type": "object",
      "properties": {
        "likely_multi_speaker": { "type": "boolean" },
        "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
        "method": { "type": "string", "enum": ["tier2_vad_mfcc"] }
      }
    },
    "diarization": {
      "type": "object",
      "properties": {
        "status": { "type": "string", "enum": ["not_needed", "queued", "completed", "failed"] },
        "transcript_ref": { "type": "string" }
      }
    },
    "completed_at": { "type": "string", "format": "date-time" },
    "error": { "type": "string" }
  }
}
```

**Acceptance:**
- [ ] Schemas created
- [ ] Python validator in `services/voice/validator.py`
- [ ] All requests/results validated before write

---

### Phase 4 — VPS Voice Runner (Daemon)

**Location:** `~/clawd/services/voice/vps_voice_runner.py` (on VPS)

**Pattern:** Similar to existing `tts-watcher.py`

**Setup prerequisites:**
```bash
# On VPS (one-time)
pip3 install openai groq jsonschema watchdog
mkdir -p ~/clawd/artifacts/voice/{requests,results,audio-cache}

# Set API keys (choose one or both)
export GROQ_API_KEY="..."      # Free tier available!
export OPENAI_API_KEY="..."    # $0.006/min
```

> ⚠️ **Use Whisper API, NOT local Whisper** - VPS is ARM64 with no GPU.
> Local inference would be 30-60s per minute of audio and could lag Claudia.

**Components:**
1. Watch `~/clawd/artifacts/voice/requests/`
2. Validate request against schema
3. Fetch audio from `~/.clawdbot/media/inbound/` (TelegramInboundFetcher)
4. **Transcribe via Whisper API** (Groq preferred - free & fast, OpenAI fallback)
5. Run speaker detection (Phase 5)
6. Write result to `~/clawd/artifacts/voice/results/{id}.json`
7. Append to audit log

**Transcription priority:**
1. **Groq** (free tier, ~10x faster than OpenAI)
2. **OpenAI** (fallback, $0.006/min)
3. **Mac local** (if VPS fails, dispatch to Mac node)

**Audit log format** (`~/clawd/artifacts/voice/audit.jsonl`):
```json
{"ts": "2026-01-28T10:00:00Z", "event": "request_received", "id": "uuid", "requested_by": "claudia"}
{"ts": "2026-01-28T10:00:05Z", "event": "transcription_complete", "id": "uuid", "duration_s": 45}
{"ts": "2026-01-28T10:00:06Z", "event": "result_written", "id": "uuid", "status": "completed"}
```

**Systemd service:** `~/.config/systemd/user/vps-voice-runner.service`

**Acceptance:**
- [ ] Daemon starts on VPS boot
- [ ] Processes requests within 60s
- [ ] Writes valid result JSON
- [ ] Appends audit log

---

### Phase 5 — Speaker Detection (Tier 2 Only)

**Location:** `services/voice/speaker_detector.py`

**Implementation (keep simple):**
```python
def detect_speakers(audio_path: Path) -> dict:
    """
    Tier 2: VAD segmentation + MFCC variance
    Returns: {"likely_multi_speaker": bool, "confidence": float}
    """
    # 1. Run VAD to get speech segments
    # 2. Extract MFCC features per segment
    # 3. Compute variance across segments
    # 4. If variance > threshold → multi-speaker likely
    return {
        "likely_multi_speaker": variance > THRESHOLD,
        "confidence": min(1.0, variance / MAX_VARIANCE),
        "method": "tier2_vad_mfcc"
    }
```

**Dependencies:** `pip3 install webrtcvad librosa`

**Acceptance:**
- [ ] Returns `likely_multi_speaker` boolean
- [ ] Confidence between 0-1
- [ ] Runs in <10s for 5-minute audio

---

### Phase 6 — Mac Diarizer (Premium Path)

**Trigger:** Via `clawdbot node run` (NOT custom RPC)

**Setup on Mac:**
```bash
# One-time setup
pip3 install pyannote.audio torch

# Start Mac as Clawdbot node
clawdbot node run --host arkai-clawdbot.taila30487.ts.net --display-name "Mac-Diarizer"
```

**Exec policy** (`~/.clawdbot/exec-approvals.json`):
```json
{
  "version": 1,
  "defaults": {
    "security": "deny",
    "ask": "always"
  },
  "agents": {
    "main": {
      "security": "allowlist",
      "ask": "always",
      "allowlist": [
        { "pattern": "*/python*" },
        { "pattern": "*/services/voice/diarize.py" }
      ]
    }
  }
}
```

**If Mac offline:**
- Queue diarization request
- Deliver baseline immediately
- Process queue when Mac comes online

**Acceptance:**
- [ ] VPS can dispatch to Mac via `clawdbot nodes run`
- [ ] Every exec requires approval
- [ ] Graceful degradation when Mac offline

---

## 5. Audio Format Handling

| Source | Format | Handler |
|--------|--------|---------|
| Telegram | `.ogg` | VPS Whisper (native support) |
| Voice Memos | `.m4a` | Mac Whisper (native support) |

No format conversion needed - Whisper handles both.

---

## 6. Security Non-Negotiables

- [ ] No `sudo NOPASSWD` for clawdbot user
- [ ] No arbitrary shell execution
- [ ] No shared mutable state between Mac/VPS
- [ ] `ask: "always"` on Mac node
- [ ] JSONL audit logging on every action
- [ ] Schema validation on every request/result

---

## 7. Explicit Non-Goals (DO NOT BUILD)

- ❌ Custom crypto / signing (use Clawdbot pairing)
- ❌ Always-on Mac server (use node on-demand)
- ❌ Long-term raw audio on VPS (24h retention max)
- ❌ Agent-decided capability escalation
- ❌ Over-engineered speaker detection tiers (Tier 2 only for now)

---

## 8. Build Order Summary

```
Phase 1: CLI caps (--limit, --max-hours, --dry-run)     [1-2 hours]
Phase 2: Path authority module (paths.rs/paths.py)       [1 hour]
Phase 3: Schemas + validators                            [1 hour]
Phase 4: VPS voice runner daemon                         [2-3 hours]
Phase 5: Speaker detection (Tier 2)                      [2 hours]
Phase 6: Mac diarizer via Clawdbot node                  [2-3 hours]

Total estimated: 10-12 hours
```

---

## 9. Testing Checklist

**Phase 1:**
```bash
arkai voice process --limit 3 --dry-run
arkai voice process --limit 1 --max-hours 0.5 --once
```

**Phase 4:**
```bash
# On VPS
echo '{"id":"test-123","action":"process","params":{"limit":1},"requested_by":"alex","requested_at":"2026-01-28T10:00:00Z"}' > ~/clawd/artifacts/voice/requests/test-123.json
# Watch for result in ~/clawd/artifacts/voice/results/test-123.json
```

**Phase 6:**
```bash
# On Mac
clawdbot node run --host arkai-clawdbot.taila30487.ts.net --display-name "Mac-Diarizer"

# From VPS (Claudia)
clawdbot nodes run --node "Mac-Diarizer" -- python3 ~/services/voice/diarize.py /path/to/audio.ogg
```

---

## 10. Post-Build Verification

- [ ] `arkai voice process --limit 5 --max-hours 1.0` works on Mac
- [ ] VPS runner processes Telegram audio within 60s
- [ ] Speaker detection returns valid JSON
- [ ] Mac diarization works via Clawdbot node
- [ ] Audit logs capture all events
- [ ] Graceful degradation when Mac offline

---

## 11. Implementation Notes (Post-Build Learnings)

> Added after Phase 4 build. Documents what we learned, not changing requirements.

### Phase 4 Simplifications (Approved)

| Spec Proposed | Implemented | Rationale |
|---------------|-------------|-----------|
| watchdog file watcher | 1s polling loop | Matches existing `tts-watcher.py` pattern. Simpler, fewer deps, same latency. |
| async with aiofiles | sync I/O | Groq/OpenAI clients are synchronous; async wrapper added complexity without benefit. |
| Transcript refs in result | Full transcript inline | Prevents data loss. Can optimize to refs later if results get large. |

### Additional Robustness (Added)

- **Atomic claim**: Requests moved to `.inflight/` directory during processing. Prevents double-processing on crashes or races.
- **Idempotency**: Skip if result already exists with `status=completed`.
- **Crash recovery**: On startup, any files in `.inflight/` are moved back to `requests/` for reprocessing.
- **Bounded retries**: 3 attempts per provider with 2s delay between retries.
- **Fail-fast startup**: Daemon exits immediately if no API keys configured (clear error vs. silent failure).

### API Key Security

Keys stored in `~/clawd/services/voice/.env` with `chmod 600`, loaded via systemd `EnvironmentFile=` directive. Never committed to git.

### Audio Retention Policy

| Location | Owner | Retention | Voice Pipeline Role |
|----------|-------|-----------|---------------------|
| `~/.clawdbot/media/inbound/` | Clawdbot | Clawdbot policy | Read-only (don't delete) |
| `~/clawd/artifacts/voice/audio-cache/` | Voice pipeline | 24h auto-cleanup | Temp processing only |
| Mac Voice Memos | Apple/User | User decision | Read-only |

**Principle**: We're a processor, not the source of truth for audio. Source systems own retention decisions.

---

*This spec is FROZEN. Start building.*

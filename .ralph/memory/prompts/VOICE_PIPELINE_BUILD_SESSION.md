---
created: 2026-01-28
purpose: Resume prompt for Voice Pipeline v2.1 build session
---

# Voice Pipeline v2.1 - Build Session

## Context

You are continuing work on the arkai voice pipeline. The architecture has been designed, reviewed by Chad (external advisor), and frozen.

**Read the spec first:**
```
.ralph/memory/specs/VOICE_PIPELINE_V2.1_BUILD_SPEC.md
```

## Key Decisions (Already Made)

1. **VPS uses Whisper API** (Groq free tier → OpenAI fallback), NOT local Whisper
2. **Mac does premium diarization** (pyannote) when online
3. **Never block on Mac** - deliver baseline immediately, queue diarization
4. **Use Clawdbot node** for Mac execution, NOT custom RPC
5. **Always Ask** exec policy on Mac node

## What Exists (DO NOT REBUILD)

- `src/ingest/queue.rs` - Event-sourced voice queue ✅
- `src/ingest/transcriber.rs` - Local Whisper (Mac) ✅
- `src/adapters/clawdbot.rs` - VPS webhook client ✅
- `~/.clawdbot/media/inbound/` - Telegram audio files ✅
- `contracts/voice_intake.schema.json` - Queue item schema ✅

## Build Order

### Phase 1: CLI Caps + Watcher Fixes (START HERE)

**Files to modify:**
- `src/cli/voice.rs` - Add CLI flags
- `src/ingest/watcher.rs` - Add .qta support, bump stability to 10s
- `src/ingest/queue.rs` - Streaming SHA256 hash, add duration_seconds

**Key changes:**
1. Add `--limit`, `--max-hours`, `--dry-run` flags
2. Support `.qta` files (iPhone sync format) + `.m4a`
3. Fix hash function to use streaming (current loads entire file into memory!)
4. Add `duration_seconds` to queue items via ffprobe
5. Add normalization: `.qta` → `.m4a` via ffmpeg before hashing

**Test:**
```bash
cargo build --release
arkai voice process --limit 3 --dry-run
arkai voice process --limit 1 --max-hours 0.5 --once
```

> ⚠️ **Critical**: Read full spec section "Phase 1" for streaming hash and normalization code.

### Phase 2: Path Authority Module

**Create:** `src/config/paths.rs`

Single source of truth for all canonical paths. See spec for details.

### Phase 3: Schemas + Validators

**Create:**
- `contracts/voice_request.schema.json`
- `contracts/voice_result.schema.json`

See spec for schema definitions.

### Phase 4: VPS Voice Runner

**Create on VPS:** `~/clawd/services/voice/vps_voice_runner.py`

Pattern similar to existing `~/tts-watcher.py`.

**Key:** Use Groq/OpenAI Whisper API, NOT local Whisper.

### Phase 5: Speaker Detection (Tier 2)

**Create:** `services/voice/speaker_detector.py`

VAD + MFCC variance approach. Keep simple.

### Phase 6: Mac Diarizer

Via `clawdbot node run`. See spec for exec policy.

## Parallelization

If running multiple sessions:
- **Session A:** Phase 1 → Phase 4 → Phase 6
- **Session B:** Phase 2 + Phase 3 → Phase 5

## Security Reminders

- No arbitrary shell on VPS
- Always Ask on Mac node
- JSONL audit logging everywhere
- Schema validation on all requests/results

## Commands

```bash
# Build arkai
cargo build --release

# Test voice CLI
arkai voice status
arkai voice process --limit 1 --dry-run

# SSH to VPS
ssh clawdbot-vps

# Check VPS services
ssh clawdbot-vps "systemctl --user status vps-voice-runner"
```

---

**Start with Phase 1. Ship incrementally. Test each phase before moving on.**

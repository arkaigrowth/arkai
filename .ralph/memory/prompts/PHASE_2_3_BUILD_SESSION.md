---
created: 2026-01-29
purpose: Resume prompt for Phase 2+3 build session (Track A - Mac/arkai)
depends_on: Phase 1 + 1.5 complete
---

# Phase 2 + 3 - Build Session (Track A)

## Context

You are continuing work on the arkai voice pipeline. Phase 1 (CLI caps, streaming hash, .qta support) and Phase 1.5 (stability hardening) are complete.

**Read the spec first:**
```
.ralph/memory/specs/VOICE_PIPELINE_V2.1_BUILD_SPEC.md
```

## What's Done (DO NOT REBUILD)

- Phase 1: CLI flags (--limit, --max-hours, --dry-run) ✅
- Phase 1: Streaming SHA256 hash ✅
- Phase 1: duration_seconds via ffprobe ✅
- Phase 1: .qta → .m4a normalization ✅
- Phase 1.5: Stability hardening (mtime, min_age, 2-check, deferral) ✅

## Build Order

### Phase 2: Path Authority Module (1 hour)

**Create:** `src/config/paths.rs`

```rust
//! Canonical paths for arkai voice pipeline.
//! Single source of truth - import this instead of hardcoding paths.

use std::path::PathBuf;
use anyhow::Result;

// Mac paths
pub fn arkai_home() -> Result<PathBuf> { crate::config::arkai_home() }
pub fn voice_queue() -> Result<PathBuf> { Ok(arkai_home()?.join("voice_queue.jsonl")) }
pub fn voice_cache() -> Result<PathBuf> { crate::config::voice_cache_dir() }
pub fn library_voice() -> Result<PathBuf> { Ok(crate::config::library_dir()?.join("voice")) }

// VPS paths (for reference - used by Python code)
pub const VPS_ARTIFACTS: &str = "~/clawd/artifacts/voice";
pub const VPS_REQUESTS: &str = "~/clawd/artifacts/voice/requests";
pub const VPS_RESULTS: &str = "~/clawd/artifacts/voice/results";
pub const VPS_AUDIO_CACHE: &str = "~/clawd/artifacts/voice/audio-cache";

// Clawdbot paths (read-only)
pub const TELEGRAM_INBOUND: &str = "~/.clawdbot/media/inbound";

// Voice Memos watch path
pub fn voice_memos_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join("Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings")
}
```

**Also create:** `services/voice/paths.py` (for VPS)
```python
"""Canonical paths for voice pipeline - VPS side."""
from pathlib import Path

# VPS paths (owned by VPS)
VPS_ARTIFACTS = Path.home() / "clawd/artifacts/voice"
VPS_REQUESTS = VPS_ARTIFACTS / "requests"
VPS_RESULTS = VPS_ARTIFACTS / "results"
VPS_AUDIO_CACHE = VPS_ARTIFACTS / "audio-cache"
VPS_AUDIT_LOG = VPS_ARTIFACTS / "audit.jsonl"

# Clawdbot paths (read-only for pipeline)
TELEGRAM_INBOUND = Path.home() / ".clawdbot/media/inbound"
```

**Acceptance:**
- [ ] All new Rust code imports paths from `crate::config::paths`
- [ ] All new Python code imports paths from `services.voice.paths`
- [ ] No hardcoded paths in new code

---

### Phase 3: Schemas + Validators (1 hour)

**Create schemas in `contracts/`:**

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

**Create validator:** `services/voice/validator.py`
```python
"""JSON Schema validator for voice pipeline requests/results."""
import json
from pathlib import Path
from jsonschema import validate, ValidationError

CONTRACTS_DIR = Path(__file__).parent.parent.parent / "contracts"

def load_schema(name: str) -> dict:
    schema_path = CONTRACTS_DIR / f"{name}.schema.json"
    return json.loads(schema_path.read_text())

def validate_request(data: dict) -> None:
    """Validate a voice request. Raises ValidationError if invalid."""
    validate(data, load_schema("voice_request"))

def validate_result(data: dict) -> None:
    """Validate a voice result. Raises ValidationError if invalid."""
    validate(data, load_schema("voice_result"))
```

**Acceptance:**
- [ ] Schemas created in `contracts/`
- [ ] Python validator works: `python -c "from services.voice.validator import validate_request"`
- [ ] All requests/results will be validated before write (Phase 4 will use this)

---

## Test Commands

```bash
# Build
cargo build --release

# Verify paths module
cargo test paths

# Verify Python validator
cd services/voice && python -c "from validator import validate_request; print('OK')"
```

---

## Parallelization Note

While you work on Phase 2+3, another session is building Phase 4 (VPS voice runner) in parallel.

**Sync point**: Phase 4 needs schemas from Phase 3 to validate requests/results.
- Commit schemas early so VPS session can pull them
- Or: VPS session can use inline schema until Phase 3 is done

---

**Start with Phase 2. Ship incrementally. Test before moving on.**

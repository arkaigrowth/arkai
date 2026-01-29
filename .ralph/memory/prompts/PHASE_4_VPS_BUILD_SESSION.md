---
created: 2026-01-29
purpose: Resume prompt for Phase 4 build session (Track B - VPS/clawd)
depends_on: Phase 1 + 1.5 complete, Phase 3 schemas (can inline temporarily)
---

# Phase 4 - VPS Voice Runner (Track B)

## Context

You are building the VPS-side voice runner daemon. This runs on `clawdbot-vps` (Hetzner CAX21, 4 ARM64 cores, 7.5GB RAM, NO GPU).

**Read the spec first:**
```
.ralph/memory/specs/VOICE_PIPELINE_V2.1_BUILD_SPEC.md
```

**VPS Access:**
```bash
ssh clawdbot-vps
# or: ssh 100.81.12.50 (Tailscale)
```

## Key Decisions (Already Made)

1. **Use Whisper API, NOT local Whisper** - VPS has no GPU, local inference would be 30-60s/min
2. **Groq primary, OpenAI fallback** - Groq is free tier and fast
3. **Owns ~/clawd/artifacts/voice/** - VPS writes here, Mac never touches
4. **JSONL audit logging** - Every action logged

## Directory Setup (One-time)

```bash
ssh clawdbot-vps "mkdir -p ~/clawd/artifacts/voice/{requests,results,audio-cache}"
ssh clawdbot-vps "mkdir -p ~/clawd/services/voice"
```

## Build Order

### Step 1: Create VPS Voice Runner Daemon

**Location:** `~/clawd/services/voice/vps_voice_runner.py` (on VPS)

```python
#!/usr/bin/env python3
"""
VPS Voice Runner - watches for voice requests and processes via Whisper API.

Pattern: Similar to existing tts-watcher.py
Transcription: Groq (free) → OpenAI (fallback)
"""
import asyncio
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional
import aiofiles
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler

# Paths (will import from paths.py when Phase 3 is done)
REQUESTS_DIR = Path.home() / "clawd/artifacts/voice/requests"
RESULTS_DIR = Path.home() / "clawd/artifacts/voice/results"
AUDIO_CACHE = Path.home() / "clawd/artifacts/voice/audio-cache"
AUDIT_LOG = Path.home() / "clawd/artifacts/voice/audit.jsonl"
TELEGRAM_INBOUND = Path.home() / ".clawdbot/media/inbound"

# API Keys
GROQ_API_KEY = os.environ.get("GROQ_API_KEY")
OPENAI_API_KEY = os.environ.get("OPENAI_API_KEY")


async def append_audit(event: str, data: dict):
    """Append to JSONL audit log."""
    entry = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "event": event,
        **data
    }
    async with aiofiles.open(AUDIT_LOG, "a") as f:
        await f.write(json.dumps(entry) + "\n")


async def transcribe_groq(audio_path: Path) -> Optional[str]:
    """Transcribe using Groq Whisper API (free tier)."""
    if not GROQ_API_KEY:
        return None
    try:
        from groq import Groq
        client = Groq(api_key=GROQ_API_KEY)
        with open(audio_path, "rb") as f:
            result = client.audio.transcriptions.create(
                file=f,
                model="whisper-large-v3",
                response_format="text"
            )
        return result
    except Exception as e:
        print(f"Groq failed: {e}")
        return None


async def transcribe_openai(audio_path: Path) -> Optional[str]:
    """Transcribe using OpenAI Whisper API (fallback, $0.006/min)."""
    if not OPENAI_API_KEY:
        return None
    try:
        from openai import OpenAI
        client = OpenAI(api_key=OPENAI_API_KEY)
        with open(audio_path, "rb") as f:
            result = client.audio.transcriptions.create(
                file=f,
                model="whisper-1",
                response_format="text"
            )
        return result
    except Exception as e:
        print(f"OpenAI failed: {e}")
        return None


async def transcribe(audio_path: Path) -> tuple[Optional[str], str]:
    """Transcribe audio using Groq (primary) or OpenAI (fallback).
    Returns: (transcript, provider) or (None, "failed")
    """
    # Try Groq first (free)
    result = await transcribe_groq(audio_path)
    if result:
        return result, "groq"

    # Fallback to OpenAI
    result = await transcribe_openai(audio_path)
    if result:
        return result, "openai"

    return None, "failed"


async def process_request(request_path: Path):
    """Process a voice request."""
    request_id = request_path.stem

    # Load and validate request
    request = json.loads(request_path.read_text())
    await append_audit("request_received", {"id": request_id, "requested_by": request.get("requested_by")})

    # TODO: Validate against schema (Phase 3)
    # from validator import validate_request
    # validate_request(request)

    action = request.get("action")
    if action != "process":
        print(f"Unknown action: {action}")
        return

    params = request.get("params", {})
    limit = params.get("limit", 10)

    # Find audio files to process (from Telegram inbound)
    audio_files = list(TELEGRAM_INBOUND.glob("*.ogg"))[:limit]

    if not audio_files:
        # Write empty result
        result = {
            "id": request_id,
            "status": "completed",
            "processed_count": 0,
            "total_duration_seconds": 0,
            "completed_at": datetime.now(timezone.utc).isoformat()
        }
        (RESULTS_DIR / f"{request_id}.json").write_text(json.dumps(result, indent=2))
        await append_audit("result_written", {"id": request_id, "status": "completed", "count": 0})
        return

    # Process each audio file
    transcripts = []
    total_duration = 0.0

    for audio_path in audio_files:
        print(f"Processing: {audio_path.name}")

        # Transcribe
        transcript, provider = await transcribe(audio_path)

        if transcript:
            transcripts.append({
                "file": audio_path.name,
                "transcript": transcript,
                "provider": provider
            })
            await append_audit("transcription_complete", {
                "id": request_id,
                "file": audio_path.name,
                "provider": provider
            })
        else:
            await append_audit("transcription_failed", {"id": request_id, "file": audio_path.name})

    # Write result
    result = {
        "id": request_id,
        "status": "completed" if transcripts else "failed",
        "processed_count": len(transcripts),
        "total_duration_seconds": total_duration,
        "baseline": {
            "transcript_refs": [t["file"] for t in transcripts]
        },
        "completed_at": datetime.now(timezone.utc).isoformat()
    }

    result_path = RESULTS_DIR / f"{request_id}.json"
    result_path.write_text(json.dumps(result, indent=2))
    await append_audit("result_written", {"id": request_id, "status": result["status"]})
    print(f"Result written: {result_path}")


class RequestHandler(FileSystemEventHandler):
    """Watch for new request files."""

    def __init__(self, loop):
        self.loop = loop

    def on_created(self, event):
        if event.is_directory:
            return
        if event.src_path.endswith(".json"):
            path = Path(event.src_path)
            print(f"New request: {path.name}")
            asyncio.run_coroutine_threadsafe(
                process_request(path),
                self.loop
            )


async def main():
    print("VPS Voice Runner starting...")
    print(f"Watching: {REQUESTS_DIR}")
    print(f"Results: {RESULTS_DIR}")

    # Ensure directories exist
    REQUESTS_DIR.mkdir(parents=True, exist_ok=True)
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    AUDIO_CACHE.mkdir(parents=True, exist_ok=True)

    # Set up file watcher
    loop = asyncio.get_event_loop()
    handler = RequestHandler(loop)
    observer = Observer()
    observer.schedule(handler, str(REQUESTS_DIR), recursive=False)
    observer.start()

    await append_audit("runner_started", {"pid": os.getpid()})

    try:
        while True:
            await asyncio.sleep(1)
    except KeyboardInterrupt:
        observer.stop()
    observer.join()


if __name__ == "__main__":
    asyncio.run(main())
```

### Step 2: Systemd Service

**Create:** `~/.config/systemd/user/vps-voice-runner.service`

```ini
[Unit]
Description=VPS Voice Runner
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/python3 /home/clawdbot/clawd/services/voice/vps_voice_runner.py
Restart=on-failure
RestartSec=5
Environment=GROQ_API_KEY=your_key_here
Environment=OPENAI_API_KEY=your_key_here

[Install]
WantedBy=default.target
```

**Enable:**
```bash
systemctl --user daemon-reload
systemctl --user enable vps-voice-runner
systemctl --user start vps-voice-runner
systemctl --user status vps-voice-runner
```

### Step 3: Test

```bash
# Create test request
echo '{"id":"test-001","action":"process","params":{"limit":1},"requested_by":"alex","requested_at":"2026-01-29T10:00:00Z"}' > ~/clawd/artifacts/voice/requests/test-001.json

# Watch for result
watch -n1 "cat ~/clawd/artifacts/voice/results/test-001.json 2>/dev/null || echo 'waiting...'"

# Check audit log
tail ~/clawd/artifacts/voice/audit.jsonl
```

---

## Dependencies

```bash
# On VPS
pip3 install groq openai watchdog aiofiles jsonschema
```

---

## Acceptance Criteria

- [ ] Daemon starts on VPS boot
- [ ] Processes requests within 60s
- [ ] Writes valid result JSON
- [ ] Appends audit log for every action
- [ ] Groq → OpenAI fallback works

---

## Security Reminders

- No sudo NOPASSWD for clawdbot
- No arbitrary shell execution
- JSONL audit logging on every action
- Schema validation on all requests/results (when Phase 3 schemas ready)

---

**Start with Step 1. Ship incrementally. Test before moving on.**

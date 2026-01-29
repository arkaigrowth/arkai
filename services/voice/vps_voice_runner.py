#!/usr/bin/env python3
"""
VPS Voice Runner - Simplified polling daemon for voice transcription.

Pattern: Matches tts-watcher.py (sync polling, 1s interval)
Transcription: Groq (free, primary) → OpenAI (fallback)

Non-negotiables implemented:
1. Atomic claim: mv to .inflight/ prevents double-processing
2. Idempotency: skip if result exists with status=completed
3. Transcripts: stored inline in result JSON
4. Audit JSONL: received → claimed → transcribed → result_written → error
5. Retention: cleanup audio-cache >24h at startup
"""
import json
import os
import shutil
import sys
import time
from datetime import datetime, timezone, timedelta
from pathlib import Path
from typing import Optional

# =============================================================================
# Configuration - Import canonical paths from paths.py
# =============================================================================

from .paths import (
    VPS_ARTIFACTS,
    VPS_REQUESTS,
    VPS_RESULTS,
    VPS_AUDIO_CACHE,
    VPS_AUDIT_LOG,
    TELEGRAM_INBOUND,
)
from .clawdbot_client import ClawdbotClient

# Alias to match existing code (simpler names)
VOICE_DIR = VPS_ARTIFACTS
REQUESTS_DIR = VPS_REQUESTS
INFLIGHT_DIR = VPS_REQUESTS / ".inflight"  # Derived from canonical path
RESULTS_DIR = VPS_RESULTS
AUDIO_CACHE = VPS_AUDIO_CACHE
AUDIT_LOG = VPS_AUDIT_LOG

POLL_INTERVAL_SECS = 1
MAX_RETRIES = 3
RETRY_DELAY_SECS = 2
AUDIO_RETENTION_HOURS = 24

# =============================================================================
# Startup Validation
# =============================================================================

def validate_startup() -> tuple[Optional[str], Optional[str], Optional[ClawdbotClient]]:
    """Validate environment and return (groq_key, openai_key, clawdbot_client).

    Fail fast if no transcription API keys.
    Clawdbot client is optional (graceful degradation if not configured).
    """
    groq_key = os.environ.get("GROQ_API_KEY")
    openai_key = os.environ.get("OPENAI_API_KEY")

    if not groq_key and not openai_key:
        print("[FATAL] Neither GROQ_API_KEY nor OPENAI_API_KEY set. Cannot transcribe.")
        sys.exit(1)

    if not TELEGRAM_INBOUND.exists():
        print(f"[WARN] Telegram inbound dir not found: {TELEGRAM_INBOUND}")

    # Ensure directories exist
    REQUESTS_DIR.mkdir(parents=True, exist_ok=True)
    INFLIGHT_DIR.mkdir(parents=True, exist_ok=True)
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    AUDIO_CACHE.mkdir(parents=True, exist_ok=True)

    # Initialize Clawdbot client (optional)
    clawdbot_client = None
    try:
        clawdbot_client = ClawdbotClient.from_env()
        print("[CONFIG] Clawdbot webhook: enabled")
    except ValueError as e:
        print(f"[WARN] Clawdbot webhook disabled: {e}")

    return groq_key, openai_key, clawdbot_client

# =============================================================================
# Audit Logging
# =============================================================================

def audit(event: str, request_id: str = "", **kwargs):
    """Append to JSONL audit log."""
    entry = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "event": event,
    }
    if request_id:
        entry["id"] = request_id
    entry.update(kwargs)

    with open(AUDIT_LOG, "a") as f:
        f.write(json.dumps(entry) + "\n")
    print(f"[AUDIT] {event}" + (f" id={request_id}" if request_id else ""))

# =============================================================================
# Retention Cleanup
# =============================================================================

def cleanup_old_cache():
    """Delete audio-cache files older than AUDIO_RETENTION_HOURS."""
    if not AUDIO_CACHE.exists():
        return

    cutoff = datetime.now() - timedelta(hours=AUDIO_RETENTION_HOURS)
    deleted = 0

    for f in AUDIO_CACHE.iterdir():
        if f.is_file():
            mtime = datetime.fromtimestamp(f.stat().st_mtime)
            if mtime < cutoff:
                f.unlink()
                deleted += 1

    if deleted:
        audit("cache_cleanup", count=deleted)

# =============================================================================
# Transcription (Groq → OpenAI fallback with retries)
# =============================================================================

def transcribe_groq(audio_path: Path, api_key: str) -> Optional[str]:
    """Transcribe using Groq Whisper API (free tier)."""
    from groq import Groq

    client = Groq(api_key=api_key)
    with open(audio_path, "rb") as f:
        result = client.audio.transcriptions.create(
            file=(audio_path.name, f),
            model="whisper-large-v3",
            response_format="text"
        )
    return result


def transcribe_openai(audio_path: Path, api_key: str) -> Optional[str]:
    """Transcribe using OpenAI Whisper API ($0.006/min)."""
    from openai import OpenAI

    client = OpenAI(api_key=api_key)
    with open(audio_path, "rb") as f:
        result = client.audio.transcriptions.create(
            file=f,
            model="whisper-1",
            response_format="text"
        )
    return result


def transcribe_with_retry(
    audio_path: Path,
    groq_key: Optional[str],
    openai_key: Optional[str]
) -> tuple[Optional[str], str]:
    """
    Transcribe with bounded retries and Groq→OpenAI fallback.
    Returns: (transcript_text, provider) or (None, "failed")
    """
    providers = []
    if groq_key:
        providers.append(("groq", lambda p: transcribe_groq(p, groq_key)))
    if openai_key:
        providers.append(("openai", lambda p: transcribe_openai(p, openai_key)))

    for provider_name, transcribe_fn in providers:
        for attempt in range(1, MAX_RETRIES + 1):
            try:
                result = transcribe_fn(audio_path)
                if result:
                    return result.strip(), provider_name
            except Exception as e:
                print(f"[WARN] {provider_name} attempt {attempt}/{MAX_RETRIES} failed: {e}")
                if attempt < MAX_RETRIES:
                    time.sleep(RETRY_DELAY_SECS)
        print(f"[WARN] {provider_name} exhausted all retries")

    return None, "failed"

# =============================================================================
# Request Processing
# =============================================================================

def is_already_completed(request_id: str) -> bool:
    """Idempotency check: skip if result exists with status=completed."""
    result_path = RESULTS_DIR / f"{request_id}.json"
    if not result_path.exists():
        return False
    try:
        result = json.loads(result_path.read_text())
        return result.get("status") == "completed"
    except Exception:
        return False


def claim_request(request_path: Path) -> Optional[Path]:
    """Atomic claim: move to .inflight/ directory."""
    inflight_path = INFLIGHT_DIR / request_path.name
    try:
        shutil.move(str(request_path), str(inflight_path))
        return inflight_path
    except Exception as e:
        print(f"[WARN] Failed to claim {request_path.name}: {e}")
        return None


def process_request(
    request_path: Path,
    groq_key: Optional[str],
    openai_key: Optional[str],
    clawdbot_client: Optional[ClawdbotClient] = None
):
    """Process a single voice request and notify Claudia."""
    request_id = request_path.stem

    # Idempotency check
    if is_already_completed(request_id):
        print(f"[SKIP] {request_id} already completed")
        request_path.unlink(missing_ok=True)  # Clean up inflight
        return

    audit("received", request_id)

    # Load request
    try:
        request = json.loads(request_path.read_text())
    except Exception as e:
        audit("error", request_id, error=f"Invalid JSON: {e}")
        request_path.unlink(missing_ok=True)
        return

    audit("claimed", request_id, requested_by=request.get("requested_by", "unknown"))

    action = request.get("action")
    if action != "process":
        audit("error", request_id, error=f"Unknown action: {action}")
        write_result(request_id, "failed", error=f"Unknown action: {action}")
        request_path.unlink(missing_ok=True)
        return

    params = request.get("params", {})
    limit = min(params.get("limit", 10), 50)  # Cap at 50

    # Find audio files
    audio_files = sorted(TELEGRAM_INBOUND.glob("*.ogg"))[:limit]

    if not audio_files:
        write_result(request_id, "completed", processed_count=0, transcripts=[])
        audit("result_written", request_id, status="completed", count=0)
        request_path.unlink(missing_ok=True)
        return

    # Process each file
    transcripts = []
    for audio_path in audio_files:
        print(f"[TRANSCRIBE] {audio_path.name}")

        transcript_text, provider = transcribe_with_retry(audio_path, groq_key, openai_key)

        if transcript_text:
            transcripts.append({
                "file": audio_path.name,
                "transcript": transcript_text,
                "provider": provider,
                "transcribed_at": datetime.now(timezone.utc).isoformat()
            })
            audit("transcribed", request_id, file=audio_path.name, provider=provider)
        else:
            audit("error", request_id, file=audio_path.name, error="transcription_failed")

    # Write result
    status = "completed" if transcripts else "failed"
    write_result(
        request_id,
        status,
        processed_count=len(transcripts),
        transcripts=transcripts,
        error=None if transcripts else "All transcriptions failed"
    )
    audit("result_written", request_id, status=status, count=len(transcripts))

    # Notify Claudia of transcription results
    if transcripts:
        notify_claudia(clawdbot_client, request_id, transcripts)

    # Cleanup inflight
    request_path.unlink(missing_ok=True)


def write_result(
    request_id: str,
    status: str,
    processed_count: int = 0,
    transcripts: list = None,
    error: str = None
):
    """Write result JSON to results directory."""
    result = {
        "id": request_id,
        "status": status,
        "processed_count": processed_count,
        "transcripts": transcripts or [],
        "completed_at": datetime.now(timezone.utc).isoformat()
    }
    if error:
        result["error"] = error

    result_path = RESULTS_DIR / f"{request_id}.json"
    result_path.write_text(json.dumps(result, indent=2))


# =============================================================================
# Claudia Notification
# =============================================================================

def notify_claudia(
    clawdbot_client: Optional[ClawdbotClient],
    request_id: str,
    transcripts: list,
    deliver_to_telegram: bool = True
):
    """Notify Claudia of completed transcriptions via webhook.

    Graceful degradation: logs warning if client not configured.
    """
    if not clawdbot_client:
        print("[INFO] Clawdbot not configured, skipping Claudia notification")
        return

    if not transcripts:
        audit("webhook_skipped", request_id, reason="no_transcripts")
        return

    try:
        response = clawdbot_client.send_batch_summary(
            transcripts=transcripts,
            request_id=request_id,
            deliver_to_telegram=deliver_to_telegram,
        )
        audit("webhook_sent", request_id, status=response.status, count=len(transcripts))
        print(f"[NOTIFY] Sent {len(transcripts)} transcript(s) to Claudia: {response.status}")
    except Exception as e:
        audit("webhook_error", request_id, error=str(e))
        print(f"[WARN] Failed to notify Claudia: {e}")

# =============================================================================
# Main Loop
# =============================================================================

def recover_inflight():
    """Recover any requests left in .inflight/ from previous crash."""
    for f in INFLIGHT_DIR.glob("*.json"):
        print(f"[RECOVER] Moving {f.name} back to requests/")
        shutil.move(str(f), str(REQUESTS_DIR / f.name))


def main():
    print("[VPS-VOICE-RUNNER] Starting...")

    # Fail-fast validation
    groq_key, openai_key, clawdbot_client = validate_startup()
    print(f"[CONFIG] Groq: {'✓' if groq_key else '✗'}, OpenAI: {'✓' if openai_key else '✗'}")
    print(f"[CONFIG] Clawdbot: {'✓' if clawdbot_client else '✗'}")
    print(f"[CONFIG] Watching: {REQUESTS_DIR}")
    print(f"[CONFIG] Results: {RESULTS_DIR}")
    print(f"[CONFIG] Telegram audio: {TELEGRAM_INBOUND}")

    # Startup tasks
    cleanup_old_cache()
    recover_inflight()
    audit("runner_started", pid=os.getpid())

    # Main polling loop
    while True:
        for request_path in REQUESTS_DIR.glob("*.json"):
            # Atomic claim
            inflight_path = claim_request(request_path)
            if inflight_path:
                try:
                    process_request(inflight_path, groq_key, openai_key, clawdbot_client)
                except Exception as e:
                    audit("error", request_path.stem, error=str(e))
                    print(f"[ERROR] Processing {request_path.name}: {e}")

        time.sleep(POLL_INTERVAL_SECS)


if __name__ == "__main__":
    main()

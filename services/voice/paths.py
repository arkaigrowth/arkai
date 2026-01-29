"""Canonical paths for voice pipeline - VPS side.

Single source of truth for all path definitions used by VPS Python code.
Import this module instead of hardcoding paths.

Usage:
    from services.voice.paths import VPS_REQUESTS, VPS_RESULTS

Path Ownership:
    VPS owns:    ~/clawd/artifacts/voice/* (requests, results, audio-cache, audit)
    Clawdbot:    ~/.clawdbot/media/inbound (read-only for pipeline)
    Mac owns:    ~/.arkai/* (NOT accessible from VPS)
"""

from pathlib import Path

# ============================================================================
# VPS paths (owned by VPS voice pipeline)
# ============================================================================

# Root directory for all voice artifacts
VPS_ARTIFACTS: Path = Path.home() / "clawd/artifacts/voice"

# Incoming request files from Claudia/Alex
VPS_REQUESTS: Path = VPS_ARTIFACTS / "requests"

# Result files written after processing
VPS_RESULTS: Path = VPS_ARTIFACTS / "results"

# Cached audio files (Telegram downloads, etc.)
VPS_AUDIO_CACHE: Path = VPS_ARTIFACTS / "audio-cache"

# Audit log for all voice pipeline events (append-only JSONL)
VPS_AUDIT_LOG: Path = VPS_ARTIFACTS / "audit.jsonl"

# ============================================================================
# Clawdbot paths (read-only for voice pipeline)
# ============================================================================

# Telegram inbound media - Clawdbot writes here, we read
TELEGRAM_INBOUND: Path = Path.home() / ".clawdbot/media/inbound"


def ensure_directories() -> None:
    """Create all required VPS directories if they don't exist.

    Call this during service startup to ensure directory structure exists.
    """
    VPS_ARTIFACTS.mkdir(parents=True, exist_ok=True)
    VPS_REQUESTS.mkdir(parents=True, exist_ok=True)
    VPS_RESULTS.mkdir(parents=True, exist_ok=True)
    VPS_AUDIO_CACHE.mkdir(parents=True, exist_ok=True)


if __name__ == "__main__":
    # Quick verification when run directly
    print(f"VPS_ARTIFACTS:    {VPS_ARTIFACTS}")
    print(f"VPS_REQUESTS:     {VPS_REQUESTS}")
    print(f"VPS_RESULTS:      {VPS_RESULTS}")
    print(f"VPS_AUDIO_CACHE:  {VPS_AUDIO_CACHE}")
    print(f"VPS_AUDIT_LOG:    {VPS_AUDIT_LOG}")
    print(f"TELEGRAM_INBOUND: {TELEGRAM_INBOUND}")

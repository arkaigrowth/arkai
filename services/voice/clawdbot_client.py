"""Clawdbot webhook client for notifying Claudia of transcripts.

Mirrors the Rust adapter (src/adapters/clawdbot.rs).
Endpoint: POST /hooks/agent
Auth: Bearer token

Usage:
    client = ClawdbotClient.from_env()
    client.send_voice_transcript(transcript, audio_file, duration_secs)
"""
import os
from dataclasses import dataclass
from typing import Optional
import urllib.request
import urllib.error
import json


@dataclass
class WebhookResponse:
    """Response from clawdbot webhook."""
    status: str
    message: Optional[str] = None


class ClawdbotClient:
    """Client for sending voice transcripts to Claudia via clawdbot webhook."""

    DEFAULT_ENDPOINT = "https://arkai-clawdbot.taila30487.ts.net/hooks/agent"

    def __init__(self, endpoint: str, token: str):
        self.endpoint = endpoint
        self.token = token

    @classmethod
    def from_env(cls) -> "ClawdbotClient":
        """Create client from environment variables.

        Env vars:
            CLAWDBOT_ENDPOINT: Webhook URL (optional, has default)
            CLAWDBOT_TOKEN: Required bearer token
        """
        endpoint = os.environ.get("CLAWDBOT_ENDPOINT", cls.DEFAULT_ENDPOINT)
        token = os.environ.get("CLAWDBOT_TOKEN")
        if not token:
            raise ValueError("CLAWDBOT_TOKEN environment variable required")
        return cls(endpoint, token)

    def send_voice_transcript(
        self,
        transcript: str,
        audio_file: str,
        duration_secs: float = 0,
        deliver_to_telegram: bool = True,
        telegram_chat_id: Optional[str] = None,
    ) -> WebhookResponse:
        """Send a voice transcript to Claudia.

        Args:
            transcript: The transcribed text
            audio_file: Source audio filename (for context)
            duration_secs: Audio duration in seconds
            deliver_to_telegram: Whether to send response to Telegram
            telegram_chat_id: Optional specific chat ID

        Returns:
            WebhookResponse with status and optional message
        """
        # Format message with context (matches Rust adapter)
        audio_id = audio_file[:8] if len(audio_file) >= 8 else audio_file
        message = f"[Voice Memo | id:{audio_id} | {duration_secs:.0f}s]\n\n{transcript}"

        payload = {
            "message": message,
            "name": "Voice",
            "session_key": "hook:voice:main",
            "deliver": deliver_to_telegram,
        }

        if deliver_to_telegram:
            payload["channel"] = "telegram"
            if telegram_chat_id:
                payload["to"] = telegram_chat_id

        return self._post(payload)

    def send_batch_summary(
        self,
        transcripts: list[dict],
        request_id: str,
        deliver_to_telegram: bool = True,
    ) -> WebhookResponse:
        """Send a summary of batch transcription to Claudia.

        Args:
            transcripts: List of {"file": str, "transcript": str, "provider": str}
            request_id: The processing request ID
            deliver_to_telegram: Whether to send response to Telegram

        Returns:
            WebhookResponse with status and optional message
        """
        if not transcripts:
            return WebhookResponse(status="skipped", message="No transcripts to send")

        # Format batch summary
        lines = [f"[Voice Batch | id:{request_id[:8]} | {len(transcripts)} memo(s)]", ""]
        for i, t in enumerate(transcripts, 1):
            lines.append(f"**Memo {i}** ({t.get('file', 'unknown')[:20]}):")
            lines.append(t.get("transcript", "(no transcript)"))
            lines.append("")

        message = "\n".join(lines)

        payload = {
            "message": message,
            "name": "Voice",
            "session_key": "hook:voice:main",
            "deliver": deliver_to_telegram,
        }

        if deliver_to_telegram:
            payload["channel"] = "telegram"

        return self._post(payload)

    def _post(self, payload: dict) -> WebhookResponse:
        """Send POST request to webhook endpoint."""
        data = json.dumps(payload).encode("utf-8")
        headers = {
            "Authorization": f"Bearer {self.token}",
            "Content-Type": "application/json",
        }

        req = urllib.request.Request(self.endpoint, data=data, headers=headers, method="POST")

        try:
            with urllib.request.urlopen(req, timeout=30) as response:
                body = response.read().decode("utf-8")
                try:
                    result = json.loads(body)
                    return WebhookResponse(
                        status=result.get("status", "ok"),
                        message=result.get("message"),
                    )
                except json.JSONDecodeError:
                    return WebhookResponse(status="ok", message=body)
        except urllib.error.HTTPError as e:
            # 202 Accepted is expected for async processing
            if e.code == 202:
                return WebhookResponse(status="accepted", message="Processing")
            raise RuntimeError(f"Webhook error ({e.code}): {e.read().decode('utf-8')}")
        except urllib.error.URLError as e:
            raise RuntimeError(f"Webhook connection failed: {e.reason}")

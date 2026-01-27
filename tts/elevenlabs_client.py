"""
ElevenLabs TTS client wrapper.

Provides a simple interface to the ElevenLabs API for text-to-speech.
Uses the new ElevenLabs SDK (v1.x+).
"""

import os
from pathlib import Path
from typing import Optional

from elevenlabs import ElevenLabs, save


class ElevenLabsError(Exception):
    """ElevenLabs API error."""
    pass


class ElevenLabsTTS:
    """
    ElevenLabs Text-to-Speech client.

    Usage:
        tts = ElevenLabsTTS.from_env()
        tts.speak("Hello world", "output.mp3")
        tts.list_voices()
    """

    # Default voice for Claudia (Rachel)
    DEFAULT_VOICE_ID = "21m00Tcm4TlvDq8ikWAM"  # Rachel
    DEFAULT_VOICE_NAME = "Rachel"

    # Model options
    MODEL_MONO_V1 = "eleven_monolingual_v1"
    MODEL_MULTI_V2 = "eleven_multilingual_v2"
    MODEL_TURBO = "eleven_turbo_v2_5"

    # Character limit warning threshold (free tier = 10k/month)
    CHAR_WARNING_THRESHOLD = 500

    def __init__(self, api_key: str, default_voice_id: str = DEFAULT_VOICE_ID):
        """
        Initialize the TTS client.

        Args:
            api_key: ElevenLabs API key
            default_voice_id: Default voice ID
        """
        if not api_key:
            raise ElevenLabsError("API key is required")

        self.client = ElevenLabs(api_key=api_key)
        self.default_voice_id = default_voice_id
        self._voice_cache: Optional[dict] = None

    @classmethod
    def from_env(cls, voice_id: Optional[str] = None) -> "ElevenLabsTTS":
        """
        Create client from environment variable.

        Looks for ELEVENLABS_API_KEY env var.
        """
        api_key = os.environ.get("ELEVENLABS_API_KEY")
        if not api_key:
            raise ElevenLabsError(
                "ELEVENLABS_API_KEY environment variable not set.\n"
                "Get your API key from: https://elevenlabs.io/app/settings/api-keys"
            )
        return cls(api_key, voice_id or cls.DEFAULT_VOICE_ID)

    def speak(
        self,
        text: str,
        output_path: str,
        voice_id: Optional[str] = None,
        model: str = MODEL_TURBO,
        output_format: str = "mp3_44100_128",
    ) -> Path:
        """
        Generate speech from text and save to file.

        Args:
            text: Text to convert to speech
            output_path: Path to save audio file (mp3)
            voice_id: Voice ID (uses default if not specified)
            model: ElevenLabs model to use
            output_format: Audio output format

        Returns:
            Path to the generated audio file
        """
        if not text.strip():
            raise ElevenLabsError("Text cannot be empty")

        voice = voice_id or self.default_voice_id
        output = Path(output_path)

        # Generate audio using new SDK API
        audio = self.client.text_to_speech.convert(
            voice_id=voice,
            text=text,
            model_id=model,
            output_format=output_format,
        )

        # Save to file
        output.parent.mkdir(parents=True, exist_ok=True)
        save(audio, str(output))

        return output

    def list_voices(self) -> list[dict]:
        """
        List available voices.

        Returns:
            List of voice info dicts with name, voice_id, category
        """
        response = self.client.voices.get_all()
        voices = response.voices if hasattr(response, 'voices') else response

        result = []
        for v in voices:
            result.append({
                "name": v.name,
                "voice_id": v.voice_id,
                "category": getattr(v, "category", "unknown"),
            })

        # Cache for voice name -> ID lookup
        self._voice_cache = {v["name"].lower(): v["voice_id"] for v in result}

        return result

    def get_voice_id(self, voice_name: str) -> str:
        """
        Get voice ID from voice name.

        Args:
            voice_name: Voice name (e.g., "Rachel")

        Returns:
            Voice ID string
        """
        # If it looks like an ID already, return as-is
        if len(voice_name) > 15 and voice_name.isalnum():
            return voice_name

        # Build cache if needed
        if self._voice_cache is None:
            self.list_voices()

        voice_id = self._voice_cache.get(voice_name.lower())
        if not voice_id:
            raise ElevenLabsError(f"Voice not found: {voice_name}")

        return voice_id

    @staticmethod
    def char_count(text: str) -> int:
        """Count characters in text (for cost estimation)."""
        return len(text)

    @classmethod
    def estimate_cost(cls, text: str, price_per_char: float = 0.00018) -> dict:
        """
        Estimate cost for generating speech.

        Args:
            text: Text to estimate
            price_per_char: Price per character (default based on Starter plan)

        Returns:
            Dict with char_count, estimated_cost_usd, warning if applicable
        """
        chars = cls.char_count(text)
        cost = chars * price_per_char

        result = {
            "char_count": chars,
            "estimated_cost_usd": round(cost, 4),
        }

        if chars > cls.CHAR_WARNING_THRESHOLD:
            result["warning"] = f"Text has {chars} characters. This will use API quota."

        return result

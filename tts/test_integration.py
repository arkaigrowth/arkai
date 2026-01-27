#!/usr/bin/env python3
"""
Integration test for arkai-tts.

Run this once ELEVENLABS_API_KEY is set to verify everything works.

Usage:
    export ELEVENLABS_API_KEY="your-key"
    python test_integration.py

Or:
    ELEVENLABS_API_KEY=xxx python test_integration.py
"""

import os
import sys
import tempfile
from pathlib import Path

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))


def test_all():
    """Run all integration tests."""
    from tts.elevenlabs_client import ElevenLabsTTS, ElevenLabsError

    print("=" * 60)
    print("arkai-tts Integration Test")
    print("=" * 60)
    print()

    # Test 1: Client initialization
    print("1. Testing client init...")
    try:
        client = ElevenLabsTTS.from_env()
        print("   ✅ Client initialized")
    except ElevenLabsError as e:
        print(f"   ❌ Failed: {e}")
        return False

    # Test 2: List voices
    print("\n2. Testing list voices...")
    try:
        voices = client.list_voices()
        print(f"   ✅ Found {len(voices)} voices")
        
        # Show first 5
        print("   Top voices:")
        for v in voices[:5]:
            print(f"      - {v['name']} ({v['voice_id'][:8]}...)")
    except Exception as e:
        print(f"   ❌ Failed: {e}")
        return False

    # Test 3: Voice lookup
    print("\n3. Testing voice name → ID lookup...")
    try:
        voice_id = client.get_voice_id("Rachel")
        print(f"   ✅ Rachel → {voice_id}")
    except ElevenLabsError as e:
        print(f"   ⚠️  Rachel not found, trying first available...")
        voice_id = voices[0]["voice_id"]
        print(f"   ✅ Using {voices[0]['name']} → {voice_id}")

    # Test 4: Cost estimation (no API call)
    print("\n4. Testing cost estimation...")
    test_text = "Hello, this is Claudia speaking. How can I help you today?"
    estimate = ElevenLabsTTS.estimate_cost(test_text)
    print(f"   ✅ Text: {estimate['char_count']} chars")
    print(f"   ✅ Est. cost: ${estimate['estimated_cost_usd']:.4f}")

    # Test 5: Actual synthesis
    print("\n5. Testing speech synthesis...")
    try:
        with tempfile.NamedTemporaryFile(suffix=".mp3", delete=False) as f:
            output_path = f.name

        result = client.speak(test_text, output_path)
        file_size = result.stat().st_size

        print(f"   ✅ Generated: {output_path}")
        print(f"   ✅ File size: {file_size:,} bytes")

        # Offer to play on macOS
        if sys.platform == "darwin":
            print(f"\n   🔊 To play: afplay {output_path}")

    except Exception as e:
        print(f"   ❌ Synthesis failed: {e}")
        return False

    print()
    print("=" * 60)
    print("✅ All tests passed!")
    print("=" * 60)
    return True


if __name__ == "__main__":
    api_key = os.environ.get("ELEVENLABS_API_KEY")

    if not api_key:
        print("❌ ELEVENLABS_API_KEY not set")
        print()
        print("Get your API key from:")
        print("  https://elevenlabs.io/app/settings/api-keys")
        print()
        print("Then run:")
        print("  export ELEVENLABS_API_KEY='your-key'")
        print("  python test_integration.py")
        sys.exit(1)

    success = test_all()
    sys.exit(0 if success else 1)

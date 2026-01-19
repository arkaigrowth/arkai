# AssemblyAI Speaker Diarization Research

**Research Date:** 2026-01-17
**Purpose:** Evaluate AssemblyAI for speaker diarization in arkai YouTube transcript pipeline

---

## 1. Overview

### What is AssemblyAI?

AssemblyAI is a speech-to-text API platform that provides production-ready AI models for transcription and audio intelligence. The platform offers comprehensive speech understanding capabilities including transcription, speaker diarization, speaker identification, sentiment analysis, content moderation, and summarization.

### Speaker Diarization Product

AssemblyAI's Speaker Diarization model detects multiple speakers in an audio file and attributes speech segments to each speaker. The resulting transcript returns a list of **utterances**, where each utterance corresponds to an uninterrupted segment of speech from a single speaker.

**Key Product Highlights:**
- Supports speaker diarization in **16 languages**
- January 2026 update: **Multichannel speaker diarization** with Universal model
- Industry-leading **2.9% speaker count error rate**
- **30% improvement** in noisy/far-field audio scenarios (2025 update)

---

## 2. Diarization Capabilities

### Accuracy

| Metric | Value | Notes |
|--------|-------|-------|
| Speaker Count Error Rate | **2.9%** | Industry-leading accuracy |
| Noisy/Far-field DER | **20.4%** | Down from 29.1% (30% improvement) |
| Clean Audio Accuracy | **85-95%** | With distinct speakers |
| Minimum Segment Detection | **250ms** | Can track single words |
| Mid-length Reverberant Improvement | **57%** | Recent model update |

### Output Format

The API returns structured JSON with timestamps and speaker labels:

```json
{
  "utterances": [
    {
      "speaker": "A",
      "text": "Hello, how are you today?",
      "start": 0,
      "end": 2340,
      "confidence": 0.95,
      "words": [
        {"text": "Hello", "start": 0, "end": 500, "confidence": 0.98},
        {"text": "how", "start": 520, "end": 720, "confidence": 0.96}
      ]
    },
    {
      "speaker": "B",
      "text": "I'm doing great, thanks for asking.",
      "start": 2500,
      "end": 4800,
      "confidence": 0.93,
      "words": [...]
    }
  ]
}
```

**Timestamp Format:** Milliseconds (integer)

### Speaker Naming/Identification

AssemblyAI offers **two levels** of speaker attribution:

1. **Speaker Diarization** (Basic): Labels speakers as A, B, C, etc.
2. **Speaker Identification** (Advanced): Maps generic labels to actual names

```python
# Speaker Identification with known names
config = aai.TranscriptionConfig(
    speaker_labels=True,
    speech_understanding={
        "request": {
            "speaker_identification": {
                "speaker_type": "name",
                "known_values": ["Michel Martin", "Peter DeCarlo"]
            }
        }
    }
)
```

**Multichannel Labels (January 2026):** Format like `1A`, `1B`, `2A` where first digit = channel, letter = speaker within channel.

### Speaker Limits

| Configuration | Limit |
|--------------|-------|
| Default maximum | **10 speakers** |
| With `speaker_options` | **Up to 50 speakers** |
| Recommended minimum speech per speaker | **30 seconds uninterrupted** |

**Best Practices:**
- Ensure each speaker has at least 30 seconds of uninterrupted speech
- Avoid relying on short phrases ("Yeah", "Right", "Sounds good")
- Minimize cross-talk for best results

---

## 3. Pricing

### Base Pricing (2026)

| Model | Cost | Per Minute |
|-------|------|------------|
| Universal (99 languages) | $0.27/hour | $0.0045/min |
| Best (English-only) | $0.15/hour | $0.0025/min |
| Nano (fast, lower accuracy) | $0.08/hour | $0.00133/min |

### Feature Add-on Costs

| Feature | Additional Cost |
|---------|-----------------|
| Speaker Diarization | +$0.02/hour ($0.00033/min) |
| Speaker Identification | +$0.02/hour |
| Sentiment Analysis | +$0.02/hour |
| PII Redaction | +$0.08/hour |
| Summarization | +$0.03/hour |

### Total Cost Example (Transcription + Diarization)

| Model | Transcription | + Diarization | Total/Hour | Total/Min |
|-------|---------------|---------------|------------|-----------|
| Universal | $0.27 | +$0.02 | **$0.29/hr** | $0.00483/min |
| Best | $0.15 | +$0.02 | **$0.17/hr** | $0.00283/min |

### Free Tier

- **$50 in free credits** upon signup
- Approximately **185 hours** of base Universal transcription
- ~33 hours with all major features enabled

### Billing Model

- **Per-second billing** (not rounded to minutes)
- A 61-second file costs for exactly 61 seconds
- No monthly subscription required (pay-as-you-go)

### Enterprise/Volume Discounts

| Volume Tier | Typical Discount |
|-------------|------------------|
| Mid-tier | 20-30% |
| High volume | 35-45% |
| Enterprise (50,000+ hours/month) | 50%+ |

**Enterprise Features:**
- Dedicated technical support
- Customized rate limits
- Tailored SLAs
- SOC 2 Type 2 compliance
- HIPAA/BAA available
- EU/US region hosting options

---

## 4. API Integration

### SDK Availability

| Language | SDK | Latest Update |
|----------|-----|---------------|
| Python | `assemblyai` | January 16, 2026 |
| Node.js/TypeScript | `assemblyai` | Active |
| Ruby | Official SDK | Active |
| Java | Official SDK | Active |
| Go | Official SDK | Active |

**REST API:** Full REST API available for any language

### Installation

```bash
pip install assemblyai
export ASSEMBLYAI_API_KEY=<YOUR_KEY>
```

### Python SDK Example - Speaker Diarization

```python
import assemblyai as aai

# Configure API key
aai.settings.api_key = "YOUR_API_KEY"

# Basic diarization from URL
audio_file = "https://storage.googleapis.com/aai-web-samples/meeting.mp3"

config = aai.TranscriptionConfig(
    speaker_labels=True,
    speakers_expected=3  # Optional: if you know speaker count
)

transcriber = aai.Transcriber()
transcript = transcriber.transcribe(audio_file, config=config)

# Access utterances with speaker labels
for utterance in transcript.utterances:
    print(f"Speaker {utterance.speaker}: {utterance.text}")
    print(f"  Start: {utterance.start}ms, End: {utterance.end}ms")
```

### Processing Local Files

```python
import assemblyai as aai

# Local file - SDK handles upload automatically
transcript = transcriber.transcribe("./local-audio.wav", config)

# Or upload separately
upload_url = transcriber.upload_file("./local-audio.wav")
transcript = transcriber.transcribe(upload_url, config)
```

### Async Processing Model

AssemblyAI uses an **asynchronous polling model** for batch/pre-recorded audio:

```python
import assemblyai as aai

# Default polling interval: 3 seconds
aai.settings.polling_interval = 1.0  # Adjust if needed

# The SDK handles polling automatically
transcript = transcriber.transcribe(audio_url, config)

# Check status manually if needed
print(transcript.status)  # "completed", "processing", "error"
```

**Alternative: Webhooks**

```python
config = aai.TranscriptionConfig(
    speaker_labels=True,
    webhook_url="https://your-server.com/webhook"
)
```

---

## 5. Quality Comparisons

### AssemblyAI vs WhisperX

| Aspect | AssemblyAI | WhisperX |
|--------|------------|----------|
| **Type** | Commercial API | Open-source (Whisper + Pyannote) |
| **Speaker Diarization** | Native, integrated | Via Pyannote integration |
| **Accuracy (DER)** | 2.9% speaker count error | Dependent on Pyannote (~10-15% DER) |
| **Noisy Audio** | 20.4% DER (optimized) | Variable, needs tuning |
| **Languages** | 16 for diarization | 99+ (Whisper base) |
| **Setup Complexity** | API key only | Significant engineering |
| **Speed** | ~1 min for 1 hr audio | 4x realtime (optimized) |
| **Cost** | $0.17-0.29/hour | Free (compute costs only) |
| **Latency** | 300ms streaming / 15-20 min batch | 380-520ms (optimized) |

### Benchmark Notes

- **No direct head-to-head benchmark** exists comparing AssemblyAI vs WhisperX on identical datasets
- WhisperX relies on Pyannote 3.1 for diarization (open-source SOTA)
- AssemblyAI's proprietary speaker embedding model shows documented improvements in challenging conditions
- For production use, commercial APIs typically provide better ROI when accounting for engineering time

### When to Choose Each

**Choose AssemblyAI when:**
- Need production reliability without DevOps overhead
- Processing variable audio quality (podcasts, meetings)
- Require enterprise compliance (SOC 2, HIPAA)
- Want integrated speaker identification

**Choose WhisperX when:**
- High volume processing (amortized setup cost)
- Full control over pipeline required
- Budget-constrained with available engineering time
- Research/experimentation purposes

---

## 6. Integration Considerations

### Input Formats

**Audio Sources:**
- Public URL (preferred)
- Local file upload via SDK
- Binary data stream

**Supported File Types:**
- MP3, WAV, FLAC, M4A, OGG
- MP4, MOV, AVI, WEBM (video files)
- Most common audio/video formats

### Output Format

**JSON Response Structure:**

```json
{
  "id": "abc123",
  "status": "completed",
  "text": "Full transcript text...",
  "words": [...],
  "utterances": [
    {
      "speaker": "A",
      "text": "Speaker A's utterance",
      "start": 0,
      "end": 2340,
      "confidence": 0.95,
      "words": [...]
    }
  ],
  "audio_duration": 3600000,
  "language_code": "en"
}
```

### Latency Expectations

| Mode | Latency | Use Case |
|------|---------|----------|
| Streaming (real-time) | **300ms P50** | Live transcription |
| Batch (1-hour file) | **15-20 minutes** | Pre-recorded content |
| Concurrent batch (1000 files) | **~Same as single file** | Bulk processing |

**Factors Affecting Latency:**
- Audio quality (poor quality = longer processing)
- Server load during peak hours
- Feature complexity (more features = longer)

### Polling Configuration

```python
# Python SDK defaults
aai.settings.polling_interval = 3.0  # seconds

# Node.js SDK
{
  polling: true,
  polling_interval: 3000,  // ms
  polling_timeout: 180000  // ms
}
```

---

## 7. Pros and Cons for Arkai Integration

### Pros

| Advantage | Relevance to Arkai |
|-----------|-------------------|
| **High accuracy (2.9% speaker count error)** | Critical for interview/podcast transcripts |
| **Native speaker identification** | Can label known hosts/guests |
| **Simple API integration** | Minimal code changes to existing pipeline |
| **Per-second billing** | Cost-efficient for variable-length YouTube videos |
| **URL-based input** | Works directly with YouTube audio URLs |
| **Async processing** | Fits batch processing workflow |
| **16 language support** | Good for multilingual content |
| **SOC 2 compliance** | Enterprise-ready if needed |
| **SDK handles complexity** | Polling, upload, error handling built-in |

### Cons

| Disadvantage | Impact |
|--------------|--------|
| **Cost per minute** | $0.0028-0.0048/min adds up at scale |
| **External API dependency** | Single point of failure, rate limits |
| **No offline processing** | Requires internet connectivity |
| **Speaker ID requires known names upfront** | May not work for unknown guests |
| **10 speaker default limit** | Need config for large panel discussions |
| **Short utterance challenges** | May miss brief acknowledgments |
| **Variable latency under load** | Not ideal for real-time-ish workflows |

### Cost Projection for Arkai

| Monthly Volume | Base + Diarization | With Volume Discount |
|----------------|-------------------|---------------------|
| 100 hours | ~$17-29 | N/A |
| 1,000 hours | ~$170-290 | ~$120-200 (30% off) |
| 10,000 hours | ~$1,700-2,900 | ~$850-1,450 (50% off) |

### Integration Recommendation

**Recommended Approach:**
1. Use AssemblyAI for diarization on content where speaker attribution matters
2. Fall back to WhisperX/local processing for high-volume, lower-priority content
3. Implement caching layer to avoid re-processing
4. Use webhooks for production, polling for development

---

## Summary Table

| Category | Details |
|----------|---------|
| **Product** | Speech-to-text API with speaker diarization |
| **Diarization Accuracy** | 2.9% speaker count error, 20.4% DER (noisy) |
| **Max Speakers** | 10 default, 50 with configuration |
| **Output Format** | JSON with utterances, timestamps (ms), speaker labels |
| **Speaker Naming** | Yes, via Speaker Identification feature |
| **Pricing (with diarization)** | $0.17-0.29/hour ($0.0028-0.0048/min) |
| **Free Tier** | $50 credits (~185 hours base) |
| **SDK** | Python, Node.js, Ruby, Java, Go |
| **Processing Model** | Async with polling or webhooks |
| **Latency** | 300ms streaming, 15-20 min per hour (batch) |
| **Compliance** | SOC 2 Type 2, GDPR, HIPAA available |
| **Languages (diarization)** | 16 |

---

## Sources

- [Speaker Diarization Documentation](https://www.assemblyai.com/docs/pre-recorded-audio/speaker-diarization)
- [Speaker Identification Documentation](https://www.assemblyai.com/docs/speech-understanding/speaker-identification)
- [AssemblyAI Pricing](https://www.assemblyai.com/pricing)
- [Speaker Diarization Update Blog](https://www.assemblyai.com/blog/speaker-diarization-update)
- [Python SDK GitHub](https://github.com/AssemblyAI/assemblyai-python-sdk)
- [How to Perform Speaker Diarization in Python](https://www.assemblyai.com/blog/speaker-diarization-python)
- [Speaker Diarization and Identification Blog](https://www.assemblyai.com/blog/assemblyai-speaker-identification-diarization)
- [Top Speaker Diarization Libraries and APIs](https://www.assemblyai.com/blog/top-speaker-diarization-libraries-and-apis)
- [AssemblyAI Benchmarks](https://www.assemblyai.com/benchmarks)
- [Real-time Speech Recognition](https://www.assemblyai.com/blog/best-api-models-for-real-time-speech-recognition-and-transcription)
- [Batch Transcription](https://www.assemblyai.com/blog/large-scale-audio-transcription)
- [Best Speaker Diarization Models Compared](https://brasstranscripts.com/blog/speaker-diarization-models-comparison)

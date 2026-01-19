# WhisperX Research: Speaker Diarization Capabilities

**Research Date**: 2026-01-17
**Purpose**: Evaluate WhisperX for arkai YouTube transcript pipeline integration

---

## 1. Overview

### What is WhisperX?

WhisperX is a powerful extension to OpenAI's Whisper ASR model, created by Max Bain (University of Oxford). It augments Whisper's transcription capabilities with features that the original model lacks:

- **Word-level timestamps** using wav2vec2 forced alignment
- **Speaker diarization** via pyannote-audio integration
- **Batched inference** for 70x realtime transcription speed
- **Voice Activity Detection (VAD)** for better audio segmentation

### How It Differs from Base Whisper

| Feature | OpenAI Whisper | WhisperX |
|---------|---------------|----------|
| Timestamps | Utterance-level (can be off by seconds) | Word-level (precise) |
| Speaker identification | Not supported | Built-in via pyannote |
| Batching | Not natively supported | 70x realtime with large-v2 |
| VAD | Basic | Advanced preprocessing |
| Memory usage | Higher | Optimized via faster-whisper backend |

**Key insight**: WhisperX uses faster-whisper under the hood, then layers on VAD, forced alignment, and optional diarization. This multi-model approach is heavier but essential for use cases like meeting transcripts, interviews, and podcasts.

---

## 2. Diarization Capabilities

### How Speaker Identification Works

WhisperX uses [pyannote-audio](https://github.com/pyannote/pyannote-audio) for speaker diarization. The process:

1. **Transcription**: Whisper transcribes audio with basic timestamps
2. **Forced Alignment**: wav2vec2 refines to word-level timestamps
3. **Diarization**: pyannote segments audio by speaker
4. **Assignment**: Word segments are mapped to speaker identities

### Output Format

WhisperX outputs in multiple formats: **JSON**, **SRT**, **VTT**, **TXT**, **TSV**, and Audacity format.

**JSON structure with diarization**:
```json
{
  "segments": [
    {
      "start": 0.0,
      "end": 2.5,
      "text": "Hello, welcome to the podcast.",
      "speaker": "SPEAKER_00",
      "words": [
        {"word": "Hello,", "start": 0.0, "end": 0.4, "score": 0.95},
        {"word": "welcome", "start": 0.5, "end": 0.9, "score": 0.92},
        {"word": "to", "start": 1.0, "end": 1.1, "score": 0.98},
        {"word": "the", "start": 1.2, "end": 1.3, "score": 0.97},
        {"word": "podcast.", "start": 1.4, "end": 2.5, "score": 0.89}
      ]
    },
    {
      "start": 2.8,
      "end": 5.2,
      "text": "Thanks for having me.",
      "speaker": "SPEAKER_01",
      "words": [...]
    }
  ],
  "language": "en"
}
```

**SRT output with speaker labels**:
```
1
00:00:00,000 --> 00:00:02,500
[SPEAKER_00]: Hello, welcome to the podcast.

2
00:00:02,800 --> 00:00:05,200
[SPEAKER_01]: Thanks for having me.
```

### Speaker Naming

- Speakers are labeled generically: `SPEAKER_00`, `SPEAKER_01`, `SPEAKER_02`, etc.
- **No automatic speaker naming** - manual post-processing required to assign real names
- Speaker count can be constrained with `--min_speakers` and `--max_speakers` flags

---

## 3. Hardware Requirements

### GPU Requirements

| Model Size | VRAM Required | Notes |
|------------|---------------|-------|
| tiny | ~1 GB | Fast but less accurate |
| base | ~2 GB | Basic quality |
| small | ~3 GB | Good balance for CPU |
| medium | ~5 GB | Recommended minimum |
| large-v2 | ~8-10 GB | Best accuracy |
| large-v3 | ~10 GB | Latest, highest quality |

**Recommended production setup**:
- GPU: NVIDIA RTX 3090 (24GB) or RTX 4090
- CPU: 12+ cores
- RAM: 32GB
- Storage: 100GB SSD

**Minimum setup**:
- GPU: 4GB+ VRAM (8GB+ recommended for large-v2)
- CPU: 4+ cores (8+ recommended)
- RAM: 16GB+
- Storage: 10GB+ for models

### Apple Silicon Support

WhisperX can run on Apple Silicon (M1/M2/M3/M4) with considerations:

| Method | Performance | Notes |
|--------|-------------|-------|
| CPU only | ~1x baseline | Works on any Mac |
| Metal (MPS) | ~3-4x faster | PyTorch MPS support |
| CoreML + Metal | ~8-12x faster | Best performance |
| whisper.cpp | Native GPU | C++ alternative, excellent M-series support |

**Benchmarks on Apple Silicon**:
- M1 Pro: 10-minute audio in ~3:36 (216 seconds)
- M2 Ultra / M3 Max: Significantly faster, comparable to RTX 4090
- Recommended: 16GB RAM minimum, Medium model for best accuracy/speed balance

**Note**: The pyannote diarization models may have reduced optimization for Apple Silicon compared to CUDA.

---

## 4. Installation Complexity

### Basic Installation

```bash
pip install whisperx
```

Or with UV:
```bash
uvx whisperx
```

### Dependencies

**Core dependencies**:
- Python 3.9 - 3.13
- PyTorch with CUDA support (for GPU)
- faster-whisper
- pyannote-audio (for diarization)
- ffmpeg (audio/video processing)

**ffmpeg installation**:
```bash
# macOS
brew install ffmpeg

# Ubuntu/Debian
sudo apt update && sudo apt install ffmpeg

# Windows
# Download from https://ffmpeg.org/download.html
```

### HuggingFace Token Requirements

**Required for speaker diarization**:

1. Create free account at [huggingface.co](https://huggingface.co)
2. Generate Read Token at [huggingface.co/settings/tokens](https://huggingface.co/settings/tokens)
3. Accept terms for required models:
   - [pyannote/segmentation-3.0](https://huggingface.co/pyannote/segmentation-3.0)
   - [pyannote/speaker-diarization-3.1](https://huggingface.co/pyannote/speaker-diarization-3.1)

**Usage**:
```bash
whisperx audio.wav --diarize --hf_token YOUR_TOKEN_HERE
```

Or set environment variable:
```bash
export HF_TOKEN=your_token_here
```

### License Restrictions

| Component | License | Commercial Use |
|-----------|---------|----------------|
| WhisperX | BSD-2-Clause | Allowed |
| OpenAI Whisper | MIT | Allowed |
| faster-whisper | MIT | Allowed |
| pyannote-audio | MIT | Review model-specific terms |
| pyannote models | Gated on HuggingFace | **Requires license review** |

**Important**: While WhisperX itself is permissively licensed (BSD-2-Clause), the pyannote speaker diarization models are "gated" on HuggingFace. You must accept their terms, and for commercial use, you should review the specific license terms carefully. pyannote.ai offers commercial licensing for enterprise use.

---

## 5. Integration Considerations

### Input Formats

WhisperX accepts any format supported by ffmpeg:

**Audio**: MP3, WAV, FLAC, AAC, OGG, M4A, WMA
**Video**: MP4, MKV, AVI, MOV, WMV, FLV, WebM

Input is automatically converted to 16kHz mono WAV internally.

### Output Formats

```bash
# Specify output format
whisperx audio.wav --output_format json
whisperx audio.wav --output_format srt
whisperx audio.wav --output_format all  # All formats
```

Available: `json`, `srt`, `vtt`, `txt`, `tsv`, `aud` (Audacity), `all`

### Working with Existing Transcripts

**Short answer**: Limited support.

WhisperX has a `--skip-whisper` flag to load existing WhisperX transcriptions for re-processing. However, using `whisperx.align()` with arbitrary external transcripts is not straightforward.

**The pipeline requires**:
1. Audio file (always needed for alignment and diarization)
2. Either: Re-transcription, OR existing WhisperX JSON output

**For YouTube integration**: If you already have YouTube auto-generated transcripts, WhisperX would need to re-transcribe the audio to provide diarization. The alignment features work best with its own transcription output.

### Python Integration Example

```python
import whisperx
import gc

# Configuration
device = "cuda"  # or "cpu" for Apple Silicon without CUDA
batch_size = 16  # Reduce if low on GPU memory
compute_type = "float16"  # Use "int8" for lower memory

# 1. Load model and transcribe
model = whisperx.load_model("large-v2", device, compute_type=compute_type)
audio = whisperx.load_audio("podcast.mp3")
result = model.transcribe(audio, batch_size=batch_size)

# 2. Align for word-level timestamps
model_a, metadata = whisperx.load_align_model(
    language_code=result["language"],
    device=device
)
result = whisperx.align(
    result["segments"],
    model_a,
    metadata,
    audio,
    device,
    return_char_alignments=False
)

# Clean up alignment model
gc.collect()

# 3. Speaker diarization
diarize_model = whisperx.DiarizationPipeline(
    use_auth_token="YOUR_HF_TOKEN",
    device=device
)
diarize_segments = diarize_model(audio)

# 4. Assign speakers to words
result = whisperx.assign_word_speakers(diarize_segments, result)

# Access results
for segment in result["segments"]:
    speaker = segment.get("speaker", "UNKNOWN")
    text = segment["text"]
    start = segment["start"]
    end = segment["end"]
    print(f"[{speaker}] {start:.2f}-{end:.2f}: {text}")
```

---

## 6. Pros and Cons for arkai YouTube Pipeline

### Pros

| Advantage | Impact on arkai |
|-----------|-----------------|
| Word-level timestamps | Precise quote extraction, clip generation |
| Speaker diarization | Identify different speakers in interviews/podcasts |
| Multiple output formats | JSON for processing, SRT for subtitles |
| Batched processing | Process long videos efficiently (70x realtime) |
| Open source | No per-API-call costs after setup |
| Active development | Regular updates, good community support |
| Handles video files | Direct YouTube download integration possible |

### Cons

| Limitation | Impact on arkai |
|------------|-----------------|
| HuggingFace dependency | External service required for diarization |
| Requires re-transcription | Cannot enhance existing YouTube transcripts |
| GPU recommended | Apple Silicon works but CUDA preferred |
| Overlapping speech handling | May miss or misattribute overlapping dialogue |
| Generic speaker labels | Manual post-processing needed for names |
| pyannote licensing | Commercial use needs license review |
| Memory intensive | 8-10GB VRAM for best models |
| Setup complexity | Multiple model acceptances required |

### Recommendations for arkai Integration

1. **Best fit**: Long-form content with multiple speakers (podcasts, interviews, panel discussions)

2. **Workflow suggestion**:
   - Download audio via yt-dlp
   - Process with WhisperX (transcription + diarization)
   - Store JSON output with speaker metadata
   - Optional: Manual speaker name mapping for known shows

3. **Hardware recommendation for Mac**:
   - Consider whisper.cpp + pyannote combination for better Apple Silicon optimization
   - Or run WhisperX with CPU/MPS mode for occasional use

4. **Alternative to consider**: For YouTube content specifically, compare with:
   - YouTube's built-in auto-captions (no speaker ID)
   - AssemblyAI/Deepgram APIs (commercial, but simpler)
   - pyannote standalone (just diarization on existing audio)

---

## Summary Table

| Aspect | Details |
|--------|---------|
| **What it is** | Whisper + word alignment + speaker diarization |
| **Speed** | 70x realtime with large-v2 on GPU |
| **Diarization output** | SPEAKER_00, SPEAKER_01, etc. |
| **Min GPU VRAM** | 4GB (8GB+ recommended) |
| **Min RAM** | 16GB |
| **Apple Silicon** | Supported via MPS/CoreML (CUDA preferred) |
| **Key dependency** | pyannote-audio (requires HuggingFace token) |
| **License** | BSD-2-Clause (check pyannote model terms) |
| **Input formats** | Any ffmpeg-supported audio/video |
| **Output formats** | JSON, SRT, VTT, TXT, TSV, Audacity |
| **Can use existing transcripts** | Limited (designed for re-transcription) |

---

## Sources

- [WhisperX GitHub Repository](https://github.com/m-bain/whisperX)
- [Speaker Diarization with WhisperX (DataCurious)](https://datacurious.hashnode.dev/unlocking-audio-insights-speaker-diarization-with-whisperx-for-who-said-what)
- [How to transcribe podcasts with WhisperX (swyx.io)](https://www.swyx.io/transcribe-podcasts-with-whisper)
- [Choosing between Whisper variants (Modal)](https://modal.com/blog/choosing-whisper-variants)
- [WhisperX vs Competitors Benchmark 2026 (BrassTranscripts)](https://brasstranscripts.com/blog/whisperx-vs-competitors-accuracy-benchmark)
- [Whisper Speaker Diarization Python Tutorial 2026 (BrassTranscripts)](https://brasstranscripts.com/blog/whisper-speaker-diarization-guide)
- [Whisper Performance on Apple Silicon (Voicci)](https://www.voicci.com/blog/apple-silicon-whisper-performance.html)
- [Interview transcription using WhisperX (Valor Software)](https://valor-software.com/articles/interview-transcription-using-whisperx-model-part-1)
- [pyannote.ai Terms of Use](https://www.pyannote.ai/terms-of-use)
- [WhisperX PyPI Package](https://pypi.org/project/whisperx/)

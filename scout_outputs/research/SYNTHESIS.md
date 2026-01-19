# Research Synthesis: Diarization + Keyframes Integration

> **Date**: 2026-01-17 | **Session**: 5 | **Status**: Research Complete

---

## Executive Summary

We researched speaker diarization (WhisperX vs AssemblyAI) and keyframe extraction (video-ops) for enhancing arkai transcripts. **Key finding:** Keyframes are already solved via video-ops integration. Diarization requires a build-vs-buy decision.

### Quick Recommendation

| Need | Recommendation | Reasoning |
|------|----------------|-----------|
| **Keyframes** | video-ops (existing) | Already integrated with video-download, no code needed |
| **Diarization (casual)** | AssemblyAI | Simple API, $0.17-0.29/hr, high accuracy |
| **Diarization (high-volume)** | WhisperX | Free after setup, but requires GPU + re-transcription |

---

## Comparison Matrix

### Diarization Options

| Factor | WhisperX (Local) | AssemblyAI (API) |
|--------|------------------|------------------|
| **Cost** | Free (+ compute) | $0.17-0.29/hour |
| **Setup** | Complex (HF token, GPU, pyannote terms) | Simple (API key) |
| **Accuracy** | Good (pyannote-based) | Excellent (2.9% speaker count error) |
| **Apple Silicon** | Slower (MPS), prefer CUDA | N/A (cloud) |
| **Speaker Naming** | No (SPEAKER_00, SPEAKER_01) | Yes (with known names) |
| **Works with YouTube captions** | No (must re-transcribe) | No (must re-transcribe) |
| **Licensing** | BSD-2 but pyannote models gated | Commercial API |
| **Offline capable** | Yes | No |

### Key Insight: Both Require Re-Transcription

Neither WhisperX nor AssemblyAI can enhance existing YouTube auto-captions. They both need to:
1. Download the audio/video
2. Run their own speech-to-text
3. Apply speaker diarization to their own output

This means diarization adds ~15-30 minutes processing time per hour of content.

---

## Keyframes: Already Solved

The `video-ops` skill provides mature keyframe extraction:

```bash
# Auto-extract during download
python3 video-download/scripts/download_single.py "URL" --extract-keyframes

# Manual extraction
python3 video-ops/scripts/extract_keyframes.py video.mp4 --threshold 0.3
```

**Capabilities:**
- Motion-based extraction (configurable threshold)
- Interval-based extraction (every N seconds)
- Output: JPEG images + JSON index with timestamps

**Gap:** No "speaker change" trigger. Would need diarization timestamps first, then extract keyframes at speaker transitions.

---

## Recommended Integration Architecture

### Phase 1: Keyframes Only (Ready Now)

```
Current arkai flow:
  URL → fabric -y (YouTube captions) → extract_wisdom → library

Enhanced with keyframes (minimal change):
  URL → video-download + keyframes → arkai ingest → library
                ↓
        Library folder now includes:
        ├── fetch.md (transcript)
        ├── wisdom.md
        ├── summary.md
        └── keyframes/
            ├── keyframe_0001.jpg
            └── keyframes_index.json
```

**Implementation:**
1. Run `video-download` with `--extract-keyframes`
2. Modify arkai ingest to accept `--keyframes-dir` parameter
3. Copy keyframes to library folder during ingest

### Phase 2: Add Diarization (Decision Required)

```
With diarization:
  URL → video-download → diarization step → fabric patterns → library
              ↓                 ↓
          Audio file     [Speaker] tagged transcript
```

**Option A: AssemblyAI (Recommended for simplicity)**
```python
# In arkai adapter or pipeline script
import assemblyai as aai

config = aai.TranscriptionConfig(speaker_labels=True)
transcript = aai.Transcriber().transcribe(audio_url, config)

# Transform output to arkai format
for utterance in transcript.utterances:
    formatted = f"[{utterance.start//1000}s] [{utterance.speaker}] {utterance.text}"
```

**Option B: WhisperX (For high-volume/cost-sensitive)**
```bash
# Add as arkai adapter or pipeline step
whisperx audio.mp3 --diarize --hf_token $HF_TOKEN --output_format json
```

---

## Cost Analysis

### Per-Video Cost (30-min YouTube video)

| Component | WhisperX | AssemblyAI |
|-----------|----------|------------|
| Transcription + Diarization | ~$0 (local compute) | ~$0.14 |
| Keyframe extraction | ~$0 (local compute) | ~$0 (local compute) |
| **Total** | **~$0** | **~$0.14** |

### At Scale (100 hours/month)

| Approach | Cost | Complexity |
|----------|------|------------|
| WhisperX | ~$0 + compute time | High (setup, GPU) |
| AssemblyAI | ~$17-29/month | Low (API call) |
| Hybrid | ~$5-10/month | Medium |

**Hybrid approach:** Use AssemblyAI for new content, WhisperX for bulk backfill.

---

## Recommended Implementation Plan

### Immediate (This Week)

1. **Test video-download keyframe integration**
   - Run: `python3 download_single.py "YOUTUBE_URL" --extract-keyframes`
   - Verify keyframes land in expected location
   - Confirm keyframes_index.json has usable timestamps

2. **Design library folder enhancement**
   - Add `keyframes/` directory to library structure
   - Update AIOS_BRIEF.md with new folder structure

### Short-Term (1-2 Weeks)

3. **Pick diarization approach**
   - If casual use (< 50 hours/month): Go with AssemblyAI
   - If high volume: Set up WhisperX locally

4. **Create diarization adapter**
   - Either: `arkai-assemblyai-adapter.py`
   - Or: `arkai-whisperx-adapter.py`
   - Both output: `[timestamp] [Speaker] text` format

### Medium-Term (Month 2)

5. **Enhanced transcript format**
   ```markdown
   [00:00:02] [Daniel] Hey, what's up? So I want to talk about AI agents.
   [KEYFRAME: keyframes/keyframe_0001.jpg @ 5.0s]
   [00:00:08] [Mike] Yeah, this is fascinating stuff.
   ```

6. **Speaker-change keyframes**
   - Parse diarization output for speaker transitions
   - Extract keyframes at those specific timestamps

---

## Open Questions for Alex

1. **Volume estimate**: How many hours/month of YouTube content do you typically process?
   - < 50 hours → AssemblyAI simpler
   - > 100 hours → WhisperX more cost-effective

2. **GPU availability**: Do you have a CUDA GPU, or just Apple Silicon?
   - CUDA → WhisperX viable
   - Apple Silicon only → AssemblyAI easier

3. **Priority**: Which matters more right now?
   - Keyframes (visual context) → Phase 1, ready now
   - Diarization (speaker ID) → Phase 2, needs decision

4. **Content type**: Mostly interviews/podcasts (multiple speakers) or solo creators?
   - Multi-speaker → Diarization valuable
   - Solo → Diarization less important

---

## Files Created

| File | Purpose |
|------|---------|
| `research/README.md` | Research scope and structure |
| `research/diarization/whisperx.md` | WhisperX local diarization research |
| `research/diarization/assemblyai.md` | AssemblyAI API research |
| `research/keyframes/video-ops.md` | video-ops keyframe capabilities |
| `research/SYNTHESIS.md` | This document |

---

## Next Actions

1. **Alex decision needed**: AssemblyAI vs WhisperX vs Hybrid
2. **Quick win**: Test keyframe integration with video-download
3. **If AssemblyAI**: Get API key, test with one video
4. **If WhisperX**: Set up HuggingFace token, accept model terms

---

## Appendix: Quick Start Commands

### Test Keyframe Extraction
```bash
# Download with keyframes
cd ~/.claude/skills/video-download
python3 scripts/download_single.py "https://youtube.com/watch?v=abc123" --extract-keyframes

# Check output
ls ~/Movies/VIDEO_DOWNLOAD/*/keyframes/
cat ~/Movies/VIDEO_DOWNLOAD/*/keyframes/keyframes_index.json
```

### Test AssemblyAI
```bash
pip install assemblyai
export ASSEMBLYAI_API_KEY="your_key"

python3 << 'EOF'
import assemblyai as aai
config = aai.TranscriptionConfig(speaker_labels=True)
result = aai.Transcriber().transcribe("https://example.com/audio.mp3", config)
for u in result.utterances[:5]:
    print(f"[{u.speaker}] {u.text[:50]}...")
EOF
```

### Test WhisperX
```bash
pip install whisperx
export HF_TOKEN="your_huggingface_token"

whisperx audio.mp3 --diarize --hf_token $HF_TOKEN --output_format json
```

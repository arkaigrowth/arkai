# Keyframe Extraction: video-ops Skill Analysis

> **Researched**: 2026-01-17 | **Source**: video-ops.skill SKILL.md

---

## Overview

The `video-ops` skill already provides comprehensive keyframe extraction via `extract_keyframes.py`. This is a mature solution using OpenCV for motion detection.

---

## Capabilities

### Script: `extract_keyframes.py`

**Location:** `~/.claude/skills/video-ops/scripts/extract_keyframes.py`

**Modes:**

| Mode | Flag | Description |
|------|------|-------------|
| **Motion-based** | Default | Detects scene changes via motion detection |
| **Interval-based** | `--mode interval` | Extracts every N seconds |

### Key Flags

```bash
# Motion-based extraction (default)
python scripts/extract_keyframes.py video.mp4

# Adjust motion sensitivity (0.0-1.0, default 0.3)
python scripts/extract_keyframes.py video.mp4 --threshold 0.5

# Limit number of frames
python scripts/extract_keyframes.py video.mp4 --max-frames 50

# Interval-based (every N seconds)
python scripts/extract_keyframes.py video.mp4 --mode interval --interval 10

# Custom output directory
python scripts/extract_keyframes.py video.mp4 --output ./keyframes
```

### Motion Threshold Tuning

| Content Type | Recommended Threshold | Reasoning |
|--------------|----------------------|-----------|
| Action/Sports | 0.2 (more sensitive) | Lots of movement, want more keyframes |
| Interview/Talking Head | 0.5 (less sensitive) | Little movement, fewer keyframes needed |
| Mixed Content | 0.3 (default) | Balanced |
| Presentations/Slides | 0.4-0.5 | Capture slide transitions |

### Output Format

**Files produced:**
```
output_dir/
├── keyframe_0001.jpg      # JPEG image (OCR-friendly quality)
├── keyframe_0002.jpg
├── keyframe_0003.jpg
└── keyframes_index.json   # Metadata
```

**`keyframes_index.json` structure:**
```json
{
  "video": "video.mp4",
  "keyframes": [
    {
      "filename": "keyframe_0001.jpg",
      "frame_number": 150,
      "timestamp": 5.0,
      "motion_score": 0.42
    },
    {
      "filename": "keyframe_0002.jpg",
      "frame_number": 450,
      "timestamp": 15.0,
      "motion_score": 0.38
    }
  ]
}
```

---

## Integration with video-download Skill

The `video-download` skill already has native integration:

```bash
# Auto-extract keyframes after download
python3 scripts/download_single.py "URL" --extract-keyframes

# Interactive prompt after download (default)
python3 scripts/download_single.py "URL"
# → "Extract keyframes from this video? [Y/n/later]"

# Skip keyframe extraction (no prompt)
python3 scripts/download_single.py "URL" --no-keyframes
```

---

## Integration with arkai Pipeline

### Current arkai Flow
```
URL → fabric -y (YouTube transcript) → extract_wisdom → summary → library
```

### Proposed Enhanced Flow
```
URL → video-download
        ├── Video file (optional)
        ├── Keyframes (optional, via video-ops)
        └── YouTube captions
            ↓
      arkai ingest → fabric patterns → library
            ↓
      Library folder with:
        ├── metadata.json
        ├── fetch.md (transcript)
        ├── wisdom.md
        ├── summary.md
        └── keyframes/
            ├── keyframe_0001.jpg
            ├── keyframe_0002.jpg
            └── keyframes_index.json
```

### Integration Options

**Option A: Extend arkai ingest to call video-ops**
- Pros: Single command, integrated
- Cons: Requires Rust changes, downloads full video

**Option B: video-download as frontend, pipes to arkai**
- Pros: Leverages existing skill, flexible
- Cons: Two-step process

**Option C: Separate keyframe step (manual)**
- Pros: Simplest, no code changes
- Cons: User must remember to run both

### Recommended: Option B

```bash
# Step 1: Download with keyframes
python3 ~/.claude/skills/video-download/scripts/download_single.py "URL" --extract-keyframes

# Step 2: Ingest to arkai (already downloaded, just process)
arkai ingest "URL" --local-video ~/Movies/VIDEO_DOWNLOAD/...
```

This requires adding `--local-video` flag to arkai to process already-downloaded content.

---

## Enhanced Transcript Format

To integrate keyframes with transcripts:

```markdown
[00:00:02] Hey, what's up? So I want to talk about AI agents.
[KEYFRAME: keyframes/keyframe_0001.jpg @ 5.0s]
[00:00:08] Yeah, this is fascinating stuff. Let me show you...
[00:00:15] Right, so the key insight here is...
[KEYFRAME: keyframes/keyframe_0002.jpg @ 15.0s]
```

### Implementation Strategy

1. Extract keyframes with timestamps via video-ops
2. Parse `keyframes_index.json` for timestamp → filename mapping
3. Post-process transcript to insert `[KEYFRAME: ...]` markers at appropriate timestamps
4. Store keyframes in library folder alongside other artifacts

---

## Dependencies

**Required:**
- Python 3.7+
- ffmpeg/ffprobe
- OpenCV (`opencv-python`) - auto-installed

**No GPU required** - OpenCV motion detection is CPU-based.

---

## Summary

| Aspect | Assessment |
|--------|------------|
| **Maturity** | Production-ready, well-documented |
| **Complexity** | Low - single script, auto-installs deps |
| **Quality** | Good - OCR-friendly JPEG output |
| **Flexibility** | High - motion-based or interval-based |
| **Integration** | Already integrated with video-download |
| **Gaps** | No "speaker change" mode (requires diarization) |

### Verdict: Ready to Use

The video-ops keyframe extraction is ready for integration. The main gap is extracting keyframes at speaker changes, which requires diarization data first.

**Proposed enhancement:** After diarization is added, we can:
1. Parse diarization output for speaker change timestamps
2. Extract keyframes at those specific moments
3. This gives us "visual context at speaker transitions"

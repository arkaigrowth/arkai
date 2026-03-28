# Keyframe Phase 1 Handoff — Scene Detection + Transcript Correlation

> **Date**: 2026-03-27
> **Status**: ACCEPTED, SCOPE FROZEN
> **Next**: Do NOT build OCR/CLIP/vision. Only follow-up is Whisper JSON preservation.

---

## What Was Built

Added `--mode scene` and `--transcript` to the video-ops keyframe extractor.

**File changed:**
```
~/.claude/skills/video-ops/video-ops/scripts/extract_keyframes.py
```

**What's new:**
- `--mode scene` — Uses ffmpeg `scdet` filter for scene change detection
- `--transcript <path>` — Correlates keyframes to transcript segments
- `load_transcript_segments()` — Handles Whisper JSON (precise) or plain text (estimated)
- `correlate_keyframes_with_transcript()` — Matches timestamps, adds `transcript_context`
- Existing `--mode motion` and `--mode interval` unchanged

## Test Commands

```bash
# Recommended invocation for tutorial/library videos
cd ~/.claude/skills/video-ops/video-ops/scripts
python3 extract_keyframes.py /path/to/video.mp4 \
  --mode scene \
  --threshold 0.1 \
  --min-interval 30 \
  --transcript /path/to/transcript.txt \
  --output /path/to/keyframes/

# Verified result on "Pi CEO Agents" (40 min):
#   26 keyframes, 26/26 correlated with transcript
#   vs ~100+ with motion detection
```

**Recommended defaults for educational/library content:**
- `--mode scene --threshold 0.1 --min-interval 30`
- Lower `min-interval` (10-15) for fast-paced content with many slides
- Higher `threshold` (0.2-0.3) to reduce further if needed

## Known Fragility

**ffmpeg stderr regex parsing**: The `scd_pattern` regex parses scene change
timestamps from ffmpeg's stderr output. Format:
```
[Parsed_scdet_0 @ 0x...] lavfi.scd.score: 10.484, lavfi.scd.time: 8.441767
```

If ffmpeg changes this output format in a future version, the parser breaks silently
(produces 0 scene changes, falls back to just frame 0). Consider adding a warning
if zero scene changes are detected on a video longer than 60 seconds.

**scdet parameter gotcha**: The threshold parameter is `threshold=N` with range
0-100 (NOT `s=N`, which is a boolean for `sc_pass`). The script maps our 0.0-1.0
CLI range to ffmpeg's 0-100 range internally.

## Next Durable Step: Preserve Whisper JSON in Arkai Ingest

**One-line change** in `src/cli/mod.rs` at line 598:

```rust
// CURRENT (line 598):
"txt",

// CHANGE TO:
"all",
```

This makes Whisper produce BOTH `audio.txt` (plain text) AND `audio.json` (word-level
timestamps with start/end per segment). The JSON file should be copied to the library
content directory as `transcript.json` alongside `transcript.txt`.

**Where the JSON artifact should live:**
```
~/AI/library/youtube/Title (ID)/
  ├── transcript.txt      ← plain text (existing, for chunking + fabric patterns)
  ├── transcript.json     ← NEW: Whisper JSON with word-level timestamps
  ├── metadata.json
  ├── wisdom.md
  └── keyframes/
      └── index.json      ← keyframe index references transcript.json for correlation
```

**Why this matters**: With `transcript.json`, the `--transcript` flag gets precise
word-level timestamps (33ms accuracy at 30fps) instead of the approximate timestamps
from plain text word-count estimation. This makes keyframe-transcript correlation
frame-accurate.

**Implementation (3 lines in ingest_youtube()):**
1. Change `"txt"` to `"all"` in Whisper args
2. After transcription, also copy `audio.json` → `content_dir/transcript.json`
3. Update `discover_artifacts()` to recognize `transcript.json`

## Scope Freeze

Do NOT build in this phase:
- OCR text detection (needs pytesseract)
- CLIP visual embeddings (needs PyTorch)
- Vision model descriptions (needs LLaVA)
- Frame similarity dedup (nice optimization, not needed at 12-video scale)
- Image embedding storage in arkai store

These become relevant at 50+ videos when manual keyframe browsing stops scaling.

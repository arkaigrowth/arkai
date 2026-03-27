# Keyframe Extraction Spec — Phase 1

> **From**: arkai session (Claude Code, 2026-03-26)
> **To**: Codex
> **Scope**: Improve video-ops keyframe extraction — scene detection + transcript correlation
> **Constraint**: Zero new dependencies. Use only what's installed.

---

## Problem

The current video-ops skill (`~/.claude/skills/video-ops.skill`) uses **motion detection**
(frame-to-frame pixel difference) for keyframe extraction. This over-triggers on:
- Camera pans and zooms (every slight movement = new keyframe)
- Presenter gestures (hand waves, head turns)
- Lighting changes

A 40-min video produces 80-150 keyframes, most of which are near-identical talking-head frames.
The useful frames (slides, code, graphs, tables) get buried in noise.

## Goal

Reduce keyframe count from ~100+ to ~15-30 per video while capturing ALL visually
meaningful frames (slides, graphs, code, tables, scene transitions).

## What's Installed (Verified 2026-03-26)

| Tool | Path | Version | Status |
|---|---|---|---|
| ffmpeg | `/opt/homebrew/bin/ffmpeg` | 8.0 | Has `scdet` filter, `libtesseract` |
| ffprobe | `/opt/homebrew/bin/ffprobe` | 8.0 | Metadata extraction |
| whisper | `/opt/homebrew/bin/whisper` | installed | `--output_format json` gives word timestamps |
| NumPy | pip | installed | Available for frame analysis |
| pytesseract | — | NOT installed | Would need `pip install pytesseract` |
| OpenCV | pip | listed but import fails | Wrong Python environment |
| LLaVA/CLIP | — | NOT installed | No vision models in Ollama |

## Phase 1 Spec (Zero New Dependencies)

### 1. Add `--mode scene` to extract_keyframes

**Current**: `--mode motion` (frame diff) or `--mode interval` (fixed time)
**New**: `--mode scene` using ffmpeg's built-in `scdet` filter

**How scdet works:**
```bash
ffmpeg -i video.mp4 -vf "scdet=s=0.35" -f null - 2>&1 | grep "^\[Parsed_scdet"
# Output: [Parsed_scdet_0 @ 0x...] lavfi.scd.score: 0.847382 lavfi.scd.time: 125.458
```

The `scdet` filter detects **scene boundaries** (hard cuts, fades, dissolves) — NOT motion.
This fundamentally eliminates the pan/gesture false positive problem.

**Implementation:**
```python
def extract_keyframes_scene(video_path, threshold=0.35, min_interval=2.0, max_frames=50):
    """Extract keyframes at scene change boundaries using ffmpeg scdet."""

    # 1. Run scdet to find scene change timestamps
    cmd = [
        "ffmpeg", "-i", video_path,
        "-vf", f"scdet=s={threshold}",
        "-f", "null", "-"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)

    # 2. Parse timestamps from stderr (scdet outputs to stderr)
    timestamps = []
    for line in result.stderr.split('\n'):
        if 'lavfi.scd.time' in line:
            # Extract timestamp value
            time_str = line.split('lavfi.scd.time:')[1].strip()
            ts = float(time_str)
            # Enforce minimum interval between keyframes
            if not timestamps or (ts - timestamps[-1]) >= min_interval:
                timestamps.append(ts)

    # 3. Cap at max_frames
    timestamps = timestamps[:max_frames]

    # 4. Extract frames at those exact timestamps
    for i, ts in enumerate(timestamps):
        output_path = f"keyframe_{i:04d}.jpg"
        subprocess.run([
            "ffmpeg", "-ss", str(ts), "-i", video_path,
            "-frames:v", "1", "-q:v", "2", output_path
        ])

    return timestamps
```

**Threshold guidance (add to tuning docs):**
- Interview/talking head: 0.45–0.55 (high — only hard cuts)
- Tutorial with slides: 0.30–0.40 (medium — catch slide transitions)
- Action/sports: 0.20–0.30 (low — more scene changes)
- Default: 0.35

**Expected results:**
- 40-min tutorial with slides: ~15-25 keyframes (vs 80-150 with motion)
- 40-min interview: ~5-10 keyframes (vs 60-100 with motion)

### 2. Add `--transcript` flag for timestamp correlation

**Current**: keyframes_index.json has frame number, timestamp, motion score
**New**: add `transcript_context` field when --transcript is provided

**How it works:**
- Whisper `--output_format json` produces word-level timestamps:
  ```json
  {"segments": [{"start": 0.0, "end": 5.2, "text": "Hello and welcome..."}, ...]}
  ```
- For each keyframe at timestamp T, find the segment where `start <= T <= end`
- Include that segment's text (trimmed to ~200 chars) in the index

**Enhanced keyframes_index.json:**
```json
{
  "video": "Pi CEO Agents (TqjmTZRL31E)",
  "mode": "scene",
  "threshold": 0.35,
  "keyframes": [
    {
      "frame_index": 0,
      "timestamp": 5.2,
      "timestamp_formatted": "00:00:05",
      "scene_score": 0.847,
      "file": "keyframe_0000.jpg",
      "transcript_context": "engineers there are three massive innovations available to you that unlock high leverage multi-agent teams..."
    },
    {
      "frame_index": 1,
      "timestamp": 125.4,
      "timestamp_formatted": "00:02:05",
      "scene_score": 0.923,
      "file": "keyframe_0001.jpg",
      "transcript_context": "this is my CEO and board multi-agent team and the value lies every one of these board members..."
    }
  ]
}
```

**Implementation:**
```python
def correlate_transcript(keyframe_timestamps, transcript_json_path):
    """Match keyframe timestamps to transcript segments."""
    with open(transcript_json_path) as f:
        transcript = json.load(f)

    segments = transcript.get("segments", [])

    for ts in keyframe_timestamps:
        # Find the segment that contains this timestamp
        context = ""
        for seg in segments:
            if seg["start"] <= ts <= seg["end"]:
                context = seg["text"][:200]
                break
        yield ts, context
```

### 3. Store keyframes in library directory

When keyframe extraction runs as part of the ingest pipeline, output goes to:
```
~/AI/library/youtube/Title (ID)/
  ├── metadata.json
  ├── transcript.txt
  ├── transcript.json          ← Whisper JSON (word timestamps)
  ├── wisdom.md
  ├── keyframes/               ← NEW
  │   ├── index.json           ← keyframe metadata + transcript context
  │   ├── keyframe_0000.jpg
  │   ├── keyframe_0001.jpg
  │   └── ...
```

## Files to Modify

| File | Location | Change |
|---|---|---|
| `extract_keyframes.py` | Inside video-ops skill | Add `--mode scene` + scdet parsing |
| `extract_keyframes.py` | Inside video-ops skill | Add `--transcript <path>` flag |
| `keyframe_tuning.md` | Inside video-ops skill | Add scdet threshold guidance |
| `SKILL.md` | Inside video-ops skill | Document new mode and flag |

The video-ops skill is at `~/.claude/skills/video-ops.skill` (zipped).
To modify: extract, edit, re-zip. Or if it's a directory, edit in place.

## Acceptance Criteria

- [ ] `--mode scene` extracts 15-30 keyframes from a 40-min video (vs 80-150 with motion)
- [ ] `--transcript transcript.json` adds transcript_context to index.json
- [ ] Default threshold (0.35) works well for tutorial/slide-heavy content
- [ ] Existing `--mode motion` and `--mode interval` still work (no regressions)
- [ ] keyframes_index.json includes scene_score, timestamp, transcript_context
- [ ] Output directory structure matches the library layout above

## Verification

```bash
# 1. Extract keyframes with scene detection
video-ops extract-keyframes --input ~/AI/library/youtube/"Pi CEO Agents..."/video.mp4 \
  --mode scene --threshold 0.35

# 2. With transcript correlation
video-ops extract-keyframes --input video.mp4 \
  --mode scene --threshold 0.35 \
  --transcript transcript.json

# 3. Compare keyframe counts
# motion mode: expect 80-150
# scene mode: expect 15-30
```

## What's NOT in Phase 1

| Feature | Why Deferred | Dependency |
|---|---|---|
| OCR text detection | Needs `pytesseract` install | Phase 2 |
| Frame similarity dedup | Nice optimization, not essential | Phase 2 |
| Vision model descriptions | No LLaVA in Ollama | Phase 3 |
| CLIP visual embeddings | No PyTorch installed | Phase 3 |
| Transcript error correction via visual | Needs vision model | Phase 4 |

## Phase 2 Preview (After Phase 1 Proves Value)

1. **Install pytesseract**: `pip install pytesseract`
2. **OCR gate**: For each keyframe, run tesseract. If text detected → HIGH PRIORITY.
   If no text → LOW PRIORITY (talking head). Keep only HIGH PRIORITY + scene changes.
3. **Store OCR text** in index.json alongside transcript_context.
4. This filters ~80% of remaining noise (talking head frames with no visual info).

## Phase 3 Preview (Future)

1. **Install LLaVA**: `ollama pull llava` (~4GB)
2. For HIGH PRIORITY frames: "Describe what's on screen in detail"
3. Store descriptions → searchable via arkai
4. Cross-check descriptions against transcript for error detection

---

## Note to Codex

The video-ops skill is a Claude Code skill (zipped or directory at ~/.claude/skills/).
The Python scripts inside use subprocess to call ffmpeg.

Key constraint: do NOT download the video separately — the keyframe extraction should
work on videos that are already downloaded (the arkai ingest pipeline handles download).

For testing: the video "Pi CEO Agents. Claude 1M Context. Multi-Agent Teams. (TqjmTZRL31E)"
was just ingested and is at:
`~/AI/library/youtube/Pi CEO Agents. Claude 1M Context. Multi-Agent Teams. (TqjmTZRL31E)/`

It has transcript.txt but no video file (we only downloaded audio for transcription).
To test keyframes, you'll need to also have yt-dlp download the video:
```bash
/opt/homebrew/bin/yt-dlp -f "bestvideo[height<=720]" -o "/tmp/test-keyframes.mp4" \
  "https://youtu.be/TqjmTZRL31E"
```

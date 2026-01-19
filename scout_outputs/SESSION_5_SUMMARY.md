# Session 5 Summary: Schema Steelmanning + Research

> **Date**: 2026-01-17 | **Context Used**: ~72% | **Status**: Ready for compaction

---

## What We Accomplished

### 1. Diarization + Keyframes Research (Complete)

Created comprehensive research in `scout_outputs/research/`:

| File | Content |
|------|---------|
| `diarization/whisperx.md` | Local option - free, needs GPU, pyannote-based |
| `diarization/assemblyai.md` | Cloud API - $0.17-0.29/hr, 2.9% speaker error |
| `keyframes/video-ops.md` | Existing skill analysis - already integrated |
| `frontend/OPTIONS.md` | GUI options (Tauri + Svelte recommended) |
| `SYNTHESIS.md` | Integration recommendation |
| `SCHEMA_SPEC.md` | **v1.2 with Chad's steelman fixes** |

### 2. Schema Specification v1.2 (Chad's Steelman Applied)

**6 fixes applied:**

| # | Issue | Fix |
|---|-------|-----|
| 1 | Header in transcript_raw.md shifts offsets | No header - metadata in metadata.json |
| 2 | timestamp extraction from slice wrong | `timestamp_at_offset()` finds preceding timestamp |
| 3 | speakers.json flat object ambiguous | Structured: `{schema_version, map, notes}` |
| 4 | diarization needs provenance | Meta line in JSONL + metadata.json |
| 5 | No transcript edit workflow | Patch/view workflow + staleness detection |
| 6 | Media storage / cloud support | `path` + optional `uri` in artifacts |

**Key architectural decision:**
```
transcript_raw.md = NO HEADER, timestamped lines only (canonical grounding)
metadata.json = ALL metadata + artifact pointers
transcript.md = RENDERED VIEW (rebuildable from raw + overlays)
```

### 3. Roadmap Created

`docs/ROADMAP.md` - canonical roadmap with:
- v1.0: CLI Core (current)
- v1.1: Enhanced Transcripts (in progress)
- v1.2: Transcript Workflows
- v2.0: Vector Search
- v3.0: Frontend/GUI

### 4. Pipeline Stub Created

`scripts/render_transcript.py` - combines raw + diarization + speakers → view

---

## Key Files Created/Modified

| File | Action |
|------|--------|
| `scout_outputs/research/` | Created entire research folder structure |
| `scout_outputs/research/SCHEMA_SPEC.md` | v1.2 with Chad's fixes |
| `scout_outputs/research/frontend/OPTIONS.md` | Frontend research |
| `docs/ROADMAP.md` | Created canonical roadmap |
| `scripts/render_transcript.py` | Pipeline step stub |

---

## Current Schema (v1.2)

### Library Folder Structure
```
~/AI/library/youtube/Video Title (id)/
├── metadata.json           # ALL metadata + artifact pointers
├── video.mp4               # Raw video (optional, or URI)
├── audio.m4a               # Audio (optional, or URI)
├── transcript_raw.md       # CANONICAL - timestamped lines ONLY, NO header
├── diarization.jsonl       # Speaker segments with meta provenance
├── speakers.json           # Structured name mapping
├── transcript.md           # RENDERED VIEW (rebuildable)
├── wisdom.md               # AI insights
├── summary.md              # AI summary
└── keyframes/
    ├── keyframe_0001.jpg
    └── keyframes_index.json
```

### transcript_raw.md (NO HEADER)
```markdown
[00:00:00] Hey, what's up? So, I want to ask and
[00:00:02] answer a question that I think is really
```

### diarization.jsonl (WITH META)
```jsonl
{"type": "meta", "schema_version": 1, "tool": "whisperx", "model": "large-v2", ...}
{"type": "segment", "start": 0.0, "end": 6.2, "speaker": "SPEAKER_00", "confidence": 0.95}
```

### speakers.json (STRUCTURED)
```json
{"schema_version": 1, "map": {"SPEAKER_00": "Daniel"}, "notes": "..."}
```

### metadata.json (ARTIFACT POINTERS)
```json
{
  "schema_version": 2,
  "source": {"url": "...", "title": "...", "duration_seconds": 1698},
  "artifacts": {
    "transcript_raw": {"path": "transcript_raw.md", "hash": "sha256:...", "tool": "fabric"},
    "keyframes": {"path": "keyframes/", "index": "keyframes/keyframes_index.json", "count": 12}
  }
}
```

---

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| No header in transcript_raw.md | Byte offset stability for evidence grounding |
| Diarization as derived overlay | Evidence stability (Chad's architecture) |
| Structured speakers.json | Futureproof, machine-safe |
| Meta line in diarization.jsonl | Provenance tracking |
| Tauri + Svelte for future GUI | Matches Rust ecosystem, small binary |
| `path` + `uri` for media | Cloud storage support without breaking local |

---

## Next Session: Keyframe/Screenshot Integration

### Goal
Test keyframe extraction and explore how to display transcripts with screenshots tied to timestamps.

### Questions to Explore
1. **Display format**: How should keyframes appear in transcript view?
   ```markdown
   [00:00:05] [Daniel] This is the key insight...
   ![keyframe](keyframes/keyframe_0001.jpg)
   [00:00:10] [Daniel] And here's why it matters...
   ```

2. **Timestamp alignment**: Should keyframes snap to nearest transcript line?

3. **Storage**: Graph DB vs enhanced metadata.json for timestamp → keyframe relationships?

4. **Graph DB exploration**:
   - Neo4j (powerful, complex)
   - LanceDB (vector + graph, Rust-native)
   - SQLite + JSON (simple, works)
   - Just metadata.json (simplest)

### Suggested First Test
```bash
# 1. Download a video with keyframes
cd ~/.claude/skills/video-download
python3 scripts/download_single.py "YOUTUBE_URL" --extract-keyframes

# 2. Check keyframes output
ls ~/Movies/VIDEO_DOWNLOAD/*/keyframes/
cat ~/Movies/VIDEO_DOWNLOAD/*/keyframes/keyframes_index.json

# 3. Align with transcript timestamps
# (This is what we'll build next session)
```

---

## Copy-Paste Prompt for Next Session

```
Continuing from Session 5. We completed:
- Diarization + keyframes research (scout_outputs/research/)
- Schema v1.2 with Chad's steelman fixes (SCHEMA_SPEC.md)
- Roadmap created (docs/ROADMAP.md)

Now test keyframe/screenshot extraction:
1. Run video-download with --extract-keyframes on a sample video
2. Examine keyframes_index.json output format
3. Design transcript + keyframe display format
4. Explore storage options: Graph DB vs enhanced metadata.json

Key files:
- scout_outputs/research/SCHEMA_SPEC.md (v1.2, canonical schema)
- scout_outputs/research/keyframes/video-ops.md (keyframe capabilities)
- docs/ROADMAP.md (project roadmap)
- scripts/render_transcript.py (pipeline stub)

Questions to answer:
- How should keyframes display inline with transcript?
- Graph DB (Neo4j, LanceDB) vs simpler options?
- How do keyframes tie into evidence grounding?
```

---

## Quick Reference

| Resource | Location |
|----------|----------|
| Schema spec | `scout_outputs/research/SCHEMA_SPEC.md` |
| Roadmap | `docs/ROADMAP.md` |
| Diarization research | `scout_outputs/research/diarization/` |
| Keyframe research | `scout_outputs/research/keyframes/` |
| Frontend research | `scout_outputs/research/frontend/` |
| Pipeline stub | `scripts/render_transcript.py` |
| Canonical library | `~/AI/library/` |
| Config | `~/AI/arkai/.arkai/config.yaml` |

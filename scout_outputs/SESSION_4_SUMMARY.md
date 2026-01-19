# Session 4 Summary: Storage Consolidation + Transcript Extraction

> **Date**: 2026-01-17 | **For**: Compaction continuity | **Context**: Chad (GPT 5.2) requested transcript format confirmation

---

## Mission Accomplished

Chad requested we run fabric/arkai YouTube transcript extraction to confirm the marker format. We did that AND discovered/fixed a storage spaghetti problem.

---

## What We Did

### 1. Ran Transcript Extraction on 3 Videos

**Videos ingested from "AI stuff" playlist:**
| Video ID | Title | Duration |
|----------|-------|----------|
| `0teZqotpqT8` | Stop Competing With 400 Applicants | 25:57 |
| `28z6OjsNsUk` | What happens now? (Theo) | 49:10 |
| `kC49JCXP-zU` | 85% Of People Will be Unemployable (Shapiro) | 32:56 |

**Command used:**
```bash
arkai ingest "https://youtube.com/watch?v=VIDEO_ID" --tags "ai,playlist-sync"
```

**Pipeline flow:**
1. `fabric -y URL --transcript-with-timestamps` → raw transcript
2. `extract_wisdom` pattern → structured insights
3. `summarize` pattern → condensed summary

### 2. Confirmed Transcript Marker Format for Chad

```
[HH:MM:SS] transcript text here
```

**Example:**
```
[00:00:00] LinkedIn is dead. You know this.
[00:00:02] Everyone applying for jobs in 2025 knows
[00:00:04] this. It's not that the volume is gone.
```

**Key details:**
- Timestamps in `[HH:MM:SS]` format
- Each timestamp starts a new line
- Timestamps increment ~2-3 seconds
- Stored in `fetch.md` artifact

### 3. Discovered & Fixed Storage Spaghetti

**Problem found:**
- Content split across 3 locations
- 2 orphaned catalog entries (failed fetches with video ID as title)
- 1 video missing from catalog entirely
- Tracking file out of sync with library

**Root cause:** Two parallel ingestion flows existed:
- Old: `playlist-sync.fish` → `~/AI/fabric-arkai/youtube-wisdom/`
- New: `arkai ingest` → `~/.arkai/library/youtube/`

### 4. Consolidated to Canonical Location

**Before (spaghetti):**
```
~/.arkai/library/youtube/           → 3 videos
~/AI/fabric-arkai/library/youtube/  → 2 videos
~/AI/library/youtube/               → 1 video (hash-named)
```

**After (clean):**
```
~/AI/library/youtube/               → 5 videos (canonical!)
```

---

## Files Changed/Created

### Config Created
```
~/AI/arkai/.arkai/config.yaml       # NEW - points to ~/AI/library/
```

**Content:**
```yaml
version: "1.0"
paths:
  library: ../library
  content_types:
    youtube: youtube
    web: web
    other: other
```

### Documentation Updated
| File | Change |
|------|--------|
| `docs/AIOS_BRIEF.md` | Updated canonical paths (lines 96-132) |
| `scout_outputs/STORAGE_ARCHITECTURE.md` | Rewrote as "RESOLVED" status doc |
| `~/.claude/commands/arkai.md` | Updated library path to `~/AI/library/` |

### Scripts Updated
| File | Change |
|------|--------|
| `scripts/playlist-sync.fish` | `WISDOM_DIR` → `~/AI/library/youtube` |

### Data Fixed
| Item | Before | After |
|------|--------|-------|
| `~/.arkai/catalog.json` | 6 entries (1 orphan, 1 bad title) | 5 clean entries |
| `~/.arkai/processed_videos.txt` | 5 stale IDs | 5 correct IDs |

---

## Architecture Decision: Canonical Library Location

**Chose `~/AI/library/` over alternatives:**

| Option | Verdict | Reason |
|--------|---------|--------|
| `~/.arkai/library/` | ❌ | Hidden, easy to forget |
| `~/AI/fabric-arkai/library/` | ❌ | Couples data to code repo |
| **`~/AI/library/`** | ✅ | Visible, tool-agnostic, clean separation |

**Final architecture:**
```
~/AI/
├── library/         → DATA (content, git-trackable)
├── arkai/           → CODE (Rust tool)
└── fabric-arkai/    → CODE (scripts, patterns)

~/.arkai/            → STATE (catalog, runs, derived)
```

---

## Current State (Verified)

| System | Count | Status |
|--------|-------|--------|
| Library folders | 5 | ✅ Synced |
| Catalog entries | 5 | ✅ Synced |
| Tracking file | 5 | ✅ Synced |
| arkai config | Points to ~/AI/library/ | ✅ Working |

**Library contents:**
1. `85% Of People Will be Unemployable (kC49JCXP-zU)`
2. `Building Your Own Unified AI Assistant Using Claude Code (iKwRWwabkEc)`
3. `Run YOUR own UNCENSORED AI & Use it for Hacking (XvGeXQ7js_o)`
4. `Stop Competing With 400 Applicants... (0teZqotpqT8)`
5. `What happens now_ (28z6OjsNsUk)`

---

## Next Steps: Diarization + Keyframes

### Research Questions for Next Session

1. **Speaker Diarization** — Who is speaking?
   - **WhisperX** (local, free) — whisper + pyannote
   - **AssemblyAI** (API, ~$0.006/min) — excellent quality
   - **Deepgram** (API) — fast, good quality

2. **Keyframe Extraction** — Visual context
   - `video-download` skill has capability (explore it)
   - Options: every N seconds, at speaker changes, smart detection

3. **Enhanced Transcript Format** (proposed):
   ```
   [00:00:02] [Daniel] Hey, what's up?
   [KEYFRAME: keyframes/0002.jpg]
   [00:00:05] [Mike] Let's talk about AI agents...
   ```

4. **Integration Architecture**:
   - Option A: Extend `video-download` skill as ingestion frontend → pipes to arkai
   - Option B: New arkai adapter for diarization pre-processing
   - Option C: Separate diarization step in youtube-wisdom pipeline

### Questions to Answer

- Local (WhisperX) vs API (AssemblyAI) for diarization?
- Should `video-download` skill become the "ingestion frontend"?
- What keyframe density? (speaker change, every 30s, smart?)

---

## Prompt for Next Session

```
Continue arkai work. Session 4 accomplished:

1. Ran transcript extraction on 3 YouTube videos
2. Confirmed marker format: [HH:MM:SS] text
3. Consolidated storage to ~/AI/library/ (canonical)
4. Fixed catalog, tracking, updated all docs

CURRENT STATE:
- 5 videos in ~/AI/library/youtube/
- Config at ~/AI/arkai/.arkai/config.yaml
- Catalog synced at ~/.arkai/catalog.json

NEXT TASKS (from Chad):
1. Research speaker diarization options (WhisperX vs AssemblyAI)
2. Explore video-download skill for keyframe extraction
3. Design enhanced transcript format with speakers + keyframes
4. Determine integration architecture (skill vs adapter vs pipeline step)

Read scout_outputs/SESSION_4_SUMMARY.md for full context.
Read scout_outputs/STORAGE_ARCHITECTURE.md for resolved storage design.
```

---

## Open Questions for Chad

1. **Diarization preference**: Local (free, slower) vs API (cost, faster)?
2. **Keyframe use case**: Navigation aid? Thumbnail generation? Content summary?
3. **Priority**: Diarization first, or keyframes first, or both together?

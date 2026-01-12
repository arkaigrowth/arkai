# Session 2 Summary: arkai Architecture & Podcast Integration (2026-01-11)

## What We Accomplished This Session

### 1. Cross-Repo Documentation Architecture
- **Implemented "Single Source of Truth + Pointer" pattern**
- `arkai/docs/AIOS_BRIEF.md` = canonical architecture doc
- `fabric-arkai/.claude/CLAUDE.md` now has `@../arkai/docs/AIOS_BRIEF.md` reference
- Claude agents in fabric-arkai automatically read arkai architecture at session start
- Commits: `14bf48b` (arkai), `17719ed` (fabric-arkai)

### 2. Corrected AIOS_BRIEF.md
- **Fixed folder naming**: Changed from `<sha256-hash>` to `Title (source_id)` format
- **Added 3-layer brain architecture**: Reflexes → Learned → LLM fallback
- **Set canonical library location**: `~/AI/fabric-arkai/library/`
- **Updated catalog example** with realistic paths

### 3. Reviewed fabric-arkai Agent Summary
- Agent was ~70% correct
- Missing: 3-layer brain, accurate folder naming, storage location clarity
- Now corrected via updated AIOS_BRIEF.md

### 4. Designed `__podcast__` Action Architecture
**Files to modify:**
- `src/adapters/fabric.rs` - Add `__podcast__` action (~40 lines)
- `src/library/content.rs` - Add `ContentType::Podcast`
- `src/cli/mod.rs` - Apple Podcasts URL detection
- `pipelines/podcast-wisdom.yaml` - New pipeline

**Key insight - SOURCE vs CONTENT TYPE:**
- SOURCE (where from) = youtube.com, podcasts.apple.com, web
- CONTENT TYPE (what is it) = video, podcast, article
- MVP: File by SOURCE, query by CONTENT TYPE later (via Neo4j)

### 5. Video-Download Skill Research (CRITICAL FOR NEXT SESSION)

**Location:** `~/.claude/skills/video-download/scripts/`

**What it does:**
- Wraps yt-dlp for video/audio acquisition (1000+ platforms)
- Query metadata without downloading
- Download single videos or playlists
- Extract captions (WebVTT/SRT)
- Generate readable transcripts from Whisper JSON
- **Does NOT do keyframe extraction** - delegates to video-ops skill

**Scripts (1,892 lines total):**
| Script | Purpose |
|--------|---------|
| `_deps.py` | Shared utilities (502 lines) |
| `download_single.py` | Main download orchestrator (491 lines) |
| `download_playlist.py` | Batch with filtering (324 lines) |
| `generate_transcript_md.py` | Whisper JSON → Markdown (240 lines) |
| `query_video.py` | Metadata query (250 lines) |
| `cleanup_fragments.py` | Remove orphaned files (85 lines) |

**Output structure:**
```
~/Movies/VIDEO_DOWNLOAD/
├── {Creator} - {Title} ({video_id})/
│   ├── video_title.mp4
│   ├── video_title.en.vtt (captions)
│   ├── video_title.info.json (metadata)
│   ├── video_title.webp (thumbnail)
│   └── SUMMARY.md
```

**Integration with video-ops:**
- After download, prompts: "Extract keyframes? [Y/n/later]"
- Can auto-extract with `--extract-keyframes` flag
- Falls back to ffmpeg instructions if video-ops unavailable

**arkai integration recommended approach:**
1. Create `SkillAdapter` in arkai (calls Python scripts)
2. Create `video-intelligence` pipeline (query → download → keyframes → transcript)
3. Store in library with metadata for Neo4j indexing

---

## The Disambiguation Problem (KEY ARCHITECTURE QUESTION)

**Problem:** YouTube videos can be:
1. Regular videos → `youtube/` folder, use transcript API
2. Podcasts → semantically podcasts, but from YouTube
3. Videos with keyframes worth extracting

**Current solution (MVP):**
- SOURCE determines folder (youtube/, podcasts/, web/)
- metadata.json stores both `source` and `content_type`
- Future: Neo4j queries by content_type across folders

**Neo4j solution (future):**
```cypher
-- Find all podcasts regardless of folder
MATCH (c:Content)-[:HAS_TYPE]->(t:ContentType {name: 'podcast'})
RETURN c

-- Find content from YouTube that has keyframes
MATCH (c:Content)-[:SOURCED_FROM]->(s:Source {name: 'youtube'})
WHERE c.has_keyframes = true
RETURN c
```

---

## Pending Tasks (FOR NEXT SESSION)

### Immediate
1. **Implement `__podcast__` action** - Download + transcribe audio from Apple Podcasts
2. **Test with Koerner Office podcast** - `https://podcasts.apple.com/us/podcast/the-koerner-office-business-ideas-and-small-business/id1705154662?i=1000744432496`
3. **YouTube video test** - URL not yet provided

### User's Next Session Prompt (IMPORTANT)
> What you were seeing earlier about the [video-download skill] makes sense! however... i foresee A complication, and that is... One of my objectives is to Process YouTube videos, which may sometimes be podcasts that have a YouTube format and will sometimes show graphics and pictures, And save keyframes based on motion detection similar to the video download skill that I have in my Claude skills folder...
>
> Is there currently a keyframe extraction pattern in fabric? And if not, is that video download skill implemented? Should we implement it? i really would love to... We had discussed operationalizing the creation of new patterns, which I think would also entail converting my library of Claude skills into patterns that work with arkai, patterns and pipelines, I should say..
>
> anyway... the fact that some podcasts We'll have a provenance of YouTube. This means they will be having screenshots, despite content... and remember the bigger picture. and let's further expand and develop on how neo4j helps solve this plz...
>
> **Key questions to address:**
> - Keyframe extraction in fabric? (No, it's in video-ops skill)
> - Convert Claude skills → fabric patterns/arkai pipelines?
> - YouTube podcasts with screenshots - how to handle?
> - Neo4j for content type queries across sources?
> - Talk to chad-neo4j agent about AI OS context

### Future
- Add `--content-type` flag for user override
- Implement `SkillAdapter` in arkai for video-download/video-ops
- Vector search with LanceDB
- Neo4j graph integration

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `/Users/alexkamysz/AI/arkai/docs/AIOS_BRIEF.md` | Canonical architecture (UPDATED) |
| `/Users/alexkamysz/AI/fabric-arkai/.claude/CLAUDE.md` | Integration layer context (UPDATED) |
| `/Users/alexkamysz/AI/arkai/src/adapters/fabric.rs` | FabricAdapter with `__youtube__`, `__web__` |
| `~/.claude/skills/video-download/` | Video acquisition skill (1,892 lines Python) |
| `~/.claude/skills/video-ops/` | Video analysis (keyframes, transcription) |

---

## Architecture Diagrams

### Control Flow (Current + Proposed)
```
USER: arkai ingest "URL"
    │
    ▼
CLI (src/cli/mod.rs)
├── Detect SOURCE: youtube.com → Source::YouTube
├── Detect SOURCE: podcasts.apple.com → Source::ApplePodcasts (NEW)
├── Infer CONTENT_TYPE from source
└── Select pipeline
    │
    ▼
Pipeline Engine → Load YAML → Execute steps
    │
    ├── Step: __youtube__ → FabricAdapter → fabric -y <url>
    ├── Step: __podcast__ → FabricAdapter → yt-dlp + fabric --transcribe (NEW)
    ├── Step: __video__ → SkillAdapter → video-download skill (FUTURE)
    └── Step: extract_wisdom → FabricAdapter → fabric -p extract_wisdom
    │
    ▼
Library → Save to library/{type}/{Title (id)}/
```

### Neo4j Integration (Future)
```
┌─────────────────────────────────────────────────────────────────┐
│                        NEO4J GRAPH                               │
│                                                                  │
│   Content ──HAS_TYPE──> ContentType (video, podcast, article)   │
│      │                                                          │
│      └──SOURCED_FROM──> Source (youtube, apple_podcasts, web)   │
│      │                                                          │
│      └──HAS_KEYFRAMES──> KeyframeSet (if extracted)            │
│      │                                                          │
│      └──TAGGED_WITH──> Tag (ai, business, tutorial)            │
│                                                                  │
│   QUERY: "All podcasts with keyframes from YouTube"             │
│   CYPHER: MATCH (c:Content)-[:HAS_TYPE]->(:ContentType {name:'podcast'})
│           WHERE (c)-[:HAS_KEYFRAMES]->()                        │
│           AND (c)-[:SOURCED_FROM]->(:Source {name:'youtube'})   │
│           RETURN c                                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Commands for Quick Reference

```bash
# Podcast (manual until __podcast__ implemented)
yt-dlp -x --audio-format mp3 -o "episode.%(ext)s" "APPLE_PODCASTS_URL"
fabric --transcribe-file episode.mp3 | fabric -p extract_wisdom > wisdom.md

# YouTube (already works)
arkai ingest "YOUTUBE_URL" --tags "topic"

# Video download skill (for keyframes)
python3 ~/.claude/skills/video-download/scripts/download_single.py "URL" --extract-keyframes

# List library
arkai library
```

---

## For chad-neo4j Agent

**Context to share:**
- arkai = Rust orchestrator, event-sourced pipelines
- fabric = Go patterns, stateless AI transformations
- Files = source of truth (`library/`), indexes are derived
- **Problem**: Need to query content by semantic type (podcast) across different sources (youtube, apple_podcasts)
- **Solution needed**: Neo4j schema for Content, Source, ContentType, KeyframeSet nodes
- **Question**: How to handle YouTube videos that are semantically podcasts but may have valuable keyframes?

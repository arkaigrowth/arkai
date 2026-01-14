# Session Summary: arkai-builder (2026-01-11)

## What We Accomplished

### 1. Architecture Decisions (Consensus Validated)
- **arkai** = Rust orchestrator (state, pipelines, library)
- **fabric** = Go patterns (stateless AI transformations)
- **Files** = Source of truth (library/), indexes are derived
- **No monorepo** — keep arkai and fabric separate
- **3-layer brain** validated: Reflex → Learned → LLM fallback

### 2. Documents Created
- `docs/AIOS_BRIEF.md` — **Canonical reference for other AI sessions**
- `scout_outputs/ARCHITECTURE_CLARITY.md` — Detailed Q&A
- `scout_outputs/STORAGE_ARCHITECTURE.md` — Storage decisions
- `scout_outputs/FABRIC_LLM_EXPLAINED.md` — Fabric uses YOUR LLM

### 3. Key Clarifications
- Fabric doesn't have an LLM — it calls YOUR configured provider (OpenRouter/Claude)
- YAML pipelines are arkai's concept, not fabric's
- Library location: `/Users/alexkamysz/AI/fabric-arkai/library/` is fine
- Vector search: LanceDB + fastembed-rs recommended for future

## Pending Tasks

### Immediate (Next Session)
1. **Podcast transcript** — Koerner Office episode
   - URL: `https://podcasts.apple.com/us/podcast/the-koerner-office-business-ideas-and-small-business/id1705154662?i=1000744432496`
   - Workflow: `yt-dlp -x` → `fabric --transcribe-file` → `fabric -p extract_wisdom`

2. **YouTube transcript** — User wants to process a video

### Future
- Add `__podcast__` action to arkai (~100 lines Rust)
- Add `arkai ask "..."` for NL interface
- Vector search with LanceDB (optional)
- Graph DB integration (separate session with AIOS_BRIEF.md)

## Commands for Quick Reference

```bash
# Podcast transcript (manual workflow)
yt-dlp -x --audio-format mp3 -o "episode.%(ext)s" "PODCAST_URL"
fabric --transcribe-file episode.mp3 | fabric -p extract_wisdom > wisdom.md

# YouTube (already works)
arkai ingest "YOUTUBE_URL" --tags "topic"

# List library
arkai library
```

## Key Files
- `/Users/alexkamysz/AI/arkai/docs/AIOS_BRIEF.md` — Give to other AI sessions
- `/Users/alexkamysz/AI/fabric-arkai/library/` — Content storage location
- `~/.config/fabric/.env` — LLM config (OpenRouter, Claude Sonnet 4)

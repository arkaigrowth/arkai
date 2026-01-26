# arkai-voice-builder Session Handoff

> **Created**: 2026-01-21
> **From Session**: arkai-triage-sidecar
> **Purpose**: Starter prompt for voice capture implementation

---

## Mission
Implement Voice Capture Phase 1 for arkai: Siri/Voice Memos → arkai → Obsidian pipeline.

## Context
- arkai is a Rust event-sourced orchestrator at `/Users/alexkamysz/AI/arkai`
- Design doc is COMPLETE: `docs/ARKAI_VOICE_CAPTURE_DESIGN.md` (~800 lines)
- Voice Memos location CONFIRMED: `~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings/`
- Whisper INSTALLED: `/opt/homebrew/bin/whisper`
- Storage pattern: JSONL (not SQLite) - matches existing EventStore

## Phase 1 Deliverables (Foundation)

1. **Watcher** (`src/ingest/watcher.rs`)
   - Use `notify` crate to watch Voice Memos directory
   - Stability check: file size unchanged for 5 seconds (iCloud sync)
   - Emit AudioFileEvent with SHA256 hash (short, 12 chars)

2. **Queue** (`src/ingest/queue.rs`)
   - JSONL-based queue (mirrors EventStore pattern)
   - States: pending → processing → done | failed
   - Idempotency via content hash

3. **CLI** (`src/cli/ingest.rs` or extend existing)
   - `arkai ingest voice status` - show queue state
   - `arkai ingest voice watch --once` - process queue once

## Key Constraints
- NO SQLite (use JSONL like EventStore)
- NO LLM calls in core (enrichment is Phase 4)
- Follow existing patterns in `src/core/event_store.rs`
- Add VoiceCapture event types to `src/domain/events.rs`

## Files to Read First
1. `docs/ARKAI_VOICE_CAPTURE_DESIGN.md` - Full spec
2. `src/core/event_store.rs` - Pattern to follow
3. `src/domain/events.rs` - Event types (add new ones)
4. `Cargo.toml` - Add `notify` crate

## Success Criteria
- [ ] `arkai ingest voice status` shows queue state
- [ ] New .m4a in Voice Memos dir gets detected and queued
- [ ] Re-running doesn't duplicate (idempotency works)
- [ ] Events logged to JSONL

## Out of Scope (Later Phases)
- Whisper transcription (Phase 2)
- Obsidian deposit (Phase 3)
- LLM enrichment (Phase 4)

---

## Session Context (from arkai-triage-sidecar)

This session also produced:
- `docs/ARKAI_GMAIL_DESIGN.md` - Gmail triage spec (separate repo, Python)
- `scripts/ralph` - RALPH session memory CLI
- `.ralph/` folder structure
- `scout_outputs/REPO_TREE_2026-01-21.md` - Repo structure for Chad
- Chad's spec-kernel v3 approved (Option B for schema_version)

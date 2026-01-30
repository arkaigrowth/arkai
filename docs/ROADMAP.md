# arkai Roadmap

> **Last Updated**: 2026-01-30 | **Maintainer**: Alex + Claude + Chad

---

## Current State: v1.0 (CLI Core)

- ✅ Content ingestion (`arkai ingest`)
- ✅ Library management (`arkai library`, `arkai search`)
- ✅ Evidence system (`arkai evidence show/open/validate`)
- ✅ Catalog + event sourcing
- ✅ Fabric pattern integration
- ✅ Storage consolidated to `~/AI/library/`

---

## v1.1: Enhanced Transcripts (In Progress)

### Schema Enhancements
- [ ] `transcript_raw.md` as canonical (no header, timestamped lines only)
- [ ] `diarization.jsonl` with meta provenance
- [ ] `speakers.json` with structured schema
- [ ] `transcript.md` as rebuildable view
- [ ] `keyframes/` integration
- [ ] Enhanced `metadata.json` with artifact pointers

### Pipeline Steps
- [ ] `render_transcript` command (raw + diarization → view)
- [ ] Diarization adapter (WhisperX or AssemblyAI)
- [ ] Keyframe extraction integration (video-ops)

### Evidence CLI Enhancement
- [ ] Speaker context in `evidence show`
- [ ] Keyframe context in `evidence show`
- [ ] `timestamp_at_offset()` lookup (not slice parsing)

### Documentation
- [x] Research: diarization options
- [x] Research: keyframe options
- [x] Schema specification (SCHEMA_SPEC.md)
- [ ] Apply Chad's steelman fixes

---

## v1.2: Transcript Workflows

- [ ] `arkai speakers` CLI (manage speakers.json)
- [ ] `arkai transcript patch` (formal edit workflow)
- [ ] `arkai transcript rebuild` (regenerate view from raw + overlays)
- [ ] Stale evidence detection after transcript edits

---

## v1.3: RLM Integration (Recursive Language Model)

> Massive context analysis for files/repos that exceed context windows.
> Based on MIT's RLM paper (arxiv.org/html/2512.24601v1)

### Phase 0: MCP Integration (✅ DONE)
- [x] Fork richardwhiteii/rlm to ~/AI/rlm-fork
- [x] Wire MCP server in ~/.claude/settings.json
- [x] Configure data directory: ~/.rlm-data
- [ ] Smoke test: rlm_load, rlm_filter, rlm_exec

### Phase 1: Budget + Sandbox (MVP)
- [ ] CostTracker middleware (token counting per provider)
- [ ] BudgetManager (per-run limits: tokens, dollars, calls)
- [ ] Sandbox hardening (RLIMIT_*, env scrub, import blocklist)
- [ ] 80% warning + HITL escalation

### Phase 2: OpenRouter Integration
- [ ] Add OpenRouter provider to rlm_mcp_server.py
- [ ] Wire steelman-style two-lane routing
- [ ] Support GLM-4.7, GPT-4o-mini, Gemini Flash

### Phase 3: Evidence Integration
- [ ] evidence_candidates.jsonl output format
- [ ] Enhanced schema (artifact_sha256, chunk_id, span_type)
- [ ] arkai evidence resolve command integration

### Phase 4: HITL + Skill Definition
- [ ] Strategy approval checkpoint
- [ ] Cost estimate approval checkpoint
- [ ] Exec code approval checkpoint
- [ ] /rlm Claude Code command
- [ ] arkai tool rlm CLI contract

### Phase 5: arkai Integration
- [ ] RLM as __skill__:rlm in pipelines
- [ ] Event logging (RLMAnalysisStarted, RLMChunkProcessed)
- [ ] Scratch → Publish workflow (session → library)

---

## v1.4: Voice Capture (Siri → arkai → Obsidian)

> Design complete: `docs/ARKAI_VOICE_CAPTURE_DESIGN.md`

### Phase 1: Foundation (Delegate to arkai-voice-builder)
- [ ] Watcher (notify crate) for Voice Memos directory
- [ ] Queue manager (JSONL, matches EventStore pattern)
- [ ] CLI: `arkai ingest voice status`

### Phase 2: Transcription
- [ ] Whisper backend (already installed at /opt/homebrew/bin/whisper)
- [ ] TranscriptionBackend trait + pluggable backends
- [ ] Apple Native backend stub (future fast mode)

### Phase 3: Deposit
- [ ] Obsidian depositor (markdown generation)
- [ ] Atomic file writes (temp → rename)
- [ ] CLI: `arkai ingest voice watch --once`

### Phase 4: Enrichment (Tier 1)
- [ ] LLM sidecar for summary/tasks extraction
- [ ] Evidence-required task validation
- [ ] Security gate (path validation, limits)

---

## v1.5: Gmail Triage (Separate Repo)

> Design complete: `docs/ARKAI_GMAIL_DESIGN.md`
> Repo: `arkai-gmail` (Python, not in arkai core)

- [ ] OAuth setup + Gmail API integration
- [ ] 7-layer security architecture (Reader/Critic/Executor)
- [ ] Label taxonomy (arkai/Priority, arkai/FYI, etc.)
- [ ] Audit log (EventStore pattern in Python)

---

## v1.6: Spec Kernel (Chad's Work)

> Reference: `arkai-spec-bootstrap-v3.zip`

- [ ] Merge spec/ folder structure
- [ ] PR-1: Add schema_version to Rust Event struct
- [ ] Align EventType/StepStatus enums with spec

---

## v2.0: Clawdbot Integration

> Clawdbot = user-facing chat agent. arkai = backend spine.

### Fit Matrix

| Layer | Clawdbot | arkai |
|-------|----------|-------|
| Interface | WhatsApp/Telegram/iMessage | CLI/API |
| Memory | Persistent preferences | Event-sourced audit log |
| Execution | Direct actions | Pipeline orchestration |
| Safety | Sandboxed permissions | Reader/Critic/Executor separation |

### Integration Path
- [ ] Clawdbot triggers arkai pipelines via CLI
- [ ] arkai provides safety rails + audit trail
- [ ] Shared Obsidian vault as knowledge base
- [ ] Voice capture → clawdbot notifications

---

## v3.0: Vector Search + Semantic Layer

- [ ] LanceDB integration for semantic search
- [ ] Cross-content entity linking
- [ ] "Find similar" across library
- [ ] Store vectors in `~/.arkai/vectors.lance`

---

## v3.0: Frontend / GUI

- [ ] **Research complete**: see `scout_outputs/research/frontend/OPTIONS.md`
- [ ] **Recommended stack**: Tauri + Svelte 5
- [ ] Transcript viewer with timestamp navigation
- [ ] Keyframe display inline
- [ ] Evidence browser with validation status
- [ ] Library search + filter by speaker/tag

### Quick Win (Available Now)
- [ ] Obsidian vault pointing at `~/AI/library/`

---

## Security Hardening (Cross-Cutting)

> **Authority**: `docs/SECURITY_POSTURE.md` is the canonical security document.
> **Tracking**: Security tickets use `.ralph/memory/tickets/` with `SECURITY_` prefix.
> **Pattern**: All content processing follows Reader/Critic/Actor split.

### Phase 0: VPS Hardening (✅ DONE)
*Ticket: PHASE0_HARDEN | Status: DONE*

- [x] Create olek-admin user with sudo
- [x] Remove clawdbot from sudoers
- [x] Remove clawdbot from docker group
- [x] Create arkai-exec user with explicit permissions
- [x] MVP egress filtering (iptables allowlist)
- [x] Verify Claudia still responds after iptables

### Phase 1: Web Search Security (🔒 NEW)
*Ticket: TBD | Status: BACKLOG*

**Implemented (2026-01-30):**
- [x] Enable web_search via Perplexity Sonar (OpenRouter)
- [x] Enable web_fetch with 30k char limit
- [x] Create injection pattern blocklist (`~/clawd/security/provenance/blocklist.txt`)
- [x] Create Python sanitizer module (`~/clawd/security/provenance/sanitizer.py`)
- [x] Initialize audit log (`~/clawd/memory/web_audit.jsonl`)
- [x] Add behavioral guidelines to Claudia's SOUL.md

**Hardening Backlog:**
- [ ] **Hook Integration**: Auto-sanitize all web results via OpenClaw hooks
  - Intercept web_search/web_fetch tool results
  - Apply sanitizer.py before content reaches agent
  - Log all fetches to audit trail
- [ ] **Domain Allowlist Mode**: Restrict to trusted domains
  - Configurable allowlist (Wikipedia, official docs, etc.)
  - Toggle between open/restricted modes
  - Audit log for blocked domain attempts
- [ ] **Provenance Wrapper**: Tag external content inline
  - `⟦WEB:hash⟧...⟦/WEB:hash⟧` markers in content
  - Hash → source mapping for traceability
  - Detect if response references external content
- [ ] **Double-Check Mode**: Human approval for suspicious content
  - Flag content matching blocklist patterns (even after redaction)
  - Queue for human review before acting on flagged content
  - Threshold configuration (auto-approve low-risk, review high-risk)
- [ ] **Rate Limiting**: Prevent search abuse/reconnaissance
  - Per-session search limits
  - Cooldown between rapid searches
  - Alert on unusual search patterns

### Phase 2: Sandbox Hardening (📋 PLANNED)
*Referenced in: v1.3 RLM Integration*

- [ ] CostTracker middleware (token counting per provider)
- [ ] BudgetManager (per-run limits: tokens, dollars, calls)
- [ ] Sandbox hardening (RLIMIT_*, env scrub, import blocklist)
- [ ] 80% warning + HITL escalation

### Phase 3: Content Processing Security (📋 PLANNED)
*Referenced in: v1.4 Voice, v1.5 Gmail*

- [ ] Voice security gate (path validation, limits)
- [ ] Gmail 7-layer security architecture
- [ ] Reader/Critic/Actor separation verified for all pipelines
- [ ] Audit logging enabled (append-only JSONL)

### Phase 4: Bash Access (🚫 BLOCKED)
*Prerequisite: ALL above phases complete*

> **DO NOT ENABLE BASH FOR CLAUDIA** until:
> 1. ✅ Phase 0: VPS hardening complete
> 2. ⬜ Phase 1: Web search hardening complete
> 3. ⬜ Phase 2: Sandbox hardening complete
> 4. ⬜ Phase 3: Content processing security complete

- [ ] Firejail sandbox configuration
- [ ] Read-only mounts by default
- [ ] Network isolation (--net=none)
- [ ] Command allowlist (no arbitrary execution)
- [ ] Timeout enforcement (30s max)

---

## Future / Exploratory

- [ ] Real-time transcription (streaming diarization)
- [ ] Multi-model consensus (run multiple diarizers, merge)
- [ ] Cloud media storage (S3/R2 pointers in metadata)
- [ ] Mobile companion app (Flutter?)
- [ ] Graph DB for relationship queries

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-17 | `~/AI/library/` as canonical | Visible, tool-agnostic, separates code from data |
| 2026-01-17 | Diarization as derived overlay | Evidence stability (Chad's architecture) |
| 2026-01-17 | `transcript_raw.md` = no header | Byte offset stability |
| 2026-01-17 | Tauri + Svelte for future GUI | Matches Rust ecosystem, small binary |
| 2026-01-18 | RLM = skill/sidecar, not LLM layer | Preserves arkai's "no LLM calls" boundary (Chad's steelman) |
| 2026-01-18 | richardwhiteii/rlm as base | True REPL-over-context MCP, not map/reduce orchestration |
| 2026-01-18 | Two-lane model routing | Claude Max (subscription) + OpenRouter (paid API) pattern from steelman |
| 2026-01-18 | Chunk IDs = sha256(artifact + strategy + offsets) | Deterministic, stable, strategy-versioned |
| 2026-01-30 | Web search via Perplexity Sonar (OpenRouter) | Uses existing auth, AI-synthesized answers safer than raw HTML |
| 2026-01-30 | Security Hardening as cross-cutting section | Aggregates security items from all versions for visibility |
| 2026-01-30 | Provenance tracking for web content | Hash-based audit trail enables tracing content influence |

# arkai System Architecture

> **The unified map** for any AI or developer working on this system.
> Last updated: 2026-01-25

---

## System Overview

arkai is a multi-layer AI orchestration system spanning three environments:
- **VPS** (24/7 availability): Claudia chatbot + synced code repos
- **Mac** (development): Claude Code, MCP servers, full toolchain
- **Mobile** (capture): Voice memos, Siri shortcuts

```
                            ALEX (User)
                               |
        +----------------------+----------------------+
        |                      |                      |
        v                      v                      v
   +---------+           +-----------+          +----------+
   | Mobile  |           | Telegram  |          |   CLI    |
   | (Siri)  |           | (Claudia) |          | (arkai)  |
   +---------+           +-----------+          +----------+
        |                      |                      |
        |                      v                      |
        |               +-------------+               |
        |               |     VPS     |               |
        |               | (clawdbot)  |               |
        |               |   24/7 ON   |               |
        |               +-------------+               |
        |                      |                      |
        +----------------------+----------------------+
                               |
                               v
                    +--------------------+
                    |   Execution Layer  |
                    |     (arkai CLI)    |
                    +--------------------+
                               |
                    +--------------------+
                    | Transformation     |
                    |   (fabric)         |
                    +--------------------+
                               |
                    +--------------------+
                    |   LLM Provider     |
                    | (Claude/OpenRouter)|
                    +--------------------+
```

---

## Layer Architecture

```
+============================================================================+
|                          LAYER 1: USER INTERFACE                            |
+============================================================================+
|                                                                             |
|  +------------------+  +------------------+  +------------------+           |
|  |  iPhone/Watch    |  |    Telegram      |  |   Terminal/CLI   |           |
|  |  Siri Shortcut   |  |    (Claudia)     |  |   (arkai/fabric) |           |
|  +--------+---------+  +--------+---------+  +--------+---------+           |
|           |                     |                     |                     |
|           v                     v                     v                     |
+============================================================================+
|                          LAYER 2: UX LAYER (VPS)                            |
|                          clawdbot-vps (Hetzner CAX21)                       |
|                          IP: 100.81.12.50 (Tailscale)                       |
+============================================================================+
|                                                                             |
|  +---------------------------------------------------------------------+   |
|  |  Claudia (Clawdbot)                                                  |   |
|  |  - Claude API via gateway                                            |   |
|  |  - Tools: read, write, edit (NO bash, NO MCP)                        |   |
|  |  - Config: ~/clawd/ (SOUL.md, ARKAI.md, AGENTS.md)                   |   |
|  |  - Repos: ~/arkai/, ~/fabric-arkai/ (git-synced)                     |   |
|  +---------------------------------------------------------------------+   |
|                                                                             |
+============================================================================+
|                          LAYER 3: DEVELOPMENT (Mac)                         |
|                          alexkamysz's MacBook                               |
+============================================================================+
|                                                                             |
|  +---------------------------------------------------------------------+   |
|  |  Claude Code                                                         |   |
|  |  - Full MCP server suite (Context7, Sequential, RLM, etc.)          |   |
|  |  - SSH access to VPS                                                 |   |
|  |  - Main development environment                                      |   |
|  +---------------------------------------------------------------------+   |
|                                                                             |
|  Code Locations:                                                            |
|  - ~/AI/arkai/          - Rust CLI (this repo)                             |
|  - ~/AI/fabric-arkai/   - Custom fabric patterns                           |
|  - ~/AI/library/        - Processed content storage                        |
|  - ~/.arkai/            - Event logs, catalog, config                      |
|                                                                             |
+============================================================================+
|                          LAYER 4: EXECUTION (arkai spine)                   |
+============================================================================+
|                                                                             |
|  +-----------------+  +-----------------+  +-----------------+              |
|  |   Event Store   |  |    Catalog      |  |   VoiceQueue    |              |
|  |   (JSONL logs)  |  | (content index) |  |   (SQLite)      |              |
|  +-----------------+  +-----------------+  +-----------------+              |
|           |                   |                   |                         |
|           +-------------------+-------------------+                         |
|                               |                                             |
|                               v                                             |
|  +-----------------------------------------------------------------+       |
|  |  Orchestrator (src/core/orchestrator.rs)                         |       |
|  |  - Pipeline execution with safety limits                         |       |
|  |  - Event sourcing (resume from failure)                          |       |
|  |  - Content-addressable storage                                   |       |
|  +-----------------------------------------------------------------+       |
|                                                                             |
+============================================================================+
|                          LAYER 5: TRANSFORMATION (fabric)                   |
+============================================================================+
|                                                                             |
|  +-----------------------------------------------------------------+       |
|  |  fabric (Go binary)                                              |       |
|  |  - 246 patterns in fabric-arkai                                  |       |
|  |  - Stateless AI transformations                                  |       |
|  |  - YouTube transcript fetching (yt-dlp)                          |       |
|  +-----------------------------------------------------------------+       |
|                                                                             |
|  Custom patterns: ~/.config/fabric/patterns/                               |
|  Pattern call:    fabric -p extract_wisdom                                 |
|                                                                             |
+============================================================================+
```

---

## Data Flow: Voice Memo Pipeline

```
+--------+     +-------------+     +------------+     +-----------+
| iPhone |     | iCloud Sync |     | Voice Memo |     |  Mac FS   |
| Record | --> |   (auto)    | --> |   .m4a     | --> |  watcher  |
+--------+     +-------------+     +------------+     +-----------+
                                                            |
                                                            v
+-----------------------------------------------------------------------+
|                        arkai voice watch                               |
|                                                                        |
|  1. Watcher (notify crate) detects new .m4a in:                       |
|     ~/Library/Group Containers/group.com.apple.VoiceMemos.shared/     |
|                                                                        |
|  2. Stability check (5s delay for iCloud sync)                        |
|                                                                        |
|  3. Queue item created: SHA256 hash as ID                              |
+-----------------------------------------------------------------------+
                                   |
                                   v
+-----------------------------------------------------------------------+
|                        arkai voice process                             |
|                                                                        |
|  Option A: Telegram Route (24/7)                                       |
|  +-------------------+     +----------------+     +------------------+ |
|  | Upload via        | --> | Claudia        | --> | Transcription    | |
|  | TelegramClient    |     | (VPS, 24/7)    |     | + Classification | |
|  +-------------------+     +----------------+     +------------------+ |
|                                                           |            |
|  Option B: Local Whisper Route (Mac required)             |            |
|  +-------------------+     +----------------+              |            |
|  | Whisper CLI       | --> | transcript.md  |              |            |
|  | /opt/homebrew/bin |     | + timestamps   |              |            |
|  +-------------------+     +----------------+              |            |
|                                   |                        |            |
+-----------------------------------+------------------------+------------+
                                    |
                                    v
+-----------------------------------------------------------------------+
|                        Enrichment (optional)                           |
|                                                                        |
|  LLM sidecar extracts:                                                 |
|  - Summary                                                             |
|  - Key points                                                          |
|  - Tasks (with evidence from transcript)                               |
|  - Tags                                                                |
+-----------------------------------------------------------------------+
                                    |
                                    v
+-----------------------------------------------------------------------+
|                        Obsidian Deposit                                |
|                                                                        |
|  Output: ~/Obsidian/MyVault/Inbox/Voice/2026-01-25__vm__abc123.md     |
|                                                                        |
|  Contents:                                                             |
|  - Frontmatter (id, created, source, language, duration)              |
|  - Summary section                                                     |
|  - Full transcript                                                     |
|  - Timestamps (collapsible)                                            |
|  - Extracted tasks with evidence                                       |
+-----------------------------------------------------------------------+
```

---

## Component Registry

### Rust CLI (arkai)

| Module | File | Responsibility |
|--------|------|----------------|
| Orchestrator | `src/core/orchestrator.rs` | Pipeline execution, step coordination |
| Event Store | `src/core/event_store.rs` | Append-only event log, replay |
| Pipeline | `src/core/pipeline.rs` | YAML pipeline parsing, step definitions |
| Safety | `src/core/safety.rs` | Limits, timeouts, denylist patterns |
| Catalog | `src/library/catalog.rs` | Content index, search, deduplication |
| Evidence | `src/evidence/` | Claim extraction, span resolution, validation |
| Voice Queue | `src/ingest/queue.rs` | Voice memo processing queue |
| Voice Watcher | `src/ingest/watcher.rs` | FS watcher for Voice Memos directory |
| Telegram | `src/adapters/telegram.rs` | Bot API client for Claudia integration |
| Fabric Adapter | `src/adapters/fabric.rs` | Shell out to fabric CLI |

### Directory Structure

```
~/AI/
├── arkai/                    # This repo (Rust CLI)
│   ├── src/                  # Source code
│   ├── pipelines/            # YAML pipeline definitions
│   ├── patterns/             # Custom patterns (deprecated, use fabric-arkai)
│   ├── contracts/            # JSON schemas for data contracts
│   ├── docs/                 # Documentation
│   └── tests/                # Test files
│
├── fabric-arkai/             # Custom fabric patterns
│   ├── patterns/             # 246+ patterns
│   └── scripts/              # Helper scripts
│
├── library/                  # Processed content (SOURCE OF TRUTH)
│   ├── youtube/              # Video transcripts + wisdom
│   ├── web/                  # Article extractions
│   └── voice/                # Voice memo transcripts
│
└── clawdbot/                 # Claudia support files
    ├── Tell-Claudia.applescript
    └── tell-claudia-shortcut-config.json

~/.arkai/                     # Engine state (DERIVED)
├── config.yaml               # Global config
├── catalog.json              # Content index
└── runs/                     # Event logs
    └── <run_id>/
        └── events.jsonl

VPS (clawdbot-vps):
~/
├── clawd/                    # Claudia config
│   ├── SOUL.md               # Personality
│   ├── ARKAI.md              # System context
│   └── AGENTS.md             # Agent registry
├── arkai/                    # Cloned repo (read-only for now)
└── fabric-arkai/             # Cloned patterns
```

---

## Zone Ownership

| Zone | Primary Owner | Can Read | Can Write | Notes |
|------|---------------|----------|-----------|-------|
| `~/AI/arkai/src/` | Claude Code (Mac) | All | All | Main development |
| `~/AI/arkai/docs/` | Claude Code (Mac) | All | All | Documentation |
| `~/AI/library/` | arkai CLI | All | arkai only | Content storage |
| `~/.arkai/` | arkai CLI | All | arkai only | Engine state |
| `~/clawd/` | Claudia (VPS) | Claudia | Claudia | Personality config |
| VPS `~/arkai/` | Git sync | Claudia | Git only (L1) | Read-only clone |
| VPS `~/fabric-arkai/` | Git sync | Claudia | Git only (L1) | Read-only clone |
| Voice Memos dir | macOS/iCloud | arkai watcher | Never | Source audio |
| Obsidian vault | Depositor | All | arkai via deposit | Transcripts land here |

### Claudia Access Levels (Current: L1)

| Level | Capabilities | Status |
|-------|--------------|--------|
| **L0** | Chat only, no file access | Baseline |
| **L1** | Read arkai/fabric repos via git pull | **CURRENT** |
| **L2** | cargo check/test (no commits) | Planned |
| **L3** | Commits with review | Future |

---

## Integration Points

### Telegram Integration

```
arkai (Mac)                    Claudia (VPS)
    |                               |
    |  TelegramClient.send_audio()  |
    |------------------------------>|
    |                               |
    |      (Telegram Bot API)       |
    |                               |
    |<------------------------------|
    |    Transcription + response   |
```

**Config required** (environment variables):
```bash
TELEGRAM_BOT_TOKEN=<bot_token>
TELEGRAM_CHAT_ID=<chat_id>
```

### Fabric Integration

```
arkai                          fabric
    |                               |
    |  fabric -p extract_wisdom     |
    |------------------------------>|
    |                               |
    |    (spawns fabric process)    |
    |                               |
    |<------------------------------|
    |         stdout result         |
```

**Pattern resolution**:
1. Check `~/.config/fabric/patterns/<name>/system.md`
2. Check `~/AI/fabric-arkai/patterns/<name>/system.md`
3. Fall back to built-in fabric patterns

---

## Data Contracts

### Voice Intake Schema

**Location**: `contracts/voice_intake.schema.json`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "id": { "type": "string", "description": "SHA256(file_content)[0:12]" },
    "file_path": { "type": "string" },
    "detected_at": { "type": "string", "format": "date-time" },
    "status": { "enum": ["pending", "processing", "done", "failed"] },
    "transcript_path": { "type": "string" },
    "obsidian_path": { "type": "string" }
  }
}
```

### Content ID

All content uses deterministic IDs:
```
content_id = SHA256(canonical_url)[0:16]
```

### Event Log Format (events.jsonl)

```jsonl
{"type":"RunStarted","timestamp":"2026-01-25T10:00:00Z","run_id":"abc123"}
{"type":"StepStarted","step":"fetch","timestamp":"2026-01-25T10:00:01Z"}
{"type":"StepCompleted","step":"fetch","duration_ms":1234}
{"type":"RunCompleted","timestamp":"2026-01-25T10:00:08Z","status":"success"}
```

---

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Rust for arkai | Single binary, no runtime deps, memory safety, fast startup |
| Go for fabric | Community patterns, YouTube integration, LLM flexibility |
| Files as source of truth | Human-readable, git-trackable, portable, grep-able |
| Event sourcing | Resume from failure, full audit trail, replay capability |
| Telegram for voice | 24/7 availability, no new infrastructure, push notifications |
| VPS L1 first | Safe starting point, read-only access before commits |
| Separate repos | Clear boundaries: arkai (Rust) vs fabric-arkai (patterns) |

---

## Related Documentation

| Document | Purpose |
|----------|---------|
| [AIOS_BRIEF.md](./AIOS_BRIEF.md) | Technical deep dive on arkai internals |
| [ARKAI_VOICE_CAPTURE_DESIGN.md](./ARKAI_VOICE_CAPTURE_DESIGN.md) | Voice pipeline design |
| [ARKAI_GMAIL_DESIGN.md](./ARKAI_GMAIL_DESIGN.md) | Email triage design |
| [ROADMAP.md](./ROADMAP.md) | Development roadmap |
| [AI_OS_ARCHITECTURE.md](./AI_OS_ARCHITECTURE.md) | High-level conceptual architecture |
| [scout_outputs/ARCHITECTURE_CLARITY.md](../scout_outputs/ARCHITECTURE_CLARITY.md) | Q&A about system boundaries |

---

## Quick Reference

### Commands

```bash
# Content ingestion
arkai ingest <url> --tags "a,b"

# Library
arkai library
arkai search <query>
arkai show <id>

# Voice capture
arkai voice watch              # Daemon mode
arkai voice status             # Queue status
arkai voice process --once     # Process pending

# Evidence
arkai evidence show <id>
arkai evidence validate <content_id>

# Fabric direct
fabric -p extract_wisdom       # Single pattern
fabric -y <youtube_url>        # Transcript
```

### VPS Access

```bash
ssh clawdbot-vps              # Via Tailscale
# Or: ssh root@100.81.12.50
```

### Environment Variables

```bash
# Required for Telegram
TELEGRAM_BOT_TOKEN=...
TELEGRAM_CHAT_ID=...

# Optional
ARKAI_LIBRARY=~/AI/library    # Override library location
ARKAI_HOME=~/.arkai           # Override config location
```

---

*This document is the single source of architectural truth. Update it when the system changes.*

# arkai Project Instructions

> **Read this first.** You're working on a multi-component AI orchestration system.
> This file ensures all Claude Code sessions understand the full architecture.

---

## Quick Orientation

**arkai** = Rust CLI spine for AI workflows (orchestration, state, storage)
**Claudia** = 24/7 AI assistant on VPS (Claude API via Clawdbot gateway)
**fabric** = Go-based AI patterns (240+ prompts, stateless transformations)

```
┌─────────────────────────────────────────────────────────────┐
│  VPS (24/7)          │  Mac (Development)                   │
├──────────────────────┼──────────────────────────────────────┤
│  Claudia (Clawdbot)  │  Claude Code (you)                   │
│  ~/arkai/ (L1 read)  │  ~/AI/arkai/ (full access)           │
│  ~/fabric-arkai/     │  SSH to VPS for operations           │
│  ~/clawd/ (config)   │  MCP servers, subagents              │
└──────────────────────┴──────────────────────────────────────┘
```

---

## Essential Reading (In Order)

1. **`docs/ARCHITECTURE.md`** — Full system map, all 5 layers, zone ownership
2. **`docs/AIOS_BRIEF.md`** — Canonical AI OS architecture brief
3. **`contracts/voice_intake.schema.json`** — Agent-to-agent contract example
4. **`.ralph/memory/handoffs/`** — Previous session context (if continuing work)

---

## The VPS Layer (Claudia)

**Location:** `clawdbot-vps` (Hetzner CAX21, Helsinki)
**Access:** `ssh clawdbot-vps` or `100.81.12.50` (Tailscale)
**User:** `clawdbot`

### Claudia's Capabilities
| Can Do | Cannot Do |
|--------|-----------|
| read, write, edit files | Run bash/exec commands |
| Read ~/arkai/, ~/fabric-arkai/ | Run cargo, git push |
| Search pattern_index.json | Access MCP servers |
| Respond via Telegram | Spawn subagents |

### Claudia's Config Files
```
~/clawd/
├── SOUL.md      # Who she is (personality, rules)
├── ARKAI.md     # System map + pattern discovery
├── AGENTS.md    # Workspace behavior
├── USER.md      # Who Alex is
└── memory/      # Daily logs
```

### Syncing Code to VPS
```bash
# Push your changes
git push origin main

# Sync VPS (Claudia can then see it)
ssh clawdbot-vps "cd ~/arkai && git pull origin main"
```

---

## Storage Architecture

### Engine State (`~/.arkai/`)
```
~/.arkai/
├── config.yaml          # Configuration
├── catalog.json         # Master index
├── voice_queue.jsonl    # Voice memo queue (append-only)
└── runs/{uuid}/
    ├── events.jsonl     # Run event log
    └── artifacts/       # Step outputs
```

### Library (`~/AI/library/`)
```
~/AI/library/
├── youtube/{Title} ({id})/
│   ├── metadata.json
│   ├── summary.md
│   └── wisdom.md
├── web/
└── voice/
```

---

## Contracts (Agent Handshakes)

**Location:** `contracts/`
**Purpose:** Define data structures for agent-to-agent communication

When building features that involve:
- Voice memos → Check `contracts/voice_intake.schema.json`
- Gmail triage → Create `contracts/gmail_triage.schema.json`
- Any multi-agent flow → Define the contract FIRST

---

## Key Design Principles

1. **Event-sourced state** — All state derived from append-only JSONL logs
2. **Content hashing** — SHA256 for idempotency (12-16 char IDs)
3. **Zone ownership** — Each agent owns specific files, no overlap
4. **Contracts as handshakes** — Explicit schemas between agents

---

## Working with Claudia

### Trigger Patterns (what Claudia watches for)
- "pattern", "fabric pattern" → Pattern discovery
- Voice memos via Telegram → Transcription + classification
- Questions about the system → Reads ARKAI.md

### Updating Claudia's Knowledge
```bash
# Edit her config
ssh clawdbot-vps "nano ~/clawd/ARKAI.md"

# Or sync from repo (if you add to ~/clawd/ in git)
# Currently ~/clawd/ is git-tracked locally on VPS
```

---

## Gateway Operations

### Restart Clawdbot Gateway
```bash
ssh clawdbot-vps "screen -S clawdbot -X quit; sleep 1; screen -dmS clawdbot bash -c 'clawdbot gateway 2>&1 | tee ~/gateway.log'"
```

### Check Gateway Status
```bash
ssh clawdbot-vps "screen -ls && tail -20 ~/gateway.log"
```

---

## Current State (as of 2026-01-25)

### Working ✅
- Claudia on VPS (Telegram interface, voice transcription)
- Pattern discovery (246 patterns indexed)
- `tell-claudia` fish function
- VPS git sync (arkai + fabric-arkai)

### In Progress 🚧
- `arkai voice process` (Telegram sender) — code written, needs testing
- arkai binary not built on VPS
- Voice Memo → Claudia pipeline end-to-end

### Not Started ❌
- Gmail triage (`arkai-gmail`)
- ElevenLabs TTS
- Claudia → Claude Code task orchestration

---

## For Gmail Triage Work

If you're building Gmail triage, read:
1. `docs/ARKAI_GMAIL_DESIGN.md` — Full design doc (~1500 lines)
2. `docs/ARCHITECTURE.md` — How it fits in the system
3. Create: `contracts/gmail_triage.schema.json`

Gmail triage should:
- Use arkai's event-sourced patterns
- Follow zone ownership (arkai owns state, Claudia owns UX)
- Define contracts for classification results
- Consider: Does Claudia need to know about emails? If yes, update her ARKAI.md

---

## Handoff Protocol

Before ending a session with significant work:
1. Create `.ralph/memory/handoffs/{date}-{topic}.md`
2. Include: what was done, decisions made, files created, next steps
3. Commit and push to GitHub
4. Sync to VPS if Claudia needs to know

---

## Quick Commands

```bash
# SSH to VPS
ssh clawdbot-vps

# Sync repos to VPS
ssh clawdbot-vps "cd ~/arkai && git pull && cd ~/fabric-arkai && git pull"

# Build arkai
cargo build --release

# Test voice queue
arkai voice status

# Send to Claudia (fish function)
tell-claudia "test message"
```

---

*This file is read automatically by Claude Code. Keep it updated as the system evolves.*

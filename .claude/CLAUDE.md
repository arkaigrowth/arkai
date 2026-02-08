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

## Current State (as of 2026-01-26)

### Working ✅
- Claudia on VPS (Telegram interface, voice transcription)
- Pattern discovery (246 patterns indexed)
- `tell-claudia` fish function
- VPS git sync (arkai + fabric-arkai)
- Ticket system for multi-session coordination

### Active Tickets 🎫
- `PHASE0_HARDEN` → triage-sidecar (security hardening + email schema)
- `VOICE_INTAKE_V1` → voice-builder (voice pipeline implementation)

### In Progress 🚧
- `arkai voice process` (Telegram sender) — code written, needs testing
- arkai binary not built on VPS (Rust not installed)
- Phase 0 security hardening (BLOCKING other work)

### Not Started ❌
- Gmail triage implementation (after Phase 0)
- ElevenLabs TTS (HOLD)
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

## Worker Protocol (Multi-Session Coordination)

> **If you're a worker session**, this section defines how you operate.
> **If you're the master session**, you create tickets and validate deliverables.

### Ticket System

Active work is tracked in **`.ralph/memory/tickets/*.yaml`** files.

Each ticket has:
- **id**: Unique identifier (e.g., `PHASE0_HARDEN`)
- **worker**: Which session owns this ticket
- **status**: `BLOCKED` → `IN_PROGRESS` → `REVIEW` → `DONE`
- **context**: Files you MUST read before starting
- **task**: What to do
- **acceptance**: Machine-checkable criteria
- **deliverables**: Expected outputs
- **proofs**: You fill this in when complete

### Worker Workflow

```
1. Read .claude/CLAUDE.md (this file)
2. Read your ticket: .ralph/memory/tickets/{YOUR_TICKET}.yaml
3. Read all files in ticket's `context` section
4. Create branch: git checkout -b {branch from ticket}
5. Execute task per acceptance criteria
6. Update ticket file:
   - Fill `proofs` section with command outputs
   - Set `status: REVIEW`
   - Update `updated` timestamp
7. Commit and push branch
8. Signal master: "Ticket {ID} ready for review"
```

### Deliverable Format (Required)

Every completed ticket MUST include:

1. **Code changes** on feature branch
2. **Proofs** in ticket file (command outputs, test results)
3. **Risk notes** (what could break, rollback steps)
4. **Artifact paths** (files created/modified)

### Merge Gates (Master validates these)

Master will only merge if:
- [ ] All acceptance criteria pass
- [ ] Proofs section is complete
- [ ] Contract schemas validate (if applicable)
- [ ] Happy-path test passes
- [ ] No protected files modified without approval

### Protected Files (Worker cannot modify)

These require master approval:
- `docs/SECURITY_POSTURE.md`
- `docs/ARCHITECTURE.md`
- `.claude/CLAUDE.md`
- `contracts/*.json` (except creating new ones per ticket)

Workers can **propose** changes to protected files in their deliverables.

### Automation Notes (Future Ralph Integration)

The ticket system is designed for future automation:
- YAML format is machine-parseable
- Status enum enables state machine tracking
- Acceptance criteria can be auto-validated
- Ralph can orchestrate workers via ticket files

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

## Sibling Repo: openclaw-local

Arkai will eventually wire into OpenClaw, which is deployed from `~/AI/openclaw-local/`.

### What openclaw-local is
- Docker Compose deployment for an OpenClaw gateway (AI agent runtime)
- Managed by both **Claude Code** and **Codex** as dev tools
- The Docker container sees only: `data/`, `workspace/output/`, `workspace/input/`, `extensions/`
- Everything else (scripts, docs, agent instructions) is dev-side only

### Three-layer architecture
```
Layer 1: Dev Tools (Claude Code, Codex)
  AGENTS.md        ← canonical agent instructions (both tools read this)
  CLAUDE.md        ← Claude Code bootstrap → points to AGENTS.md
  docs/CONTEXT.md  ← live shared state and decisions
  docs/THREADS.md  ← active work tracking
  docs/ai/         ← shared research, decisions, handoffs

Layer 2: OpenClaw Config (Docker runtime)
  data/openclaw.json  ← gateway config, sessions, credentials

Layer 3: OpenClaw Agent Workspace (AI agent inside Docker)
  workspace/output/   ← agent writes here (has its own nested instructions)
  workspace/input/    ← read-only input for the agent
```

### AI Coordination Protocol (docs/ai/)
Cross-tool shared workspace, git-tracked, NOT mounted into Docker:
- `docs/ai/research/YYYY-MM-DD-topic.md` — investigation reports, subagent findings
- `docs/ai/decisions/NNNN-title.md` — Architecture Decision Records
- `docs/ai/handoffs/YYYY-MM-DD-tool-summary.md` — cross-session/cross-tool handoffs
- Full protocol: `docs/ai/README.md`

### Key conventions
- Secrets: macOS Keychain only, never in git
- Gateway: localhost-bound (`127.0.0.1:18789`)
- Apple CLI Bridge: native on host (`0.0.0.0:19789`, IP allowlisted)
- All file mutations inside Docker go through `safe_fs_*` tools (no raw fs access)

### Integration boundary
- Arkai interacts with OpenClaw via the **gateway API** (port 18789), not by modifying repo files directly
- Cross-repo decisions use **canonical + pointer** pattern (see ADR-0001 below)
- Research relevant to both repos goes in openclaw-local's `docs/ai/research/`

### Cross-repo ADR pointers
- **ADR-0001**: See `~/AI/openclaw-local/docs/ai/decisions/0001-cross-repo-coordination-pattern.md`
  (Canonical copy lives in openclaw-local. Covers: integration boundary, ADR pattern, contracts/ convention, sibling repo awareness.)
- **ADR-0002**: See `~/AI/openclaw-local/docs/ai/decisions/0002-email-triage-integration-pattern.md`
  (Option C: daemon pattern. Host-side triage_daemon.py classifies emails, writes JSON to workspace/input/emails/. Contract: `contracts/email_daemon_contract.json`)

### Sibling repo: arkai-gmail
- **`~/AI/arkai-gmail/`** — Complete email triage pipeline (pre-dates openclaw-local)
- 5-layer architecture: Ingestion → Pre-AI Gate → Reader LLM → Critic → Actor
- Reader uses Claude Sonnet, 6-category classifier, JSON-only output
- Critic: 10-rule deterministic policy gate + SQLite rate limiter
- Actor: dry-run default, whitelisted actions only
- 32+ tests, working CLI (`arkai-gmail triage`)
- **Daemon wrapper**: `scripts/triage_daemon.py` — classify-only pipeline, writes JSON per email
- **Do not rebuild this.** Evaluate for reuse when wiring email into OpenClaw.

---

*This file is read automatically by Claude Code. Keep it updated as the system evolves.*

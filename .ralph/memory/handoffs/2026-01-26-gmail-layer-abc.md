# Gmail Layer A+B+C Session Handoff

> **Created**: 2026-01-26
> **Session**: triage-sidecar (resumed)
> **PR**: https://github.com/arkaigrowth/arkai-gmail/pull/1

---

## What Was Accomplished

### Layer A: Gmail Ingestion ✅ E2E TESTED
- `auth.py`: OAuth flow with token refresh
- `ingestion.py`: Gmail client with history API (incremental sync)
- `cli.py`: Typer CLI with `auth`, `triage`, `status`, `revoke` commands
- **Tested**: Fetched 5 emails from alexkamysz@gmail.com

### Layer B: Pre-AI Gate ✅ INTEGRATED
- `gate.py`: Deterministic filtering before LLM
- Denylist domains → auto-skip
- VIP senders → fast-track to Priority
- Suspicious patterns → flag for scrutiny
- `sanitize_for_llm()` → strips HTML, base64, collapses quotes
- **Integrated** into `--dry-run` output (shows gate decisions)

### Layer C: Reader LLM ✅ IMPLEMENTED (needs testing)
- `reader.py`: Claude-based classifier
- NO TOOLS - can only output JSON
- Strict Pydantic validation
- Clear content delimiters to prevent injection
- **Not tested yet** - needs `ANTHROPIC_API_KEY`

---

## First Task Next Session

**Test Layer C with real emails:**

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
cd ~/AI/arkai-gmail
python3.11 -m arkai_gmail.cli triage --dry-run  # Current: shows gate decisions
# TODO: Add --classify flag to run Layer C
```

If classification looks wrong, fix before building D/E/F/G.

---

## Remaining Layers

| Layer | Purpose | Status |
|-------|---------|--------|
| D | Critic (validates actions) | Stub |
| E | Actor (executes on Gmail) | Stub |
| F | Memory (learned preferences) | Stub |
| G | Audit (event logging) | Stub |

---

## Key Files

```
~/AI/arkai-gmail/
├── src/arkai_gmail/
│   ├── auth.py        # Layer A: OAuth
│   ├── ingestion.py   # Layer A: Gmail client
│   ├── gate.py        # Layer B: Pre-AI filter
│   ├── reader.py      # Layer C: LLM classifier
│   ├── cli.py         # CLI with --dry-run
│   ├── critic.py      # Layer D (stub)
│   ├── actor.py       # Layer E (stub)
│   ├── memory.py      # Layer F (stub)
│   └── audit.py       # Layer G (stub)
└── config/
    └── gates.yaml     # Gate configuration
```

---

## Git Status

| Repo | Branch | Status |
|------|--------|--------|
| arkai-gmail | feat/gmail-layer-a | PR #1 open |
| arkai | main | Up to date |

---

## Commands Cheat Sheet

```bash
# Test Layer A (already working)
python3.11 -m arkai_gmail.cli auth
python3.11 -m arkai_gmail.cli triage --dry-run

# Test Layer C (next session)
export ANTHROPIC_API_KEY="sk-ant-..."
# Need to add --classify flag to CLI

# Check status
python3.11 -m arkai_gmail.cli status
```

---

## Security Architecture

```
Gmail API → Layer A → Layer B → Layer C → Layer D → Layer E
            (fetch)   (filter)  (classify) (validate) (execute)
                        ↓          ↓           ↓          ↓
                    deterministic  NO TOOLS   NO CONTENT  whitelist
                    Python code    JSON only  sees action only
```

---

*Handoff created 2026-01-26*

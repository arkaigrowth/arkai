# Gmail Pipeline Complete - Session Handoff

> **Created**: 2026-01-27
> **Session**: triage-sidecar
> **PR**: https://github.com/arkaigrowth/arkai-gmail/pull/1

---

## What Was Accomplished

### Layer A: Gmail Ingestion ✅ E2E TESTED
- `auth.py`: OAuth flow with token refresh
- `ingestion.py`: Gmail client with history API (incremental sync)
- `cli.py`: Typer CLI with `auth`, `triage`, `status`, `revoke` commands

### Layer B: Pre-AI Gate ✅ E2E TESTED
- `gate.py`: Deterministic filtering before LLM
- Denylist domains → auto-skip
- VIP senders → fast-track
- Suspicious patterns → flag
- `sanitize_for_llm()` → strips HTML, base64, collapses quotes

### Layer C: Reader LLM ✅ E2E TESTED
- `reader.py`: Claude-based classifier
- NO TOOLS - JSON output only
- Strict Pydantic validation
- **Tested**: Zocdoc→NEWSLETTER, Temu→SPAM_ISH, Robinhood→RECEIPT

### Layer D: Critic ✅ 13 UNIT TESTS
- `critic/rules.py`: Typed Rule dataclass with callable checks
- `critic/rate_limiter.py`: SQLite-backed persistent rate limits
- 7 rules: forbidden actions, newsletter+forward, spam+action, low confidence, etc.
- Decision: APPROVE / HUMAN_REVIEW / BLOCK

### Layer E: Actor ✅ 14 UNIT TESTS
- `actor.py`: Gmail API executor
- `dry_run=True` by default (SAFE)
- Only executes APPROVE verdicts
- Forward requires explicit `allow_forward=True`

### Layer F: Memory ✅ STUB
- `memory.py`: Pass-through stub
- `get_preferences()` → empty dict
- `record_feedback()` → no-op

### Layer G: Audit ✅ MINIMAL
- `audit.py`: JSONL logger
- Logs: timestamp, email_id, category, verdict, actions
- Path: `~/.arkai-gmail/audit.jsonl`

---

## First Task Next Session

**E2E test the full pipeline:**

```bash
cd ~/AI/arkai-gmail
export ANTHROPIC_API_KEY="sk-ant-..."

# Reset history to get fresh emails
rm ~/.arkai-gmail/history_id.txt

# Run full pipeline (actually executes!)
python3.11 -m arkai_gmail.cli triage --execute --limit 3

# Check audit log
cat ~/.arkai-gmail/audit.jsonl | jq .
```

---

## Key Files

```
~/AI/arkai-gmail/
├── src/arkai_gmail/
│   ├── auth.py           # Layer A: OAuth
│   ├── ingestion.py      # Layer A: Gmail client
│   ├── gate.py           # Layer B: Pre-AI filter
│   ├── reader.py         # Layer C: LLM classifier
│   ├── critic/           # Layer D: Policy gate
│   │   ├── __init__.py   #   Critic class
│   │   ├── rules.py      #   RuleEngine + typed rules
│   │   └── rate_limiter.py  # SQLite rate limits
│   ├── actor.py          # Layer E: Gmail executor
│   ├── memory.py         # Layer F: Stub
│   ├── audit.py          # Layer G: JSONL logger
│   ├── cli.py            # CLI with --dry-run/--execute
│   └── models.py         # All Pydantic models
├── tests/
│   ├── test_critic.py    # 13 tests
│   └── test_actor.py     # 14 tests
└── config/
    └── gates.yaml        # Gate configuration
```

---

## Git Status

| Repo | Branch | Status |
|------|--------|--------|
| arkai-gmail | feat/gmail-layer-a | PR #1 ready for merge |
| arkai | main | Ticket GMAIL_CORE_PIPELINE → REVIEW |

---

## Commands Cheat Sheet

```bash
# Navigate
cd ~/AI/arkai-gmail

# Set API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Preview mode (safe - no execution)
python3.11 -m arkai_gmail.cli triage --dry-run --limit 5

# Preview with classification
python3.11 -m arkai_gmail.cli triage --dry-run --classify --limit 3

# EXECUTE mode (actually processes emails!)
python3.11 -m arkai_gmail.cli triage --execute --limit 3

# Run all tests
python3.11 -m pytest tests/ -v

# Check auth status
python3.11 -m arkai_gmail.cli status

# Re-authenticate
python3.11 -m arkai_gmail.cli auth --force
```

---

## Security Architecture

```
Gmail API → Layer A → Layer B → Layer C → Layer D → Layer E → Layer G
            (fetch)   (filter)  (classify) (validate) (execute) (audit)
                        ↓          ↓           ↓          ↓
                    deterministic  NO TOOLS   NO CONTENT  whitelist
                    Python code    JSON only  sees action  dry_run
                                              only         default
```

---

## Config Locations

```
~/.arkai-gmail/
├── credentials.json     # OAuth creds (from Google Cloud)
├── token.json           # OAuth token (auto-generated)
├── history_id.txt       # Gmail sync position
├── rate_limits.db       # SQLite rate limit counters
└── audit.jsonl          # Triage audit log
```

---

## Known Limitations

1. **Layer F (Memory)** - Stub only, no learned preferences
2. **FORWARD** - Requires `allow_forward=True`, not fully implemented
3. **CREATE_DRAFT** - Returns SKIPPED (not implemented)
4. **SNOOZE** - Returns SKIPPED (no native Gmail API)
5. **Provider abstraction** - Direct Anthropic only

---

## Test Results

```
27 tests passing:
- test_critic.py: 13 tests (RuleEngine, RateLimiter, Critic)
- test_actor.py: 14 tests (DryRun, Verdicts, Actions, Security)
```

---

*Handoff created 2026-01-27*

# triage-sidecar Session Handoff

> **Created**: 2026-01-26
> **Session**: triage-sidecar
> **Purpose**: Phase 0 security hardening + Gmail scaffold

---

## What Was Accomplished

### 1. PHASE0_HARDEN ✅ MERGED
VPS security hardening completed:
- Created `olek-admin` user with sudo (backup admin)
- **Removed clawdbot from sudoers** (no longer root equivalent)
- **Removed clawdbot from docker group** (no longer root equivalent)
- Created `arkai-exec` user for sandboxed execution
- Applied MVP egress filtering (monitoring mode, logs unmatched traffic)
- Verified Claudia still responds after iptables changes

**New admin access**: `ssh olek-admin@clawdbot-vps`

### 2. Email Triage Schema ✅
Created `contracts/email_triage.schema.json` with:
- `EmailTriageItem` - Queue item for triage pipeline
- `TriageClassification` - Reader LLM structured output
- `ProposedAction` / `ExecutedAction` - Action tracking
- `EmailEvent` - Audit trail events
- `CriticPolicy` - Policy configuration

### 3. GMAIL_SCAFFOLD_V1 ✅ REVIEW
Created `~/AI/arkai-gmail/` repo with full structure:
```
arkai-gmail/
├── README.md (architecture + security model)
├── pyproject.toml
├── config/{gates,labels,risk_levels}.yaml
├── src/arkai_gmail/
│   ├── models.py      # Pydantic models
│   ├── ingestion.py   # Layer A stub
│   ├── gate.py        # Layer B stub
│   ├── reader.py      # Layer C stub (NO Gmail imports!)
│   ├── critic.py      # Layer D stub
│   ├── actor.py       # Layer E stub (HAS Gmail imports)
│   ├── memory.py      # Layer F stub
│   └── audit.py       # Layer G stub
└── tests/
```

**Security enforced in structure**: reader.py cannot import Gmail API

---

## Next Ticket: GMAIL_IMPL_LAYER_A

**Location**: `.ralph/memory/tickets/GMAIL_IMPL_LAYER_A.yaml`

**Scope**:
1. OAuth setup (`arkai-gmail auth`)
2. Gmail ingestion (incremental sync via historyId)
3. Dry run command (`arkai-gmail triage --dry-run`)

**Prerequisites** (user does manually):
1. Create Google Cloud project
2. Enable Gmail API
3. Create OAuth credentials (Desktop app)
4. Download credentials.json to `~/.arkai-gmail/`

---

## Key Files to Read

1. `docs/ARKAI_GMAIL_DESIGN.md` - Full 1500-line design doc
2. `docs/SECURITY_POSTURE.md` - Security rules + Reader/Actor split
3. `contracts/email_triage.schema.json` - Data contract
4. `~/AI/arkai-gmail/src/arkai_gmail/models.py` - Pydantic models

---

## Security Architecture Summary

```
Reader (LLM) → NO TOOLS, JSON only → Critic validates → Actor executes
     ↑                                      ↑                ↑
  Sees content                    Sees action+metadata    Gmail API
  CANNOT execute                  NEVER sees content      Whitelisted only
```

**Blocked forever**: send, delete, forward

---

## Decisions Made

1. **Gmail repo location**: `~/AI/arkai-gmail/` (separate repo, not monorepo)
2. **Storage**: JSONL for audit, SQLite for preferences
3. **Egress filtering**: Monitoring mode (logs but doesn't DROP yet)
4. **Reader security**: Enforced by not importing Gmail in reader.py

---

## Commands Cheat Sheet

```bash
# SSH to VPS as admin
ssh olek-admin@clawdbot-vps

# Check clawdbot privileges (should show NO sudo/docker)
ssh clawdbot-vps "groups clawdbot && sudo -l -U clawdbot"

# Work on arkai-gmail
cd ~/AI/arkai-gmail

# Check egress logs
ssh olek-admin@clawdbot-vps "sudo journalctl -k | grep EGRESS"
```

---

*Handoff created by triage-sidecar session, 2026-01-26*

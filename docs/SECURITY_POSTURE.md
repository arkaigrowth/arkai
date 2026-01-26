# Security Posture

> **Non-negotiable security requirements for the arkai ecosystem.**
> All Claude sessions and agents MUST read and follow this document.

---

## Current State (⚠️ NEEDS HARDENING)

| Issue | Risk | Status |
|-------|------|--------|
| `clawdbot` has `sudo NOPASSWD: ALL` | 🔴 Root equivalent | **FIX REQUIRED** |
| `clawdbot` in `docker` group | 🔴 Root equivalent | **FIX REQUIRED** |
| Telegram bot token in code | ⚠️ Medium | Acceptable for now |
| No egress filtering | ⚠️ Medium | Should lock down |

---

## Phase 0: Hardening (PREREQUISITE)

**Must complete before Gmail triage goes live.**

### 1. Remove clawdbot Privileges

```bash
# SSH to VPS as root or another sudo user
ssh clawdbot-vps

# Remove from sudoers
sudo visudo
# Delete line: clawdbot ALL=(ALL) NOPASSWD: ALL

# Remove from docker group
sudo gpasswd -d clawdbot docker

# Verify
groups clawdbot  # Should NOT show docker
sudo -l -U clawdbot  # Should show nothing or limited
```

### 2. Create Execution User

```bash
# Create non-privileged user for arkai execution
sudo useradd -m -s /bin/bash arkai-exec

# NO sudo access
# NO docker access
# Can only run specific binaries
```

### 3. Egress Lockdown (Future)

```bash
# Allow only:
# - GitHub (for git pull)
# - api.telegram.org (for Claudia)
# - api.anthropic.com (for LLM calls)
# - api.openai.com (for Whisper)
# - accounts.google.com, gmail.googleapis.com (for Gmail API)

# Block everything else
```

---

## Reader/Actor/Critic Split (CORE SECURITY PATTERN)

**This pattern applies to ALL content processing: voice, email, web.**

```
┌─────────────────────────────────────────────────────────────────┐
│  READER (LLM)                                                    │
│  - Sees raw content (email body, transcript, etc.)              │
│  - Has NO tools (cannot execute anything)                       │
│  - Outputs ONLY structured JSON                                 │
│  - Prompt injection attempts are contained here                 │
└─────────────────────────────┬───────────────────────────────────┘
                              │ JSON output
┌─────────────────────────────▼───────────────────────────────────┐
│  CRITIC (Code)                                                   │
│  - Validates JSON schema                                        │
│  - Checks for policy violations                                 │
│  - Blocks: "forward", "send", "external links", "credentials"   │
│  - Rejects malformed or suspicious output                       │
└─────────────────────────────┬───────────────────────────────────┘
                              │ Validated action
┌─────────────────────────────▼───────────────────────────────────┐
│  ACTOR (Code)                                                    │
│  - Executes ONLY approved actions                               │
│  - Gmail: Create drafts only, apply labels, archive             │
│  - Voice: Write to Obsidian, update memory                      │
│  - CANNOT send, delete, or modify original content              │
└─────────────────────────────────────────────────────────────────┘
```

### Why This Works

1. **Prompt injection is contained** — Even if malicious content tricks the Reader, it can only output JSON. No tool access.

2. **Critic is deterministic** — Code-based validation catches policy violations. No LLM judgment.

3. **Actor has limited blast radius** — Can only do pre-approved safe actions (drafts, labels).

---

## Gmail-Specific Security

### Allowed Actions (Actor)
- ✅ Create DRAFT (never send directly)
- ✅ Apply labels
- ✅ Archive (move to archive)
- ✅ Mark read/unread

### Blocked Actions (Critic rejects)
- ❌ Send email
- ❌ Delete email
- ❌ Forward email
- ❌ Any action with external URLs in body
- ❌ Any action mentioning credentials/passwords

### Data Handling
- Email bodies: Encrypted at rest
- Retention: 7-14 days max
- Raw bodies: Reader sees them, Claudia does NOT (unless explicitly requested)
- Claudia sees: Metadata + summary + action recommendations

---

## Voice-Specific Security

### Allowed Actions (Actor)
- ✅ Write to Obsidian inbox
- ✅ Update Claudia's daily memory
- ✅ Create task in arkai queue

### Blocked Actions
- ❌ Execute commands mentioned in voice memo
- ❌ Send messages on behalf of user
- ❌ Access external services based on voice content

### Intent Classification
```json
{
  "intent": "NOTE | TASK | IDEA | QUESTION | COMMAND",
  "confidence": 0.0-1.0,
  "requires_confirmation": true  // Always true for COMMAND
}
```

---

## Claudia-Specific Rules

### What Claudia CAN Do
- Read files (arkai repos, her config)
- Write to her memory/workspace
- Respond via Telegram
- Search pattern index

### What Claudia CANNOT Do
- Run bash/exec commands
- Access MCP servers
- Send emails directly
- Execute voice commands without confirmation

### Claudia's Bash (If Ever Enabled)

**ONLY via sandboxed execution:**
```bash
firejail --private --net=none --timeout=30 bash -c "command"
```

Or Docker ephemeral container:
```bash
docker run --rm --network none --read-only alpine sh -c "command"
```

---

## Audit Trail Requirements

All actions must be logged to append-only JSONL:

```json
{
  "timestamp": "ISO8601",
  "event_type": "email_triaged | voice_classified | action_executed",
  "actor": "reader | critic | actor | claudia",
  "input_hash": "sha256",
  "output": { ... },
  "approval_status": "pending | approved | rejected",
  "approved_by": "user | auto"
}
```

---

## Security Checklist (Pre-Launch)

- [ ] clawdbot removed from sudoers
- [ ] clawdbot removed from docker group
- [ ] arkai-exec user created
- [ ] Gmail OAuth token stored securely (not in code)
- [ ] Email body encryption implemented
- [ ] Retention policy enforced
- [ ] Egress allowlist configured
- [ ] Audit logging enabled
- [ ] Reader/Actor split verified
- [ ] Critic policy rules tested

---

*This document is authoritative. If any code or agent violates these rules, it's a bug.*

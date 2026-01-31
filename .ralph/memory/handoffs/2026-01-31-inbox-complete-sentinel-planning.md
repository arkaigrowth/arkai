# Handoff: Inbox Complete + Sentinel Security Planning

**Date:** 2026-01-31
**Session:** Inbox MVP complete, pivoting to Sentinel security hardening
**Status:** Inbox DONE, Sentinel in planning phase

---

## PART 1: INBOX SYSTEM (COMPLETE)

### Summary

All 5 phases of Chad's recommended order completed in one session:

| Phase | Command | Commit | Status |
|-------|---------|--------|--------|
| 1. Fixtures | `pipeline` | `be4db6d` | ✅ |
| 2. Thin CLI | `pipeline` | `be4db6d` | ✅ |
| 3. Gmail Live | `gmail` | `1f304ae` | ✅ |
| 4. Interactive | `triage` | `9e90b92` | ✅ |
| 5. Obsidian | `digest` | `10bc9bb` | ✅ |

### Commands Available

```bash
# Test with fixtures
arkai-inbox pipeline -d tests/fixtures/linkedin_spoof/

# Live Gmail scan (non-interactive)
arkai-inbox gmail -q "from:linkedin.com" -n 10
arkai-inbox gmail -q "from:linkedin.com newer_than:7d" -n 20

# Interactive triage with copy/open/skip
arkai-inbox triage -q "from:linkedin.com" -n 5

# Generate Obsidian digest from audit log
arkai-inbox digest --vault ~/Obsidian/vault-sandbox
```

### Files Created

```
services/inbox/
├── src/arkai_inbox/
│   ├── __init__.py
│   ├── normalize.py          # HTML→text, unicode normalization
│   ├── quarantine.py         # 3-tier sender evaluation, hard rules
│   ├── url_extractor.py      # BeautifulSoup link parsing, phishing detection
│   ├── models.py             # EmailRecord, CriticEvidenceBundle, AuditEvent
│   ├── audit.py              # Append-only JSONL logging
│   ├── ingestion/
│   │   ├── __init__.py
│   │   ├── gmail.py          # Gmail API JSON → EmailRecord parser
│   │   └── gmail_client.py   # Live Gmail API client (reuses arkai-gmail tokens)
│   └── cli/
│       ├── triage.py         # Main CLI: pipeline, gmail, triage, digest
│       └── obsidian.py       # Digest generator
└── tests/
    ├── test_normalize.py     # 39 tests
    ├── test_quarantine.py    # 34 tests
    ├── test_url_extractor.py # 44 tests
    ├── test_ingestion.py     # 46 tests
    └── fixtures/
        ├── linkedin_real/    # 5 real Gmail API exports
        │   ├── msg1.json ... msg5.json
        └── linkedin_spoof/   # 2 synthetic phishing examples
            ├── spoof_link_mismatch.json
            └── spoof_reply_to.json
```

### Test Coverage

- **163 total tests** passing in 0.09s
- All Pre-Gate modules at ~100% coverage

### Config File Created

```yaml
# ~/.arkai/config.yaml
obsidian:
  enabled: true
  vault_path: ~/Obsidian/vault-sandbox
  inbox_root: 00-Inbox/Digest

linkedin:
  exact_pass:
    - notifications-noreply@linkedin.com
    - messages-noreply@linkedin.com
    - invitations@linkedin.com
    - jobs-noreply@linkedin.com
    - jobalerts-noreply@linkedin.com
  domain_review: "@linkedin.com"
```

**Note:** Config file exists but quarantine.py still uses hardcoded values. Wiring up config → code is a future task.

### Quarantine Rules (Hardcoded in quarantine.py)

```python
# Valid LinkedIn senders (PASS tier)
LINKEDIN_VALID_SENDERS = [
    "notifications-noreply@linkedin.com",
    "messages-noreply@linkedin.com",
    "invitations@linkedin.com",
    "jobs-noreply@linkedin.com",
    "jobalerts-noreply@linkedin.com",
]

# Approved link domains
LINKEDIN_APPROVED_DOMAINS = {"linkedin.com", "www.linkedin.com"}
LINKEDIN_SUSPICIOUS_DOMAINS = {"lnkd.in"}

# Expected third-party domains (app store links in footers)
EXPECTED_THIRD_PARTY_DOMAINS = {
    "play.google.com",
    "itunes.apple.com",
    "apps.apple.com",
    "apps.microsoft.com",
    "support.apple.com",
    "support.google.com",
}
```

### Hard Quarantine Rules

| Rule | Trigger | Severity |
|------|---------|----------|
| `sender_wrong_domain` | From not @linkedin.com | QUARANTINE |
| `sender_not_in_exact_allowlist` | @linkedin.com but unknown sender | REVIEW |
| `reply_to_mismatch` | Reply-To ≠ From | QUARANTINE |
| `deep_link_wrong_domain` | Link to unapproved domain | QUARANTINE |
| `link_text_href_mismatch` | Visible text domain ≠ href domain | QUARANTINE |

### Why App Store Links Were False Positives

LinkedIn emails contain "Download the app" links in footers:
- `play.google.com/store/apps/details?id=com.linkedin.android`
- `itunes.apple.com/us/app/linkedin/id288429040`
- `apps.microsoft.com/store/detail/...`

These triggered `deep_link_wrong_domain` because the rule checked ALL links, not just suspicious ones. Fixed by adding `EXPECTED_THIRD_PARTY_DOMAINS` allowlist.

### Audit Log Location

```
~/.arkai/runs/inbox-{timestamp}/events.jsonl
```

Each run creates a new directory. Events are append-only JSONL:

```json
{"event_id": "uuid", "timestamp": "ISO8601", "stage": "pre_gate", "message_id": "...", "channel": "linkedin", "quarantine_tier": "PASS", "quarantine_reasons": [], "link_domains": ["linkedin.com"]}
```

### arkai-gmail Fix

Fixed `Console.print(err=True)` → `Console(file=sys.stderr).print()` in export command.
Commit: `6072f92` on `feat/gmail-layer-a` branch.

---

## PART 2: SENTINEL SECURITY PLANNING

### Context: What is Sentinel?

"Project Sentinel" = a secure XO (Executive Officer) agent that:
1. Triages comms (email, Telegram) - READ ONLY
2. Prepares code changes - with approval gates
3. Resists prompt injection and model failures
4. Logs everything for audit

### Chad's 4-Step Hardening Plan

| Step | What | Effort | Status |
|------|------|--------|--------|
| 1 | Fix workspace confusion | ✅ Done | Symlink exists: `~/clawd/arkai -> ~/arkai/` |
| 2 | Enforce tool verification | 5 min | Add to SOUL.md |
| 3 | Split agents (triage/operator) | Medium | Needs architecture decision |
| 4 | Add audit JSONL | Medium | Pattern exists from inbox |

### Current VPS Architecture (Recon Results)

```
VPS: clawdbot-vps (100.81.12.50 via Tailscale)
User: clawdbot
Agent: Claudia (running via clawdbot gateway)

~/.clawdbot/
├── agents/main/agent/
│   ├── auth-profiles.json    # OpenRouter API key
│   └── models.json           # Kimi K2.5 config
├── clawdbot.json             # Global config
└── credentials/              # API keys

~/clawd/                      # Claudia's workspace
├── SOUL.md                   # Personality + rules (11KB)
├── TOOLS.md                  # Environment notes
├── ARKAI.md                  # System map
├── AGENTS.md                 # Workspace behavior
├── arkai -> ~/arkai/         # Symlink to git repo ✓
├── memory/                   # Daily logs
├── drafts/                   # Draft messages
└── security/                 # (empty, for audit logs)

~/arkai/                      # Git repo (synced from GitHub)
```

### clawdbot Config Structure

```json
// ~/.clawdbot/clawdbot.json (partial)
{
  "auth": {
    "profiles": {
      "anthropic:clawd-claude-max": {"provider": "anthropic"},
      "openrouter:arkai": {"provider": "openrouter"}
    }
  },
  "agents": {
    "defaults": {
      "model": {
        "primary": "openrouter/moonshotai/kimi-k2-0905",
        "fallbacks": ["openrouter/z-ai/glm-4.7"]
      }
    }
  }
}
```

### Security Layers Already In Place (via clawdbot-ansible)

| Layer | Implementation | Status |
|-------|----------------|--------|
| Network isolation | UFW + Tailscale-only | ✅ |
| Docker hardening | DOCKER-USER chain | ✅ |
| Localhost binding | 127.0.0.1 only | ✅ |
| Non-root | clawdbot user | ✅ |
| Tool firewall | **MISSING** | ❌ |
| Two-tier agents | **MISSING** | ❌ |
| Audit logging | **MISSING** | ❌ |
| Approval UI | **MISSING** | ❌ |

### Proposed SOUL.md Additions (Not Yet Applied)

```markdown
## HARD RULES (NON-NEGOTIABLE)

1. **NEVER claim a file/directory exists without tool verification.**
   - WRONG: "The config is at ~/arkai/config.yaml"
   - RIGHT: Use `cat ~/arkai/config.yaml` first, then quote output

2. **Default mode is TRIAGE (read-only).**
   - Allowed: read, search, grep, glob, web_fetch
   - FORBIDDEN: exec, write, edit (unless /operator invoked)

3. **Operator mode requires explicit command.**
   - User must say "/operator" to unlock write/exec
   - Announce mode change: "Switching to OPERATOR mode"
```

### Implementation Options (Not Yet Decided)

**Option A: Prompt-Only (Lowest Effort)**
- Update SOUL.md with strict rules
- Rely on model following instructions
- Risk: Model can ignore rules under prompt injection

**Option B: Wrapper/Middleware (Medium Effort)**
- TypeScript wrapper around clawdbot tool calls
- Intercept and validate before execution
- Add audit logging at wrapper layer

**Option C: Fork clawdbot (Highest Effort)**
- Modify clawdbot source to add tool firewall
- Add role-based tool permissions
- Full control but maintenance burden

### Sentinel Full Spec (From Original Prompt)

The user provided a detailed spec for "Project Sentinel" with:

1. **Two-tier agents:**
   - Triage: READ/EXTRACT only (no exec/write/edit)
   - Operator: exec/write/edit but constrained by policy firewall

2. **Policy Firewall requirements:**
   - Allowlist of binaries (/usr/bin/git, /usr/bin/ls, etc.)
   - Allowlist of repo roots (/home/.../clawd, /home/.../arkai)
   - Denylist of sensitive paths (~/.ssh, ~/.aws, /etc, /proc)
   - "ask-on-miss" mode for operator only

3. **Audit requirements:**
   - Every tool call logged with timestamp + hash
   - Append-only (ideally read-only filesystem)
   - Injection detection via log analysis

4. **Approval UI requirements:**
   - NiceGUI, mobile-first
   - Comms queue (draft replies)
   - Code queue (diffs, test results)
   - Emergency stop button

5. **Secrets handling:**
   - Never stored in agent containers
   - Injected only at action time
   - Revoked on completion

### Language Choice Discussion

- clawdbot is TypeScript/Node.js based
- Rust would be overkill for wrapper/middleware
- TypeScript matches existing stack
- Python could work for standalone sentinel service

### Integration with arkai-inbox

The inbox system we just built could serve as Sentinel's triage data source:

```
Gmail → arkai-inbox Pre-Gate → CriticEvidenceBundle → Sentinel Triage Agent
                                                            ↓
                                                    Approval UI
                                                            ↓
                                                    Sentinel Operator Agent
```

---

## PART 3: OPEN QUESTIONS FOR CHAD

1. **Prompt-only vs Middleware vs Fork?**
   - Prompt-only is fastest but least secure
   - Middleware requires understanding clawdbot internals
   - Fork is most control but highest maintenance

2. **Where should audit logs live?**
   - `~/clawd/security/audit.jsonl`?
   - `~/.arkai/runs/`? (matches inbox pattern)
   - Separate read-only mount?

3. **How to enforce tool restrictions?**
   - clawdbot appears to use agent profiles but unclear if tool permissions are configurable
   - May need to inspect clawdbot source or ask OpenClaw maintainers

4. **Integration with inbox pipeline?**
   - Should Sentinel consume arkai-inbox output?
   - Or should inbox be folded into Sentinel?

5. **Model choice for triage vs operator?**
   - Currently using Kimi K2.5 via OpenRouter
   - Should triage use cheaper/faster model?
   - Should operator use Claude for reliability?

---

## PART 4: SESSION COMMITS

### arkai repo (main branch)

| Commit | Description |
|--------|-------------|
| `be4db6d` | feat(inbox): add Gmail ingestion + thin CLI pipeline slice |
| `6072c36` | feat(inbox): add 5 real LinkedIn fixtures + fix quarantine rules |
| `1f304ae` | feat(inbox): add Gmail live ingestion command |
| `9e90b92` | feat(inbox): add interactive triage command |
| `10bc9bb` | feat(inbox): add Obsidian digest generator |
| `68b3c86` | docs: mark config.yaml task complete |

### arkai-gmail repo (feat/gmail-layer-a branch)

| Commit | Description |
|--------|-------------|
| `6072f92` | fix(cli): use stderr console for export command logging |

---

## PART 5: REMAINING TASKS

### Inbox (Minor Polish)

- [ ] Wire config.yaml to quarantine.py (currently hardcoded)
- [ ] Add more LinkedIn sender types as discovered
- [ ] LinkedIn auth checks (DKIM/SPF parsing) - enhancement

### Sentinel (Not Started)

- [ ] Decide: prompt-only vs middleware vs fork
- [ ] Step 2: Update SOUL.md with tool verification rules
- [ ] Step 3: Implement agent split (triage/operator)
- [ ] Step 4: Add audit JSONL to clawdbot flow

### Other

- [ ] Voice Mac→VPS Flow (P1) - separate workstream

---

## PART 6: HOW TO CONTINUE

### For Inbox Work

```bash
cd ~/AI/arkai/services/inbox
uv run pytest tests/ -v           # Run all tests
uv run arkai-inbox gmail -n 5     # Test live Gmail
```

### For Sentinel Work

1. Read this handoff
2. Read Chad's feedback (user will provide)
3. Decide on implementation approach
4. Start with SOUL.md update (lowest risk)
5. Build up to middleware/audit as needed

### Key Files to Read

- `/Users/alexkamysz/AI/arkai/.ralph/memory/handoffs/2026-01-30-inbox-review-architecture-locked.md` (original inbox spec)
- `~/clawd/SOUL.md` on VPS (current Claudia personality)
- `~/.clawdbot/clawdbot.json` on VPS (clawdbot config)

---

*This handoff is authoritative for session continuity. Chad's additional feedback should be incorporated before implementation decisions.*

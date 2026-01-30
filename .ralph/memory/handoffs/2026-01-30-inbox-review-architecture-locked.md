# Handoff: Inbox Review Architecture LOCKED

**Date:** 2026-01-30
**Session:** Steelmanning with Chad
**Status:** Architecture locked, ready for implementation

---

## Executive Summary

After 5+ iterations of steelmanning with Chad, the Unified Inbox Review System architecture is now LOCKED. This handoff captures all decisions, corrections, and implementation details.

---

## 1. REPO DECISION: MONOREPO

**Location:** `arkai/services/inbox/`

**Rationale:**
- Contracts + code in same repo = atomic commits, no split-brain
- Follows existing pattern (services/voice/, tts/)
- arkai-gmail is separate but inbox_review will be in monorepo

**Structure:**
```
arkai/
├── contracts/
│   ├── email_triage.schema.json          # Existing
│   ├── critic_evidence_bundle.schema.json # NEW
│   └── inbox_triage.schema.json          # NEW
│
├── services/inbox/                        # NEW
│   ├── pyproject.toml
│   ├── src/arkai_inbox/
│   │   ├── __init__.py
│   │   ├── normalize.py              # Pre-gate normalization
│   │   ├── risk_patterns.py          # RISK_PATTERNS + matching
│   │   ├── auth_score.py             # LinkedIn auth (soft signals)
│   │   ├── quarantine.py             # Hard quarantine rules
│   │   ├── evidence.py               # CriticEvidenceBundle
│   │   ├── url_extractor.py          # HTML parsing (not regex)
│   │   ├── clipboard.py              # Cross-platform copy
│   │   ├── audit.py                  # JSONL logging
│   │   ├── ingestion/
│   │   │   ├── gmail.py
│   │   │   └── linkedin.py
│   │   ├── reader/
│   │   │   └── classifier.py
│   │   ├── critic/
│   │   │   ├── policy.py
│   │   │   └── rules.py
│   │   └── cli/
│   │       ├── triage.py
│   │       └── obsidian.py           # Digest generator
│   └── tests/
│       ├── test_normalize.py
│       ├── test_risk_patterns.py
│       ├── test_quarantine.py
│       ├── test_url_extractor.py
│       └── fixtures/
│           ├── linkedin_real/        # 10 real Gmail API exports
│           └── linkedin_spoof/       # 5 spoof examples
```

---

## 2. HARD QUARANTINE RULES (LOCKED)

These override any score and immediately quarantine the email:

```python
HARD_QUARANTINE_RULES = [
    # Sender domain is NOT linkedin.com at all
    ("sender_wrong_domain",
     lambda e: not extract_email_address(e.from_header).endswith("@linkedin.com")),

    ("reply_to_mismatch",
     lambda e: e.reply_to and e.reply_to.lower() != e.from_address.lower()),

    ("deep_link_wrong_domain",
     lambda e: e.deep_link and not is_approved_linkedin_domain(e.deep_link)[0]),

    ("link_text_href_mismatch",
     lambda e: any(url.is_mismatch for url in e.extracted_urls)),
]
```

**NOT a hard quarantine:**
- `sender_not_exact_match` → goes to REVIEW tier (see Section 19)
- `missing_auth_headers` → soft signal only
- `missing_security_footer` → soft signal only

---

## 3. LINKEDIN VALID SENDERS (LOCKED)

```python
LINKEDIN_VALID_SENDERS = [
    "notifications-noreply@linkedin.com",
    "messages-noreply@linkedin.com",
    "invitations@linkedin.com",
    "jobs-noreply@linkedin.com",
]
```

---

## 4. LINKEDIN DOMAIN ALLOWLIST (LOCKED)

```python
# Approved (no extra warning)
LINKEDIN_APPROVED_DOMAINS = {
    "linkedin.com",
    "www.linkedin.com",
}

# Suspicious (allowed with extra warning + OPEN confirm)
LINKEDIN_SUSPICIOUS_DOMAINS = {
    "lnkd.in",  # LinkedIn's shortener
}

# Everything else → QUARANTINE
```

---

## 5. AUTH RISK SCORE (SOFT SIGNALS ONLY)

Score is for sorting/prioritization, NEVER a permission gate.

```python
SOFT_SIGNALS = [
    ("spf_pass", 0.15),
    ("dkim_pass", 0.15),
    ("dmarc_pass", 0.10),
    ("security_footer_present", 0.10),
    ("arc_valid", 0.05),
]
```

**Best-effort parsing:** Don't assume Gmail headers are consistent or present.

---

## 6. CRITIC EVIDENCE BUNDLE (LOCKED)

Reader CANNOT influence any of these fields. Pre-Gate computes all excerpts.

```python
@dataclass
class CriticEvidenceBundle:
    channel: Literal["gmail", "linkedin", "imessage", "telegram"]
    sender: str
    timestamp: datetime
    subject: Optional[str]

    # DETERMINISTIC (Pre-Gate computes, not Reader)
    first_200_normalized: str
    last_200_normalized: str

    # Link analysis (HTML-parsed)
    link_domains: list[str]
    link_mismatch_flags: list[str]
    link_shortener_flags: list[str]

    # Attachments
    has_attachments: bool
    attachment_types: list[str]

    # Hard quarantine results
    quarantine_reasons: list[str]

    # Soft auth score (sorting only)
    auth_risk_score: float
    auth_signals: dict[str, Any]

    # Reader's proposed output
    proposed_action: Optional[str]
    proposed_reply_draft: Optional[str]
```

---

## 7. URL EXTRACTION (LOCKED)

**Use HTML parsing (BeautifulSoup), NOT regex.**

Key checks:
- Extract `<a href>` targets + visible text
- Flag if visible text domain != href domain (phishing indicator)
- Flag shorteners (bit.ly, lnkd.in, etc.)
- Handle punycode/IDN safely

---

## 8. CLI UX (LOCKED)

### Open Link: 2-Step Confirmation
```
Type OPEN to open in browser, or press Enter to cancel:
> OPEN
```

### Explicit Labels (must show):
- "📝 DRAFT ONLY (not sent)"
- "⚠️ UNTRUSTED LINK"
- "Policy: NO delete, NO forward, NO auto-send"

### Extra Warning for lnkd.in:
```
⚠️⚠️ SUSPICIOUS SHORTENER: lnkd.in
LinkedIn shorteners can be spoofed. Proceed with caution.
```

---

## 9. OBSIDIAN (LOCKED)

**Mode:** View-only digest generator (Mode 1)
**Source of truth:** `~/.arkai/runs/{run_id}/events.jsonl`
**NOT a state machine** in MVP

**Output path formula:**
```
{vault_path}/{inbox_root}/{YYYY-MM-DD}.md
```

Example: `~/Obsidian/MainVault/00-Inbox/Digest/2026-01-30.md`

---

## 10. GOLDEN FIXTURES (REQUIREMENTS)

- Must be real Gmail API exports (not hand-constructed)
- Redacted (names, emails, content)
- Preserve header structure exactly
- 10 real LinkedIn notifications + 5 spoof examples

---

## 11. PRE-GATE NORMALIZATION (LOCKED)

```python
def normalize_for_risk_detection(raw_content: str) -> str:
    text = html_to_text(raw_content)
    text = unicodedata.normalize('NFKC', text)
    text = strip_zero_width_chars(text)
    text = collapse_whitespace(text)
    text = text.lower()
    return text
```

---

## 12. PRIORITIES (LOCKED)

| Priority | Task | Parallel OK |
|----------|------|-------------|
| P1 | Voice Mac→VPS Flow | Yes |
| P2 | Scaffold services/inbox/ | Yes (with P1) |
| P3+ | Pre-Gate, Quarantine, CLI | After P2 |

---

## 13. THINGS NOT TO BUILD YET

- ❌ AppleScript iMessage export (unproven)
- ❌ Obsidian as state machine
- ❌ Web dashboard (CLI-first)
- ❌ LinkedIn API (Gmail notifications only)
- ❌ Auto-send anything

---

## 14. THINGS ALREADY BUILT (DON'T DUPLICATE)

- ✅ arkai-gmail (separate repo, Reader/Critic/Actor proven)
- ✅ services/voice/ (VPS runner, Clawdbot client)
- ✅ contracts/email_triage.schema.json
- ✅ docs/SECURITY_POSTURE.md (threat model added)

---

## 15. NEXT SESSION CHECKLIST

1. [x] **Add `arkai-gmail export` command** ✅ DONE (c783bea) - exports message as raw JSON for fixtures
2. [x] Create services/inbox/ directory structure ✅ DONE (2e9bbb2)
3. [x] Create pyproject.toml with dependencies ✅ DONE
4. [x] Implement normalize.py + tests ✅ DONE (39 tests, 100% coverage)
5. [x] Implement quarantine.py + tests ✅ DONE (34 tests)
6. [x] Implement url_extractor.py (BeautifulSoup) + tests ✅ DONE (44 tests)
7. [x] Create critic_evidence_bundle.schema.json ✅ DONE
8. [ ] Export 10 real LinkedIn notifications + 5 spoofs as fixtures
9. [ ] Implement CLI triage loop

---

## 15b. STYLE CONSTRAINTS (REPLY DRAFTS)

When generating reply drafts:
- ❌ No em dashes (—)
- ✅ Use regular dashes (-) or commas instead
- More constraints TBD (user will provide example replies later)

---

## 16. FILES MODIFIED THIS SESSION

- `docs/SECURITY_POSTURE.md` - Added Inbox Review Threat Model section

---

## 17. CHAD'S WISDOM (KEY QUOTES)

> "Treat score as sorting/quarantine signal ONLY, never a permission gate."

> "Avoid regex URL extraction on raw HTML. Use HTML parsing."

> "Hard quarantine rules should not depend on headers that may be missing."

> "Real fixtures must be exported from actual Gmail API, not hand-constructed."

> "Obsidian is NOT a state machine in MVP; JSONL event log remains source of truth."

---

---

## 18. CONFIG.YAML (FINAL - ADHD-OPTIMIZED)

Single config file. No sprawl. Tilde allowed (expanded at runtime).

```yaml
# ~/.arkai/config.yaml

obsidian:
  enabled: true
  vault_path: ~/Obsidian/vault-sandbox  # Test vault
  inbox_root: 00-Inbox/Digest            # Relative to vault

linkedin:
  exact_pass:
    - notifications-noreply@linkedin.com
    - messages-noreply@linkedin.com
    - invitations@linkedin.com
    - jobs-noreply@linkedin.com
  domain_review: "@linkedin.com"
```

---

## 19. LINKEDIN SENDER TIERS (FINAL)

```python
import re

def extract_email_address(from_header: str) -> str:
    """
    Extract email from From header.
    'LinkedIn <notifications-noreply@linkedin.com>' -> 'notifications-noreply@linkedin.com'
    'notifications-noreply@linkedin.com' -> 'notifications-noreply@linkedin.com'
    """
    match = re.search(r'<([^>]+)>', from_header)
    if match:
        return match.group(1).lower()
    return from_header.strip().lower()

def evaluate_sender(from_header: str) -> tuple[str, list[str]]:
    sender = extract_email_address(from_header)

    # Tier 1: PASS (exact match from config)
    if sender in config["linkedin"]["exact_pass"]:
        return ("PASS", [])

    # Tier 2: REVIEW (linkedin.com domain but unknown sender)
    if sender.endswith(config["linkedin"]["domain_review"]):
        return ("REVIEW", ["sender_not_in_exact_allowlist"])

    # Tier 3: QUARANTINE (wrong domain entirely)
    return ("QUARANTINE", ["sender_wrong_domain"])
```

---

## 20. OBSIDIAN STRUCTURE (MINIMAL)

```
00-Inbox/
├── Digest/              # Daily digests (MVP)
│   └── 2026-01-30.md
└── Quarantine/          # Hard quarantines (add later if needed)
```

**NOT building (avoid until real usage data):**
- ❌ Channels/
- ❌ Queue/
- ❌ Archive/
- ❌ Routing rules

---

## 21. CONFIG VALIDATION (LIGHTWEIGHT)

No JSON schema. Runtime checks only. Tilde is allowed and expanded.

```python
from pathlib import Path

def validate_config(config: dict) -> list[str]:
    errors = []

    if config.get("obsidian", {}).get("enabled"):
        vault_path = config["obsidian"].get("vault_path", "")

        # Expand ~ and resolve to absolute
        resolved_path = Path(vault_path).expanduser().resolve()

        # Must exist after expansion
        if not resolved_path.exists():
            errors.append(f"vault_path does not exist: {resolved_path}")

        # inbox_root must be relative (no leading /)
        inbox_root = config["obsidian"].get("inbox_root", "")
        if inbox_root.startswith("/"):
            errors.append(f"inbox_root must be relative (got: {inbox_root})")

    return errors
```

---

## 22. DIGEST FORMAT (WITH EVENT POINTERS)

```markdown
# Inbox Digest - 2026-01-30

## 📌 Action Needed

### John Smith (LinkedIn) - 2h ago
- **Summary:** Wants to discuss your post about...
- **Draft:** Hey John! Thanks for reaching out...
- **Link:** `linkedin.com` [^evt-abc123]

> [!info]- Full URL (audit)
> https://www.linkedin.com/messaging/thread/2-xxx

---
*Source: ~/.arkai/runs/inbox-2026-01-30/events.jsonl*

[^evt-abc123]: Event ID: abc123
```

---

## 23. ADHD OPTIMIZATION PRINCIPLES

| Principle | Implementation |
|-----------|----------------|
| One source of truth | `config.yaml` (config), JSONL (data) |
| No decision fatigue | One folder (`Digest/`), no routing |
| Clear naming | `Digest` not `Unified` |
| Immediate feedback | Runtime validation, no schema |
| Start minimal | Add complexity when real data shows need |

---

## 24. CANONICAL PATHS (USE THESE EVERYWHERE)

All paths use `~` for generalizability. Expand at runtime with `Path.expanduser()`.

| Purpose | Path | Notes |
|---------|------|-------|
| **Config** | `~/.arkai/config.yaml` | Single source of config |
| **Event Store** | `~/.arkai/runs/{run_id}/events.jsonl` | JSONL is source of truth |
| **Catalog** | `~/.arkai/catalog.json` | Content index |
| **Voice Queue** | `~/.arkai/voice_queue.jsonl` | Voice memo queue |
| **Voice Cache** | `~/.arkai/voice_cache/` | Transcription cache |
| **Obsidian Digest** | `{vault_path}/{inbox_root}/{YYYY-MM-DD}.md` | View layer only |
| **arkai repo** | `~/AI/arkai/` | Main monorepo |
| **arkai-gmail** | `~/AI/arkai-gmail/` | Separate repo (existing) |
| **Services** | `~/AI/arkai/services/` | Python services (voice/, inbox/) |
| **Contracts** | `~/AI/arkai/contracts/` | JSON schemas |

**Runtime expansion:**
```python
from pathlib import Path

config_path = Path("~/.arkai/config.yaml").expanduser()
vault_path = Path(config["obsidian"]["vault_path"]).expanduser().resolve()
```

**Never hardcode usernames** (no `/Users/alexkamysz/`, no `/Users/olek/`).

---

*This handoff is authoritative. If any implementation contradicts these decisions, refer back here.*

---

## 25. INTEGRATION CONTRACTS (LOCKED - 2026-01-30)

**Files added:** `services/inbox/src/arkai_inbox/models.py`, `audit.py`

### EmailRecord (Ingestion Output)

```python
@dataclass
class EmailRecord:
    message_id: str       # Gmail API message ID
    thread_id: str
    from_header: str      # "LinkedIn <notifications-noreply@linkedin.com>"
    to: list[str]
    subject: str | None
    date: datetime
    reply_to: str | None = None
    snippet: str = ""
    html_body: str | None = None
    text_body: str | None = None
    has_attachments: bool = False
    attachment_types: list[str] = field(default_factory=list)
    labels: list[str] = field(default_factory=list)
    channel: Literal["gmail", "linkedin", "imessage", "telegram"] = "gmail"
    raw_headers: dict[str, str] = field(default_factory=dict)
```

### create_evidence_bundle() Pipeline

```
EmailRecord ──► create_evidence_bundle() ──► CriticEvidenceBundle
                       │
                       ├── normalize.py (first_200, last_200)
                       ├── url_extractor.py (links, mismatches, shorteners)
                       └── quarantine.py (tier, reasons)
```

### AuditEvent (JSONL Logging)

```python
@dataclass
class AuditEvent:
    event_id: str         # UUID
    timestamp: datetime
    stage: Literal["ingested", "pre_gate", "quarantined", "reader_start",
                   "reader_complete", "critic_verdict", "action_executed", "skipped"]
    message_id: str
    channel: str
    quarantine_tier: str | None = None
    quarantine_reasons: list[str] | None = None
    link_domains: list[str] | None = None
    action: str | None = None
    draft_path: str | None = None
    error: str | None = None
```

---

## 26. CHAD'S RECOMMENDED ORDER (2026-01-30)

**Do NOT skip steps. This order prevents thrash.**

### Phase 1: Fixtures (NEXT SESSION)

Minimum to unblock integration:
- [ ] 3 real LinkedIn message notifications
- [ ] 1 LinkedIn connection invite
- [ ] 1 LinkedIn digest/weekly email
- [ ] 2 spoofs:
  - link text says linkedin.com but href is evil.com
  - From is @linkedin.com but Reply-To is different

Use: `arkai-gmail export -m <message_id> -o fixtures/linkedin_real/msg1.json`

### Phase 2: Thin Vertical Slice CLI

**NOT full interactive UI.** Just:
1. Reads fixtures from folder
2. Runs: `EmailRecord` → `create_evidence_bundle()` → print summary
3. Prints: PASS/REVIEW/QUARANTINE, reasons, domains, first_200/last_200
4. Logs audit event
5. No "open link" yet

### Phase 3: Gmail Live Ingestion

Only after fixtures pipeline is stable:
- Query "LinkedIn notifications in last N days"
- Export to fixtures-like JSON
- Run pipeline

### Phase 4: Interactive CLI

Add commands: copy, open (with 2-step confirm), skip

---

## 27. REMAINING TASKS (UPDATED)

| # | Task | Status | Blocked By |
|---|------|--------|------------|
| 1 | Export 5+2 LinkedIn fixtures | **NEXT** | - |
| 2 | Thin vertical slice CLI | Pending | #1 |
| 3 | Gmail live ingestion | Pending | #2 |
| 4 | Interactive CLI commands | Pending | #3 |
| 5 | Obsidian digest generator | Pending | #2 |

**Completed this session:**
- [x] arkai-gmail export command (c783bea)
- [x] services/inbox/ scaffold (2e9bbb2)
- [x] normalize.py + tests (39 tests)
- [x] quarantine.py + tests (34 tests)
- [x] url_extractor.py + tests (44 tests)
- [x] critic_evidence_bundle.schema.json
- [x] models.py integration contracts (8d396f0)
- [x] audit.py MVP JSONL logging (8d396f0)

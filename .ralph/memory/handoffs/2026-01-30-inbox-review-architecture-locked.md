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
    ("sender_not_exact_match",
     lambda e: e.from_address.lower() not in LINKEDIN_VALID_SENDERS),

    ("reply_to_mismatch",
     lambda e: e.reply_to and e.reply_to.lower() != e.from_address.lower()),

    ("deep_link_wrong_domain",
     lambda e: e.deep_link and not is_approved_linkedin_domain(e.deep_link)[0]),

    ("link_text_href_mismatch",
     lambda e: any(url.is_mismatch for url in e.extracted_urls)),
]
```

**NOT a hard quarantine (soft signal only):**
- missing_auth_headers (Gmail headers unreliable)
- missing_security_footer (soft signal)

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

Output: `~/Obsidian/Inbox/{date}.md`

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

1. [ ] Create services/inbox/ directory structure
2. [ ] Create pyproject.toml with dependencies
3. [ ] Implement normalize.py + tests
4. [ ] Implement quarantine.py + tests
5. [ ] Implement url_extractor.py (BeautifulSoup) + tests
6. [ ] Create critic_evidence_bundle.schema.json
7. [ ] Obtain real Gmail API fixture exports (from you)
8. [ ] Implement CLI triage loop

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

*This handoff is authoritative. If any implementation contradicts these decisions, refer back here.*

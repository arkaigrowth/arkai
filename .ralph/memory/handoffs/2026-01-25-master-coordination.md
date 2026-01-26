# Master Coordination Handoff

**Date:** 2026-01-25
**Session:** arkai master (clawdbot-builder)
**Context:** Consolidating multiple Claude sessions into coordinated plan

---

## Session Inventory

| Session | Focus | Status | Next Action |
|---------|-------|--------|-------------|
| **This (master)** | Architecture, Claudia, VPS, coordination | Active | Create this handoff, then compact |
| **triage-sidecar** | Gmail integration | Active | Read SECURITY_POSTURE.md, do Phase 0 |
| **arkai-voice-builder** | Voice pipeline | Active | Complete Voice Intake v1 only |

---

## MASTER PLAN (Chad-Approved)

### Priority Order (What Gets Built First)

```
Phase 0: Security Hardening     ← BLOCKING, do first
    ↓
Phase 1a: Voice Intake v1       ← Can proceed after Phase 0
Phase 1b: Gmail Triage MVP      ← Can proceed after Phase 0
    ↓
Phase 2: Claudia UX Integration ← After 1a and 1b artifacts exist
    ↓
Phase 3: TTS/Voice Response     ← HOLD, not now
```

### Phase 0: Security Hardening (triage-sidecar owns)

**PREREQUISITE for all Gmail work.**

- [ ] Remove clawdbot from sudoers NOPASSWD
- [ ] Remove clawdbot from docker group
- [ ] Create `arkai-exec` user (non-privileged)
- [ ] Document in SECURITY_POSTURE.md ✅ (done this session)

### Phase 1a: Voice Intake v1 (arkai-voice-builder owns)

**Scope:** Siri → Voice Memo → arkai watcher → classify → Obsidian

- [ ] Intent classification (NOTE/TASK/IDEA/QUESTION/COMMAND)
- [ ] Confidence score
- [ ] Canonical artifact write
- [ ] Daily memory summary for Claudia
- [ ] Idempotency (dedup by content hash)

**Contract:** `contracts/voice_intake.schema.json` ✅ (exists)

**Pattern:** Reader/Actor split (Reader classifies, Actor writes)

### Phase 1b: Gmail Triage MVP (triage-sidecar owns)

**Scope:** Gmail API → fetch → classify → draft-only

- [ ] Gmail API OAuth setup
- [ ] Fetch emails to VPS (encrypted at rest)
- [ ] Reader LLM triages (no tools, JSON only)
- [ ] Critic validates (blocks policy violations)
- [ ] Actor creates drafts only, applies labels
- [ ] Retention policy (7-14 days)

**Contract:** `contracts/email_triage.schema.json` (TODO: create)

**Pattern:** Reader/Actor/Critic split

### Phase 2: Claudia UX Integration (this session owns)

**Scope:** Claudia reads artifacts, presents to user

- [ ] Claudia command: "check email"
- [ ] Claudia reads triage results JSON (not raw bodies)
- [ ] Claudia presents: counts + top 5 + approval prompt
- [ ] Claudia writes daily memory summary
- [ ] Same pattern for voice summaries

### Phase 3: TTS/Voice Response (HOLD)

**Do NOT start until Phases 1-2 complete.**

- ElevenLabs integration
- Claudia voice responses via Telegram

---

## Reader/Actor/Critic Pattern (Use for Everything)

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│     READER      │     │     CRITIC      │     │     ACTOR       │
│  (LLM, no tools)│ ──▶ │  (Code, validates)│ ──▶ │  (Code, executes)│
│  Outputs JSON   │     │  Blocks bad actions│    │  Safe actions only│
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

**Voice:** Reader classifies intent → Critic validates → Actor writes to Obsidian
**Email:** Reader triages → Critic validates → Actor creates drafts

---

## Files Created This Session

| File | Purpose |
|------|---------|
| `docs/ARCHITECTURE.md` | Unified system map |
| `docs/SECURITY_POSTURE.md` | Security requirements |
| `contracts/voice_intake.schema.json` | Voice pipeline contract |
| `contracts/README.md` | Contract philosophy |
| `.claude/CLAUDE.md` | Project onboarding for all sessions |
| Claudia's `ARKAI.md` update | Pattern discovery instructions |

---

## Files Needed (TODO)

| File | Owner | Purpose |
|------|-------|---------|
| `contracts/email_triage.schema.json` | triage-sidecar | Email triage contract |
| VPS security hardening | triage-sidecar | Phase 0 execution |
| Voice Intake v1 code | arkai-voice-builder | Complete the pipeline |

---

## VPS State

| Component | Status |
|-----------|--------|
| Claudia (Clawdbot) | ✅ Running |
| ~/arkai/ | ✅ Synced (but no binary) |
| ~/fabric-arkai/ | ✅ Synced (pattern_index.json) |
| ~/clawd/ | ✅ Git-tracked |
| Rust/Cargo | ❌ Not installed |
| clawdbot user | ⚠️ Has root-equivalent access (FIX) |

---

## Open Questions

1. **Where does arkai-gmail service run?**
   - Chad says VPS (not Mac-dependent)
   - Needs arkai-exec user first

2. **How does Claudia trigger "check email"?**
   - Reads triage results JSON from arkai-gmail output
   - Location TBD: `~/.arkai/gmail/triage_results.json`?

3. **Shared memory format for Claudia?**
   - Daily summary from voice + email
   - Format: `memory/YYYY-MM-DD.md` with sections for each source

---

## Commands for Other Sessions

### For triage-sidecar:
```
Read docs/SECURITY_POSTURE.md first.
Complete Phase 0 hardening before any Gmail work.
Create contracts/email_triage.schema.json using voice_intake as template.
```

### For arkai-voice-builder:
```
Freeze streaming/audio-to-VPS ideas.
Complete Voice Intake v1 only:
- Intent classification + confidence
- Canonical artifact write
- Daily memory summary
- Idempotency
Follow Reader/Actor pattern from SECURITY_POSTURE.md.
```

---

## Resume This Session With

```bash
cd ~/AI/arkai
claude
# Read .ralph/memory/handoffs/2026-01-25-master-coordination.md
# Continue Phase 2: Claudia UX integration
```

---

## Chad's Coordination Points

1. **Hold ElevenLabs/TTS** — Not now
2. **Security hardening is BLOCKING** — Must complete before Gmail
3. **Same Reader/Actor pattern** — Reuse for voice and email
4. **Claudia sees summaries, not raw bodies** — Privacy + context efficiency
5. **Draft-only for Gmail** — Never auto-send

---

*This is the coordination document. All sessions should read this and their specific phase.*

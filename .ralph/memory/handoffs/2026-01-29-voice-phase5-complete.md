# Voice Pipeline Phase 5 Complete + Next Steps Handoff

> **Created**: 2026-01-29
> **Session**: Master Session 5
> **Status**: VPS→Claudia WORKING, Mac→VPS NOT WIRED

---

## TL;DR

**What's working:** Telegram voice → VPS runner → Groq transcription → Claudia webhook ✅
**What's NOT working:** Mac Voice Memos → VPS (the "???" gap)
**What's NOT started:** Speaker Detection, Mac Diarizer, Web Research, Slack

---

## 1. Current State (Verified Working)

### Voice Pipeline Components

| Component | Status | Location |
|-----------|--------|----------|
| Path Authority (Rust) | ✅ DONE | `src/config/paths.rs` |
| Path Authority (Python) | ✅ DONE | `services/voice/paths.py` |
| Contracts (schemas) | ✅ DONE | `contracts/voice_*.schema.json` |
| VPS Voice Runner | ✅ RUNNING | `~/clawd/services/voice/vps_voice_runner.py` |
| Clawdbot Client (Python) | ✅ DONE | `services/voice/clawdbot_client.py` |
| Webhook to Claudia | ✅ WORKING | `webhook_sent` in audit log |
| Smoke Test | ✅ DONE | `scripts/smoke_voice.sh` |
| Groq→OpenAI Fallback | ✅ TESTED | Audit shows `provider: openai` |
| Schema Rejection | ✅ TESTED | Malformed requests logged as errors |

### VPS Runner Features
- Polling loop (1s interval) - matches tts-watcher pattern ✅
- Atomic claim via `.inflight/` directory ✅
- Idempotency (skip if result exists) ✅
- Groq primary, OpenAI fallback ✅
- Retry with backoff (3 attempts) ✅
- Audit JSONL logging ✅
- Webhook notification to Claudia ✅

### E2E Test Results
```
Request: e2e-test-1769671569
Flow: received → claimed → transcribed (groq, 0.8s) → result_written → webhook_sent (ok)
Latency: <1 second total
```

---

## 2. The Gap: Mac → VPS

```
┌──────────────────────┐         ┌──────────────────────────┐
│     Mac              │   ???   │         VPS              │
│  Voice Memos App     │  ───►   │   Request JSON           │
│  .m4a files          │         │   Runner processes       │
└──────────────────────┘         └──────────────────────────┘
```

**Options to implement:**
- A) `arkai voice process` creates request JSON + rsyncs audio to VPS
- B) Webhook-based: Mac tells VPS "go fetch"
- C) Tailscale mount: shared filesystem

**Recommendation:** Option A is most straightforward. Mac creates request, copies audio to VPS audio-cache, runner picks it up.

---

## 3. NOT DONE (From Chad/Claudia Build Prompt)

### Voice Pipeline Remaining

| Phase | Description | Status |
|-------|-------------|--------|
| Speaker Detection (Tier 2) | VAD + MFCC variance | ❌ NOT STARTED |
| Mac Diarizer | pyannote via Moltbot node | ❌ NOT STARTED |
| Mac → VPS Request Flow | The "???" gap | ❌ NOT STARTED |

### New Workstreams (Not Started)

| Workstream | Description | Status |
|------------|-------------|--------|
| **Web Research** | WebReader → PolicyGate → Sanitized Evidence | ❌ NOT STARTED |
| **Slack Integration** | Ingest Slack messages safely | ❌ NOT STARTED |

---

## 4. Chad's Key Wisdom (Carry Forward)

### Security Patterns
1. **Reader/Critic/Actor** - Proven with Gmail, apply to Web Research
2. **Sanitizer is a speed bump, not security** - Real safety = tool isolation + write-only evidence + denied web tools + human approvals
3. **Slack = untrusted input forever** - Treat like email

### Architecture Patterns
1. **Polling loop > watchdog complexity** - VPS runner uses simple glob+sleep
2. **Artifact-first** - If it's not on disk, it doesn't exist
3. **Fail-fast at startup** - Missing API keys = immediate exit
4. **Audit everything** - JSONL logs for full traceability

### Testing Patterns
1. **Golden path first** - Test simplest working path before adding complexity
2. **Lock observability** - Single smoke script with PASS/FAIL
3. **Test failure modes** - Fallback + rejection explicitly verified

---

## 5. Files Created/Modified This Session

### New Files
```
services/voice/clawdbot_client.py    # Python webhook client
scripts/smoke_voice.sh               # E2E smoke test
```

### Modified Files
```
services/voice/vps_voice_runner.py   # Added webhook notification
.ralph/memory/tickets/VOICE_PHASE_5_INTEGRATION.yaml  # Marked DONE
```

### VPS Deployment
```
~/clawd/services/voice/vps_voice_runner.py  # Updated with webhook
~/clawd/services/voice/clawdbot_client.py   # NEW
~/clawd/services/voice/paths.py             # NEW
~/clawd/services/voice/.env                 # Added CLAWDBOT_TOKEN
```

---

## 6. Environment Variables (VPS)

```bash
# ~/clawd/services/voice/.env
GROQ_API_KEY=gsk_...        # Primary transcription
OPENAI_API_KEY=sk-proj-...  # Fallback transcription
CLAWDBOT_TOKEN=07b67384...  # Webhook auth
```

---

## 7. Quick Commands

```bash
# Run smoke test (Mac)
./scripts/smoke_voice.sh

# Check VPS runner
ssh clawdbot-vps 'ps aux | grep voice'

# Check audit log
ssh clawdbot-vps 'tail -20 ~/clawd/artifacts/voice/audit.jsonl'

# Restart VPS runner
ssh clawdbot-vps 'pkill -f vps_voice_runner; cd ~/clawd/services/voice && source .env && export GROQ_API_KEY OPENAI_API_KEY CLAWDBOT_TOKEN && nohup .venv/bin/python vps_voice_runner.py &'

# Manual test request
ssh clawdbot-vps 'echo "{\"id\":\"test-$(date +%s)\",\"action\":\"process\",\"params\":{\"limit\":1},\"requested_by\":\"test\",\"requested_at\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" > ~/clawd/artifacts/voice/requests/test-$(date +%s).json'
```

---

## 8. Next Session Build Order

**Recommended sequence:**

1. **Mac → VPS Flow** (unblocks real voice memo testing)
   - Update `arkai voice process` to create VPS request + sync audio
   - Test with actual Voice Memo

2. **Speaker Detection** (enhances transcription quality)
   - Create `speaker_detector.py` on VPS
   - Integrate into runner

3. **Web Research Pipeline** (new capability)
   - WebReader agent config
   - PolicyGate sanitizer
   - Evidence pack schema
   - Claudia restrictions

4. **Slack Integration** (optional, lowest priority)
   - Adapter for channel ingestion
   - Normalized message schema

---

## 9. Known Issues / Follow-ups

1. **VPS runner log location** - `runner.log` not writing (nohup redirect issue)
2. **contracts/README.md** - Exists but may need update per build prompt
3. **Systemd service** - Not enabled (runner started manually)
4. **Mac diarizer** - Needs pyannote installed on Mac
5. **Moltbot node setup** - Mac not registered as node yet

---

## 10. Phase Numbering Clarification

The build prompt uses different phase numbers than our tickets:

| Build Prompt | Our Ticket | Status |
|--------------|------------|--------|
| Phase 2 (Paths) | VOICE_PHASE_2_3 | ✅ DONE |
| Phase 3 (Schemas) | VOICE_PHASE_2_3 | ✅ DONE |
| Phase 4 (VPS Runner) | VOICE_PHASE_4 | ✅ DONE |
| Phase 5 (Speaker Detect) | NOT TICKETED | ❌ TODO |
| Phase 6 (Mac Diarizer) | NOT TICKETED | ❌ TODO |
| "Phase 5 Integration" | VOICE_PHASE_5_INTEGRATION | ✅ DONE |

---

*Handoff created 2026-01-29 by Master Session 5*

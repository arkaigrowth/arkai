# OpenRouter Migration Complete + Voice Pipeline Next

> **Created**: 2026-01-30
> **Previous**: `2026-01-29-voice-phase5-complete.md`
> **Status**: OpenRouter DONE, Voice Pipeline READY

---

## TL;DR

**Completed this session:** Switched Claudia from Claude Max to OpenRouter (Kimi K2-0905)
**Next session:** Mac→VPS voice flow, Speaker Detection, Web Research

---

## 1. What We Accomplished

### OpenRouter Migration (Priority 0) ✅ DONE

| Task | Status |
|------|--------|
| Verified clawdbot package authenticity | ✅ steipete (legit) |
| Updated clawdbot 2026.1.23-1 → 2026.1.24-3 | ✅ |
| Added OpenRouter API key | ✅ |
| Set Kimi K2-0905 as default model | ✅ |
| Set GLM-4.7 as fallback | ✅ |
| Gateway running on Telegram | ✅ |
| Bug monitoring cron job | ✅ |

### Current Model Configuration

```
Default:  openrouter/moonshotai/kimi-k2-0905 (256k context)
Fallback: openrouter/z-ai/glm-4.7 (198k context)
Future:   openrouter/moonshotai/kimi-k2.5 (when bugs fixed)
```

### Why K2-0905 Instead of K2.5?

- K2.5 not in clawdbot 2026.1.24-3 catalog (added in 2026.1.29 beta)
- Beta has open bugs: #4143 (K2.5 null outputs), #3475 (silent failures)
- K2-0905 is stable, same 256k context, 8x cheaper than Opus

### Monitoring Setup

```bash
# Check Kimi bug status (runs every 6 hours via cron)
ssh clawdbot-vps 'cat ~/logs/moltbot-monitor.log'

# Issues being tracked:
# - #4143: Kimi 2.5 null outputs
# - #3475: Kimi/Moonshot silent failures
# - #3155: K2.5 support feature request
```

---

## 2. Environment Variables (VPS)

```bash
# ~/.clawdbot/.env
ANTHROPIC_API_KEY=sk-ant-oat01-...  # Backup (not primary)
OPENAI_API_KEY=sk-proj-...          # For Whisper transcription
OPENROUTER_API_KEY=sk-or-v1-078...  # PRIMARY - Kimi/GLM models
```

**OpenRouter Key**: Stored in Mac Keychain as `moltbot-openrouter-api-key`

---

## 3. Model Comparison Summary

| Model | Cost vs Opus | Best For | User Sentiment |
|-------|--------------|----------|----------------|
| **Kimi K2-0905** | 8x cheaper | Daily coding, agents | "Smart, proven" |
| **GLM-4.7** | 12x cheaper | Speed, budget, UI | "Just works" |
| **Kimi K2.5** | 8x cheaper | Vision, agents | "Buggy in beta" |
| **Opus 4.5** | Baseline | Complex architecture | "Great but expensive" |

---

## 4. What's NOT Done (From Original Plan)

### Voice Pipeline Remaining

| Phase | Description | Status |
|-------|-------------|--------|
| **Mac → VPS Flow** | Voice Memos → VPS request | ❌ NOT STARTED |
| Speaker Detection | VAD + MFCC variance | ❌ NOT STARTED |
| Mac Diarizer | pyannote via Moltbot node | ❌ NOT STARTED |

### New Workstreams (From Chad/Claudia Build Prompt)

| Workstream | Description | Status |
|------------|-------------|--------|
| Web Research | WebReader → PolicyGate → Evidence | ❌ NOT STARTED |
| Slack Integration | Ingest Slack messages safely | ❌ NOT STARTED |

---

## 5. Next Session Build Order

**PRIORITY 1: Mac → VPS Voice Flow**

This is the "???" gap from the original handoff:
```
┌──────────────────────┐         ┌──────────────────────────┐
│     Mac              │   ???   │         VPS              │
│  Voice Memos App     │  ───►   │   Request JSON           │
│  .m4a files          │         │   Runner processes       │
└──────────────────────┘         └──────────────────────────┘
```

Implementation options:
- A) `arkai voice process` creates request JSON + rsyncs audio to VPS
- B) Webhook-based: Mac tells VPS "go fetch"
- C) Tailscale mount: shared filesystem

**Recommendation:** Option A is most straightforward.

**THEN:**
1. Speaker Detection (enhances transcription quality)
2. Web Research Pipeline (new capability)
3. Slack Integration (optional, lowest priority)

---

## 6. Quick Commands

```bash
# Check gateway status
ssh clawdbot-vps 'ps aux | grep clawdbot-gateway | grep -v grep'

# Check model in use
ssh clawdbot-vps 'tail -20 /tmp/clawdbot/clawdbot-$(date +%Y-%m-%d).log | grep model'

# Restart gateway
ssh clawdbot-vps 'source ~/.nvm/nvm.sh && pkill -f clawdbot-gateway; sleep 2; source ~/.clawdbot/.env && export ANTHROPIC_API_KEY OPENAI_API_KEY OPENROUTER_API_KEY && nohup clawdbot gateway >> ~/gateway.log 2>&1 &'

# Check voice runner
ssh clawdbot-vps 'ps aux | grep voice_runner'

# Run smoke test
./scripts/smoke_voice.sh

# Check Kimi bug monitor
ssh clawdbot-vps 'tail -20 ~/logs/moltbot-monitor.log'
```

---

## 7. Files Modified This Session

### VPS Files
```
~/.clawdbot/.env                    # Updated OpenRouter key
~/.clawdbot/clawdbot.json           # Model config (K2-0905 + GLM-4.7)
~/bin/check-moltbot-issues.sh       # NEW - bug monitoring script
~/logs/moltbot-monitor.log          # NEW - monitoring output
```

### No Local Files Modified
(All changes were VPS-side configuration)

---

## 8. Key Learnings / Gotchas

1. **npm moltbot is squatted** - Use `clawdbot` package or `moltbot@beta` only
2. **K2.5 needs clawdbot 2026.1.29+** - Current stable (2026.1.24) doesn't have it
3. **Gateway hot-reloads config** - But model changes need restart
4. **OpenRouter keys can be disabled** - Check account if 401 errors
5. **Clawdbot uses nvm** - Must `source ~/.nvm/nvm.sh` before commands

---

## 9. Context for Next Session

**Read these files first:**
1. This handoff (you're reading it)
2. `.ralph/memory/specs/VOICE_PIPELINE_V2.1_BUILD_SPEC.md` - Full voice spec
3. `src/cli/voice.rs` - Existing voice CLI commands
4. `services/voice/vps_voice_runner.py` - VPS runner (working)

**What's already working:**
- VPS voice runner (Groq transcription + webhook to Claudia)
- Smoke test script (`./scripts/smoke_voice.sh`)
- Claudia responding via Telegram on Kimi K2-0905

**What needs building:**
- Mac → VPS request creation + audio sync
- Integration with `arkai voice process` command

---

*Handoff created 2026-01-30 by Master Session (post-OpenRouter migration)*

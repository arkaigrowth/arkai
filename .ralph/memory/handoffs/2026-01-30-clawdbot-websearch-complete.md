# Session Handoff: 2026-01-30 Clawdbot Web Search

## Summary
Fixed Clawdbot gateway crash, configured web search with Perplexity Sonar, created security hardening backlog, and set up provenance tracking infrastructure.

## Completed ✅
1. **Gateway crash fixed** - Invalid model `kimi-k2.5` removed from config
2. **Web search enabled** - Perplexity Sonar via OpenRouter working
3. **Web fetch enabled** - 30k char limit, markdown extraction
4. **Provenance infrastructure** - blocklist, sanitizer, audit log created
5. **Security Hardening section** - Added to ROADMAP.md with phased approach
6. **SECURITY_POSTURE.md** - Updated with web search section
7. **Voice/text logic** - Added to SOUL.md (fixes voice-for-text issue)
8. **Session reset** - `/new` command fixed Claudia's tool awareness

## Known Limitations ⚠️
- **Freshness filter** doesn't work via OpenRouter (use date terms in query instead)
- **Brave fallback** not configured (only Perplexity)
- **Audit log** - Claudia narrates logging but may not actually write (needs verification)

## Files Changed

### VPS (~/.clawdbot/, ~/clawd/)
- `~/.clawdbot/clawdbot.json` - web_search, web_fetch enabled
- `~/clawd/SOUL.md` - Web security section + voice/text logic
- `~/clawd/security/provenance/blocklist.txt` - 30+ injection patterns
- `~/clawd/security/provenance/sanitizer.py` - Python sanitization module
- `~/clawd/memory/web_audit.jsonl` - Created (empty)

### Repo (committed + pushed)
- `docs/ROADMAP.md` - Security Hardening section (Phases 0-4)
- `docs/SECURITY_POSTURE.md` - Web search section, updated checklists

## Backlog (from Security Hardening Phase 1)
- [ ] Hook integration (auto-sanitize web results)
- [ ] Domain allowlist mode
- [ ] Provenance wrapper (content tagging)
- [ ] Double-check mode (human review)
- [ ] Rate limiting
- [ ] Brave API fallback
- [ ] Slack integration (user interested)

## Commands Reference
```bash
# SSH to VPS
ssh clawdbot-vps

# Restart gateway
screen -S clawdbot -X quit; screen -dmS clawdbot bash -c 'source ~/.nvm/nvm.sh && clawdbot gateway 2>&1 | tee ~/gateway.log'

# Check config
python3 -c "import json; print(json.dumps(json.load(open('/home/clawdbot/.clawdbot/clawdbot.json')).get('tools',{}).get('web',{}), indent=2))"

# Test sanitizer
python3 ~/clawd/security/provenance/sanitizer.py
```

## Future Integration: Apple Notes

**Skill:** `apple-notes` (bundled, requires `memo` CLI on macOS)

**Architecture:**
```
Telegram → VPS (Claudia) → Remote Skill → Mac → memo CLI → Apple Notes
```

**Security requirements:**
- Read-only first (list, view only)
- Notes treated as untrusted input (Reader/Critic/Actor)
- Folder allowlist (specific notebooks only)
- Mac must be online

**Prerequisites:**
- Install `memo` CLI on Mac: `brew install memoapp/tap/memo`
- Configure remote skills in Clawdbot
- Add to Security Hardening backlog

# Handoff: OpenClaw VPS Deployment (In Progress)

**Date:** 2026-02-01
**Session:** Fresh VPS install + OpenClaw secure deployment
**Status:** Gateway running, Telegram NOT responding to messages

---

## PART 1: WHAT WE ACCOMPLISHED

### VPS Wipe & Fresh Install
- Wiped old `arkai-clawdbot` Hetzner VPS
- Renamed to `arkai-openclaw`
- Fresh Ubuntu 24.04 LTS installed
- Provisioning script from other Claude ran successfully

### Backups Created (Safe on Mac)
```
~/AI/clawdbot-backups/
├── vps-backup-20260201-clean.tar.gz        27 KB  (curated essentials)
└── final-backup-comprehensive-*.tar.gz     13 MB  (full dump including sessions)
```

Contains: SOUL.md, ARKAI.md, AGENTS.md, USER.md, TOOLS.md, MEMORY.md, all .openclaw config, session logs

### VPS Access
```bash
# Via Tailscale (WORKING)
ssh openclaw@100.66.171.78

# Via direct IP (WORKING after we fixed it)
ssh openclaw@65.21.54.211

# User: openclaw (UID 1000)
# Root login: enabled (temporarily for debugging)
```

### Docker Containers Status
```
NAME               STATUS
openclaw-gateway   Up (healthy after fixes)
openclaw-redis     Up (healthy)
openclaw-worker    Restarting (has issues, not critical)
openclaw-squid     Restarting (broken, bypassed)
```

---

## PART 2: CONFIGURATION CHANGES MADE

### docker-compose.yml Changes
1. **Fixed seccomp path** (other Claude)
2. **Changed `read_only: false`** for gateway (was causing EROFS errors)
3. **Changed `user: "1000:1000"`** (was 1001, mismatched host UID)
4. **Added `external` network** to gateway (for internet access)
5. **Added missing env vars:**
   ```yaml
   - TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}
   - OPENROUTER_API_KEY=${OPENROUTER_API_KEY}
   ```
6. **Removed HTTP_PROXY/HTTPS_PROXY** (Squid broken, gateway has direct access now)

### Config File Changes (`~/openclaw-secure-deployment/openclaw-data/config.json`)
```json
"dm": {
  "dmPolicy": "allowlist",           // Changed from "pairing"
  "allowFrom": ["732979045"],        // Added user's Telegram ID
  "requirePairingApproval": false    // Changed from true
}
```

### Volume Mounts
- Changed from single config file mount to full directory mount:
  ```yaml
  # OLD: ./config/openclaw-config.json:/home/node/.openclaw/config.json:ro
  # NEW: ./openclaw-data:/home/node/.openclaw:rw
  ```
- Config copied to `~/openclaw-secure-deployment/openclaw-data/config.json`

---

## PART 3: WHAT'S WORKING

| Component | Status | Notes |
|-----------|--------|-------|
| VPS SSH | ✅ Working | Both Tailscale and direct IP |
| Gateway container | ✅ Running | Starts, listens on ws://127.0.0.1:18789 |
| Redis | ✅ Healthy | Kill switch infrastructure ready |
| Telegram API connectivity | ✅ Working | `curl` to api.telegram.org succeeds |
| Bot token valid | ✅ Working | getMe returns `@arkai_clawdbot` (Clawdia) |
| Telegram provider starts | ✅ Working | Logs show "[telegram] starting provider" |

---

## PART 4: WHAT'S NOT WORKING

### Telegram Bot Not Responding to Messages

**Symptoms:**
- Bot shows "starting provider (@arkai_clawdbot)" in logs
- First message showed pairing prompt (so it DID receive that message)
- After config changes, no response to messages
- No incoming message logs visible

**Attempted Fixes:**
1. ✅ Added external network for internet access
2. ✅ Added TELEGRAM_BOT_TOKEN env var
3. ✅ Added user ID to allowFrom list
4. ✅ Changed dmPolicy from "pairing" to "allowlist"
5. ✅ Set requirePairingApproval to false
6. ✅ Multiple container restarts

**Possible Issues:**
1. Config might not be reloading properly (there's a "config watcher error: EPERM")
2. The `openclaw.json` runtime state might be overriding `config.json`
3. Telegram polling might not be starting (no "polling started" log visible)
4. There might be a permission issue preventing message processing

### Squid Proxy Broken
- Crashes with "ERROR: setgid: Operation not permitted"
- Not critical - bypassed by giving gateway direct internet access

### Worker Container Restarting
- Not investigated yet
- May be related to sandbox configuration

---

## PART 5: KEY FILE LOCATIONS ON VPS

```
/home/openclaw/openclaw-secure-deployment/
├── docker-compose.yml              # Main deployment config
├── .env                            # Secrets (tokens, keys)
├── openclaw-data/                  # Mounted as /home/node/.openclaw
│   ├── config.json                 # Main OpenClaw config
│   ├── openclaw.json               # Runtime state
│   ├── telegram/                   # Telegram state
│   └── agents/                     # Agent sessions
├── config/                         # Original config (not mounted anymore)
├── logs/                           # Log directories
├── workspace/                      # Input/output workspace
└── scripts/                        # Helper scripts
```

---

## PART 6: DEBUGGING COMMANDS

```bash
# SSH to VPS
ssh openclaw@100.66.171.78

# Check container status
cd ~/openclaw-secure-deployment
docker compose ps

# View gateway logs (live)
docker compose logs -f openclaw-gateway

# View recent logs
docker compose logs --tail 50 openclaw-gateway

# Check config inside container
docker compose exec openclaw-gateway cat /home/node/.openclaw/config.json | grep -A10 telegram

# Test Telegram API directly
docker compose exec openclaw-gateway sh -c 'curl -s https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMe'

# Restart gateway
docker compose restart openclaw-gateway

# Full restart
docker compose down && docker compose up -d
```

---

## PART 7: NEXT STEPS TO TRY

### Priority 1: Debug Telegram
1. Check if there's a separate "enabled plugins" state overriding config
2. Look at `/home/node/.openclaw/openclaw.json` for Telegram enabled state
3. Try running `openclaw doctor --fix` somehow (CLI not available in container)
4. Check if there's a webhook vs polling config issue
5. Look for any Telegram-specific error in full logs

### Priority 2: Fix EPERM Config Watcher
- The config watcher can't watch openclaw.json
- Might need different permissions or volume mount options

### Priority 3: Investigate Worker Container
- Check why it's restarting
- May need similar fixes to gateway

### Priority 4: Re-enable Security (Later)
- Once Telegram works, consider re-enabling:
  - read_only: true (with proper tmpfs mounts)
  - Squid proxy (with fixed permissions)
  - Seccomp profiles

---

## PART 8: SECURITY NOTES

### Current Security Posture (Relaxed for Debugging)
| Setting | Current | Target |
|---------|---------|--------|
| read_only | false | true |
| Squid proxy | disabled | enabled |
| Network | direct external | via proxy |
| root SSH | enabled | disabled |

### Secrets Location
- `.env` file at `~/openclaw-secure-deployment/.env`
- Contains: OPENCLAW_GATEWAY_TOKEN, OPENROUTER_API_KEY, TELEGRAM_BOT_TOKEN
- File permissions: should be 600

---

## PART 9: OTHER CLAUDE AGENT

There's another Claude (in Windsurf) that created the deployment package:
- Location: `~/Library/Application Support/Claude/local-agent-mode-sessions/.../outputs/openclaw-secure-deployment/`
- Files: DEPLOYMENT-GUIDE.md, docker-compose.yml, provision-vps.sh, config files
- That agent did the initial VPS provisioning

---

## PART 10: USER'S TELEGRAM ID

```
Telegram User ID: 732979045
Bot Username: @arkai_clawdbot
Bot Name: Clawdia
Bot ID: 8470717774
```

---

## PART 11: CONTINUATION INSTRUCTIONS

When resuming:
1. Read this handoff
2. SSH to VPS: `ssh openclaw@100.66.171.78`
3. Check current status: `cd ~/openclaw-secure-deployment && docker compose ps`
4. Check logs: `docker compose logs --tail 50 openclaw-gateway`
5. Focus on debugging why Telegram messages aren't being processed
6. The config changes are made, but something else is preventing message handling

**Key Question:** Why did the FIRST message (pairing prompt) work, but subsequent messages don't trigger any response?

---

*This handoff is authoritative for session continuity.*

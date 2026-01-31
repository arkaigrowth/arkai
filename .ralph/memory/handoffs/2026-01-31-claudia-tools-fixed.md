# Session Handoff: 2026-01-31 Claudia Tools Fixed

## Summary
Fixed Claudia's tool access issues, diagnosed model behavior, added comprehensive SOUL.md guidelines, and confirmed Kimi K2-0905 works when given explicit instructions.

## Root Cause Analysis

### Problem Chain
1. **Gateway crashed** → `tools.bash` config key deprecated, should be `tools.exec`
2. **Claudia fabricated ROADMAP.md** → Couldn't access file outside workspace, created fake one
3. **Workspace sandboxing** → `read` tool rewrites paths to stay within `~/clawd/`
4. **Inaccurate search results** → Model didn't know occurrence vs line counting

### The Real Fix
**It wasn't the model - it was the instructions.** Both Kimi and GLM-4.7 work when SOUL.md has explicit, unambiguous guidelines.

## Completed ✅

### 1. Gateway Config Fixed
- Removed deprecated `tools.bash` → use `tools.exec`
- Created `~/.clawdbot/exec-approvals.json` with command allowlist
- Exec approvals use separate file, not main config

### 2. Symlink Fix (Chad's Recommendation)
```bash
ln -s /home/clawdbot/arkai /home/clawdbot/clawd/arkai
```
- Claudia can now access arkai files via `~/clawd/arkai/`
- Updated ARKAI.md to reference symlinked path

### 3. SOUL.md Guardrails Added
- **Never fabricate content** - If can't access file, say so
- **Tool Use Policy (Mandatory)** - Must call tools, never guess
- **Search Guidelines** - Bounded searches, default exclusions
- **Occurrence vs Line counting** - `rg -oi | wc -l` for occurrences

### 4. Ripgrep Installed
```bash
ssh olek-admin@100.81.12.50 "sudo apt-get install -y ripgrep"
```
- Added `/usr/bin/rg` to exec-approvals allowlist
- Added `/usr/bin/head` for piping

### 5. VPS Synced
- Pulled latest to VPS: `cd ~/arkai && git pull origin main`
- 43 files updated including Security Hardening section

### 6. Model Testing
| Model | Result | Notes |
|-------|--------|-------|
| Kimi K2-0905 | ✅ Works | With explicit SOUL.md guidelines |
| GLM-4.7 | ✅ Works | Worked first try |
| Verdict | **Keep Kimi** | Cheaper, works with good instructions |

## Files Changed

### VPS (~/.clawdbot/, ~/clawd/)
- `~/.clawdbot/clawdbot.json` - Fixed tools config
- `~/.clawdbot/exec-approvals.json` - Command allowlist (created)
- `~/clawd/SOUL.md` - Added guardrails + search guidelines
- `~/clawd/ARKAI.md` - Updated to symlinked path
- `~/clawd/arkai` - Symlink to ~/arkai (created)
- Deleted: `~/clawd/ROADMAP.md` (fabricated file)

### exec-approvals.json
```json
{
  "exec": {
    "security": "allowlist",
    "ask": "on-miss"
  },
  "agents": {
    "default": {
      "allowlist": [
        "/usr/bin/git", "/usr/bin/ls", "/usr/bin/cat",
        "/usr/bin/head", "/usr/bin/tail", "/usr/bin/wc",
        "/usr/bin/find", "/usr/bin/grep", "/usr/bin/pwd",
        "/usr/bin/whoami", "/usr/bin/date", "/usr/bin/uptime",
        "/usr/bin/rg"
      ]
    }
  }
}
```

## Key Learnings

### 1. Workspace Sandboxing
The `read` tool rewrites paths to stay within workspace. Files outside `~/clawd/` need:
- Symlink into workspace, OR
- `allowedRoots` config (not tested), OR
- `exec` with `cat` command

### 2. Model Intelligence vs Instructions
Kimi K2-0905 isn't dumb - it needs explicit instructions. When it fails:
1. Add specific guideline to SOUL.md
2. Don't immediately switch models

### 3. ripgrep Counting
- `rg -c` = lines with matches
- `rg -oi | wc -l` = total occurrences (what VSCode Cmd+F shows)

### 4. Clawdbot Config Migrations
- `tools.bash` → deprecated, use `tools.exec`
- Exec restrictions go in separate `exec-approvals.json`
- Run `clawdbot doctor --fix` after config issues

## Research Files Created
- `.ralph/memory/research/kimi-k2-openclaw-research.md`
- `.ralph/memory/research/openrouter-tool-calling-research.md`
- `.ralph/memory/research/clawdbot-tool-logs.md`

## Current Model Config
```
Primary:  openrouter/moonshotai/kimi-k2-0905 ($0.39/$1.90 per M)
Fallback: openrouter/z-ai/glm-4.7 ($0.40/$1.50 per M)
```

## Commands Reference

```bash
# SSH to VPS
ssh clawdbot-vps

# Admin access (for apt-get, etc.)
ssh olek-admin@100.81.12.50

# Restart gateway
ssh clawdbot-vps "screen -S clawdbot -X quit; sleep 1; screen -dmS clawdbot bash -c 'source ~/.nvm/nvm.sh && clawdbot gateway 2>&1 | tee ~/gateway.log'"

# Sync VPS repos
ssh clawdbot-vps "cd ~/arkai && git pull origin main"

# Check gateway model
ssh clawdbot-vps "grep 'agent model' ~/gateway.log"

# Switch model (example to GLM)
# Edit ~/.clawdbot/clawdbot.json agents.defaults.model.primary
```

## Backlog (Not Completed)

### From Previous Session
- [ ] Slack integration
- [ ] Brave API fallback for web search
- [ ] Hook integration (auto-sanitize web results)

### Kimi K2.5 Status
- Available on OpenRouter ($0.50/M input)
- NOT supported in clawdbot 2026.1.24-3 (needs 2026.1.29+ beta)
- Open bugs: #4143 (null outputs), #3475 (silent failures)
- **Decision:** Stay on K2-0905 until bugs fixed

## Next Session: OpenClaw Ecosystem Exploration

User wants to explore OpenClaw ecosystem for:
1. **Reusable software** - Components we don't have to rewrite
2. **Security fuzzing** - Look for vulnerabilities
3. **Adapt/copy patterns** - Learn from existing implementations

Areas to explore:
- Skills repository
- Plugin architecture
- Security patterns
- Tool implementations

---

## Verification Checklist (for next session)

After `/new` with Claudia, test:
- [ ] `rg -oi 'security' ~/clawd/arkai/docs/ROADMAP.md | wc -l` → should return 17
- [ ] Ask her to read the roadmap → should work via symlink
- [ ] Web search test → verify still working

**Tell Claudia:**
> "You have ripgrep now. For counting occurrences, use `rg -oi 'pattern' file | wc -l`"

# Codex Spec: Fix OpenClaw Stalls

> **From**: arkai session (Claude Code, 2026-03-22)
> **To**: Codex (openclaw-local repo)
> **Priority**: High — blocking daily-driver AIOS
> **Repo**: `~/AI/openclaw-local/`

---

## Context

OpenClaw container is healthy (`docker ps`: Up 2 days, gateway :18789 live, Apple bridge :19789 healthy). But three subsystems are stalled:

1. **Email daemon** — Last file written Feb 9 (6 weeks ago). 182 stale files in `workspace/input/emails/`.
2. **Email classifier agent** — Session locks timing out, missing tool permissions.
3. **Daily digest** — `workspace/output/digests/` is completely empty. Generation stopped.

The infrastructure works. The agent configs are broken.

---

## Issue 1: Email Daemon Stopped (Host-Side)

**Location**: `~/AI/arkai-gmail/scripts/triage_daemon.py`
**Output dir**: `~/AI/openclaw-local/workspace/input/emails/`
**Last output**: `1644844097f7.json` (Feb 9, 2026 18:24)

### Investigation Steps
1. Check if the daemon is running: `ps aux | grep triage_daemon`
2. Check launchd status: `launchctl list | grep arkai` (or `gmail` or `triage`)
3. If launchd plist exists, check its logs: look at StandardErrorPath in the plist
4. Check if OAuth token expired: `ls -la ~/.arkai-gmail/token.json` — if stale, re-auth needed
5. Try a manual run: `cd ~/AI/arkai-gmail && .venv/bin/python3 scripts/triage_daemon.py --dry-run`

### Expected Fix
Either: restart the launchd job, re-authenticate OAuth, or fix a Python dependency issue. The daemon itself has 27 passing tests as of Feb 2026.

---

## Issue 2: Email Classifier Agent (OpenClaw-Side)

**Location**: `~/AI/openclaw-local/data/openclaw.json` — look for email-classifier session config
**Symptoms**: Session lock timeouts, missing filesystem-read tool permission

### Investigation Steps
1. Check agent configs: `cat data/openclaw.json | jq '.sessions[] | select(.name | contains("email"))'`
2. Look for session lock files: `find workspace/ -name "*.lock" -o -name "*session*"`
3. Check OpenClaw logs: `docker logs openclaw-local --tail 100 2>&1 | grep -i "lock\|timeout\|permission\|email"`
4. Verify the email-classifier agent has `safe_fs_read_text` in its allowed tools
5. Check if workspace/input/emails/ is readable from inside the container: `docker exec openclaw-local ls /workspace/input/emails/ | head -5`

### Expected Fix
Add missing tool permission to the email-classifier agent config. Clear stale session locks. Restart the agent.

---

## Issue 3: Daily Digest Generation

**Location**: `workspace/output/digests/` (currently empty)
**Likely cause**: Depends on email-classifier output. If classifier is stalled, digest has no input.

### Investigation Steps
1. Check if digest generation is a scheduled task or triggered by classifier completion
2. Look for HEARTBEAT.md or digest config: `grep -r "digest" data/openclaw.json`
3. Check if there's a digest agent/session: `cat data/openclaw.json | jq '.sessions[] | select(.name | contains("digest"))'`
4. After fixing Issue 2, verify digest auto-generates

### Expected Fix
Fixing Issue 2 likely fixes this. If digest is a separate scheduled job, verify its cron/timer config.

---

## Issue 4: Apple Bridge E2E Verification

**Status**: Health check passes (`{"status":"healthy"}` on :19789)
**But**: Health != working. Need to verify actual tool execution.

### Verification Steps
1. Test Notes read: `curl -s http://127.0.0.1:19789/notes/list | head -20`
2. Test Notes create: `curl -s -X POST http://127.0.0.1:19789/notes/create -d '{"title":"test","body":"codex e2e test"}'`
3. Test Reminders list: `curl -s http://127.0.0.1:19789/reminders/list | head -20`
4. Verify from inside container: `docker exec openclaw-local curl http://host.docker.internal:19789/notes/list`
5. If any fail, check bridge logs and macOS permissions (Reminders needs NSRemindersUsageDescription)

### Known Issues (from arkai memory)
- `remindctl add` may silently fail (Issue #19) — verify reminder actually persists after create
- `imsg` is NOT installed — skip iMessage tests
- Bridge must bind `0.0.0.0` not `127.0.0.1` for Docker Desktop

---

## Acceptance Criteria

- [ ] Email daemon is running and producing new `{12-hex}.json` files
- [ ] Email classifier processes files without session lock errors
- [ ] At least one digest file appears in `workspace/output/digests/`
- [ ] `notes list` returns actual Apple Notes content via bridge
- [ ] `reminders list` returns actual Reminders content via bridge
- [ ] All fixes documented in a handoff file

---

## Files to Check

```
~/AI/openclaw-local/
├── data/openclaw.json          # Agent configs, sessions, tool permissions
├── workspace/input/emails/     # 182 stale files (newest Feb 9)
├── workspace/output/digests/   # EMPTY — should have digest files
├── workspace/output/queue/     # Task queue (check for stuck tasks)
└── docker-compose.yml          # Container config

~/AI/arkai-gmail/
├── scripts/triage_daemon.py    # Email daemon (Python)
├── .venv/                      # Virtual environment
└── src/arkai_gmail/            # Pipeline code (27 tests)

# launchd plists (check both locations)
~/Library/LaunchAgents/com.arkai.*.plist
/Library/LaunchAgents/com.arkai.*.plist
```

---

## Coordination

- arkai session is building semantic search in parallel
- Once both sides work, we wire `arkai search` as an OpenClaw CLI tool
- Don't modify arkai contracts — they're frozen
- Report back via `docs/ai/handoffs/2026-03-22-codex-openclaw-fix-results.md`

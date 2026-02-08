# Handoff: launchd Integration Spec for Email + Calendar Daemons

Date: 2026-02-08
From: Arkai Claude → Codex
Purpose: Exact specs for building launchd plists that run both daemons on schedule.

---

## Architecture Overview

Both daemons follow the same pattern (ADR-0002):
- **One-shot scripts**, NOT persistent processes
- Run via launchd on a timer
- Write JSON to `~/AI/openclaw-local/workspace/input/{emails,calendar}/`
- OpenClaw agent reads via `safe_fs_read_text`
- All OAuth tokens stay on host — never enter Docker container

---

## 1. Email Triage Daemon

### Plist: `com.arkai.email-triage.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.arkai.email-triage</string>

    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>-c</string>
        <string>
ANTHROPIC_API_KEY="$(security find-generic-password -s openclaw-anthropic -w)" \
GMAIL_OUTPUT_DIR="$HOME/AI/openclaw-local/workspace/input/emails" \
PYTHONPATH="$HOME/AI/arkai-gmail/src" \
"$HOME/AI/arkai-gmail/.venv/bin/python3" \
"$HOME/AI/arkai-gmail/scripts/triage_daemon.py"
        </string>
    </array>

    <key>StartInterval</key>
    <integer>600</integer>

    <key>StandardOutPath</key>
    <string>/tmp/arkai-email-triage.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/arkai-email-triage.stderr.log</string>

    <key>RunAtLoad</key>
    <false/>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin</string>
        <key>HOME</key>
        <string>/Users/alexkamysz</string>
    </dict>
</dict>
</plist>
```

### Email Daemon Details
| Property | Value |
|----------|-------|
| Script | `~/AI/arkai-gmail/scripts/triage_daemon.py` |
| Interpreter | `~/AI/arkai-gmail/.venv/bin/python3` |
| PYTHONPATH | `~/AI/arkai-gmail/src` (required — imports `arkai_gmail` package) |
| API key source | macOS Keychain: `security find-generic-password -s openclaw-anthropic -w` |
| Output dir | `~/AI/openclaw-local/workspace/input/emails/` |
| Interval | 600s (10 min) — each run takes 3-8s per email |
| Retention | 7 days (GMAIL_RETAIN_DAYS env, mtime-based) |
| Exit 0 | Success (even if 0 emails processed) |
| Exit 1 | Fatal (auth failure, missing API key) |
| Logging | All to stderr — stdout is clean |

### Optional email env vars
```
GMAIL_TRIAGE_LIMIT=10      # max emails per run (default 10)
GMAIL_RETAIN_DAYS=7         # delete output older than N days (default 7)
GMAIL_CONFIG_DIR=~/.arkai-gmail/  # OAuth token location (default)
```

---

## 2. Calendar Daemon

### Plist: `com.arkai.calendar-fetch.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.arkai.calendar-fetch</string>

    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>-c</string>
        <string>
GCAL_OUTPUT_DIR="$HOME/AI/openclaw-local/workspace/input/calendar" \
GCAL_CONFIG_DIR="$HOME/.arkai-calendar" \
"$HOME/AI/arkai-gmail/.venv/bin/python3" \
"$HOME/AI/arkai-gmail/scripts/calendar_daemon.py"
        </string>
    </array>

    <key>StartInterval</key>
    <integer>900</integer>

    <key>StandardOutPath</key>
    <string>/tmp/arkai-calendar-fetch.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/arkai-calendar-fetch.stderr.log</string>

    <key>RunAtLoad</key>
    <false/>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin</string>
        <key>HOME</key>
        <string>/Users/alexkamysz</string>
    </dict>
</dict>
</plist>
```

### Calendar Daemon Details
| Property | Value |
|----------|-------|
| Script | `~/AI/arkai-gmail/scripts/calendar_daemon.py` |
| Interpreter | `~/AI/arkai-gmail/.venv/bin/python3` |
| PYTHONPATH | NOT needed (no package imports, inline everything) |
| API key | NONE — uses OAuth only (calendar.readonly), no Anthropic key |
| Config dir | `~/.arkai-calendar/` (credentials.json + token.json) |
| Output dir | `~/AI/openclaw-local/workspace/input/calendar/` |
| Interval | 900s (15 min) — each run takes 1-3s (Google API only, no LLM) |
| Retention | 3 days past event end time (GCAL_RETAIN_DAYS, end-time-based) |
| Exit 0 | Success (even if 0 events) |
| Exit 1 | Fatal (auth failure, missing credentials) |
| Logging | All to stderr — stdout is clean |

### Optional calendar env vars
```
GCAL_LOOKAHEAD_DAYS=7       # how far ahead to fetch (default 7)
GCAL_EVENT_LIMIT=50         # max events per calendar (default 50)
GCAL_RETAIN_DAYS=3          # keep files N days past event end (default 3, -1 to disable)
GCAL_CALENDARS=             # comma-separated cal IDs (default: all enabled)
GCAL_SHOW_CANCELLED=0       # include cancelled events (default 0)
```

---

## 3. Installation Steps

```bash
# Install plists
cp com.arkai.email-triage.plist ~/Library/LaunchAgents/
cp com.arkai.calendar-fetch.plist ~/Library/LaunchAgents/

# Load (start scheduling)
launchctl load ~/Library/LaunchAgents/com.arkai.email-triage.plist
launchctl load ~/Library/LaunchAgents/com.arkai.calendar-fetch.plist

# Manual test run (before enabling schedule)
launchctl start com.arkai.email-triage
launchctl start com.arkai.calendar-fetch

# Check logs
tail -f /tmp/arkai-email-triage.stderr.log
tail -f /tmp/arkai-calendar-fetch.stderr.log

# Unload (stop scheduling)
launchctl unload ~/Library/LaunchAgents/com.arkai.email-triage.plist
launchctl unload ~/Library/LaunchAgents/com.arkai.calendar-fetch.plist
```

---

## 4. Prerequisites (Alex must complete before plists work)

### Email daemon
- [x] OAuth token at `~/.arkai-gmail/token.json` (already exists)
- [x] Anthropic key in Keychain as `openclaw-anthropic` (already exists)
- [x] Output dir exists (Docker Compose creates it)

### Calendar daemon
- [ ] Copy credentials: `cp ~/.arkai-gmail/credentials.json ~/.arkai-calendar/`
- [ ] Enable Google Calendar API in GCP console
- [ ] First auth (browser OAuth):
  ```bash
  mkdir -p ~/.arkai-calendar
  cp ~/.arkai-gmail/credentials.json ~/.arkai-calendar/
  ~/AI/arkai-gmail/.venv/bin/python3 -c "
  import sys; sys.path.insert(0, '$HOME/AI/arkai-gmail/scripts')
  from calendar_daemon import CalendarAuth
  from pathlib import Path
  CalendarAuth(Path.home() / '.arkai-calendar').authenticate()
  print('Done!')
  "
  ```
- [ ] Create output dir: `mkdir -p ~/AI/openclaw-local/workspace/input/calendar/`

---

## 5. Output File Layout (What the Agent Sees)

```
workspace/input/
├── emails/
│   ├── a1b2c3d4e5f6.json      # per-email triage result
│   ├── f7e8d9c0b1a2.json
│   └── ...
├── calendar/
│   ├── b888dc5501b5.json       # per-event JSON
│   ├── 4ac68b34ece5.json       # recurring instance
│   ├── f86802912772.json       # cancelled event
│   ├── upcoming_index.json     # sorted summary of all events
│   └── ...
```

The agent should:
1. Read `upcoming_index.json` first for a quick overview
2. Read individual `{id}.json` files for full event details
3. Check `status` field — cancelled events are explicitly marked
4. Use `calendar_name` to distinguish between calendars

---

## 6. Validation

Codex can validate daemon output against contracts:

```bash
# Email daemon output
python3 contracts/validate_daemon_output.py workspace/input/emails/*.json

# Calendar daemon output
python3 contracts/validate_calendar_output.py workspace/input/calendar/*.json

# With Pydantic round-trip (stricter)
~/AI/arkai-gmail/.venv/bin/python3 contracts/validate_calendar_output.py \
    --pydantic workspace/input/calendar/*.json
```

Fixtures for testing:
- `contracts/fixtures/calendar_daemon_sample.json` — standard confirmed event (PASS)
- `contracts/fixtures/calendar_daemon_recurring.json` — recurring event instance (PASS)
- `contracts/fixtures/calendar_daemon_cancelled.json` — cancelled event (PASS)
- `contracts/fixtures/calendar_daemon_invalid.json` — 6 intentional violations (FAIL)

---

## 7. Key Differences Between Daemons

| | Email | Calendar |
|---|---|---|
| LLM needed | Yes (Sonnet) | No |
| API key | ANTHROPIC_API_KEY | None |
| Latency | 3-8s/email | 1-3s total |
| Retention | mtime-based | end-time-based |
| Output | 1 file/email | 1 file/event + index |
| Schedule | 10 min | 15 min |
| PYTHONPATH | Required | Not needed |

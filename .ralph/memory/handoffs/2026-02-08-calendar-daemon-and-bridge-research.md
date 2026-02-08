# Session Handoff: Calendar Daemon + Apple CLI Bridge Research

Date: 2026-02-08
Session: Continued from compaction — research + calendar daemon implementation

## What Happened This Session

1. **Apple CLI Bridge research** — Three parallel research agents investigated Notes, iMessage, and remindctl CLIs
   - Published report: `~/AI/openclaw-local/docs/ai/research/2026-02-07-apple-cli-bridge-compatibility.md`
   - OC Claude tidied and committed the report to openclaw-local
   - Recommendation: Rewrite ~50 lines of bridge handlers, not find matching CLIs
   - macnotesapp (RhetTbull) replaces memo for Notes (Tahoe-confirmed, non-interactive)
   - imsg reading DEFERRED (Full Disk Access required)
   - remindctl syntax matches BUT has silent-fail bug (Issue #19)

2. **Email daemon validator script** — `contracts/validate_daemon_output.py`
   - Requested by Codex for OC compatibility checks
   - Two-tier: inline structure validation + optional Pydantic round-trip
   - Committed: `381da38` on `main`

3. **Negative fixture for email validator** — `contracts/fixtures/email_daemon_invalid.json`
   - 11 distinct contract violations, `.expected` reference file
   - Committed: `a5bf0f1` on `main`

4. **Calendar daemon** — `~/AI/arkai-gmail/scripts/calendar_daemon.py`
   - Per-event JSON + upcoming_index.json summary
   - Inline CalendarAuth (separate token at ~/.arkai-calendar/, calendar.readonly scope)
   - Multi-calendar aware (calendar_id + calendar_name fields)
   - End-time-based retention (not mtime), GCAL_RETAIN_DAYS default 3
   - Cancelled events explicitly marked (status: "cancelled")
   - No LLM needed — pure Google API, ~1-3s per run
   - All models validated: imports, serialization, round-trip, retention logic
   - Committed: `96688cb` on `feat/gmail-layer-a`

5. **Calendar contract + fixtures + validator**
   - `contracts/calendar_daemon_contract.json` — mirrors email contract pattern
   - `contracts/fixtures/calendar_daemon_sample.json` — positive fixture (PASS)
   - `contracts/fixtures/calendar_daemon_invalid.json` — negative fixture, 6 violations
   - `contracts/validate_calendar_output.py` — same two-tier pattern as email
   - Committed: `e93aed1` on `main`

## Codex Interaction Summary

- Codex requested: negative fixture for email validator (DONE)
- Codex requested: calendar daemon contract + fixtures + validator (DONE)
- Codex requested: calendar daemon implementation (DONE)
- Codex reminder: contract frozen at v1, no openclaw-local edits
- Codex reminder: track remindctl Issue #19 — do NOT integrate create until fix validated

## Setup Required Before Calendar Daemon Can Run

```bash
# 1. Create config dir + copy OAuth client
mkdir -p ~/.arkai-calendar
cp ~/.arkai-gmail/credentials.json ~/.arkai-calendar/

# 2. Enable Google Calendar API in GCP Console
# https://console.cloud.google.com/apis/library/calendar-json.googleapis.com

# 3. First auth (opens browser for calendar.readonly consent)
~/AI/arkai-gmail/.venv/bin/python3 -c "
from pathlib import Path; import sys; sys.path.insert(0, '$HOME/AI/arkai-gmail/scripts')
from calendar_daemon import CalendarAuth
CalendarAuth(Path.home() / '.arkai-calendar').authenticate()
print('Done!')
"
```

## Key Files This Session

### arkai repo (commits 381da38, a5bf0f1, e93aed1)
- `contracts/validate_daemon_output.py` (created)
- `contracts/fixtures/email_daemon_invalid.json` (created)
- `contracts/fixtures/email_daemon_invalid.expected` (created)
- `contracts/calendar_daemon_contract.json` (created)
- `contracts/fixtures/calendar_daemon_sample.json` (created)
- `contracts/fixtures/calendar_daemon_invalid.json` (created)
- `contracts/fixtures/calendar_daemon_invalid.expected` (created)
- `contracts/validate_calendar_output.py` (created)

### arkai-gmail repo (commit 96688cb on feat/gmail-layer-a)
- `scripts/calendar_daemon.py` (created)

### openclaw-local repo (committed by OC Claude)
- `docs/ai/research/2026-02-07-apple-cli-bridge-compatibility.md` (created)

## Pending / Next Steps

1. **Calendar daemon first auth** — Alex needs to run the setup steps above (browser OAuth)
2. **Calendar daemon E2E test** — run against real Google Calendar, verify output
3. **Codex: launchd plists** — Codex has everything needed for both email + calendar daemons
4. **Codex: wire agent to read calendar JSON** — same pattern as email (safe_fs_read_text)
5. **Bridge handler rewrites** — OC Claude or Codex implements the ~50 lines of Notes handler changes (macnotesapp)
6. **remindctl Issue #19** — track, do NOT integrate create until fix released + validated
7. **iMessage bridge** — deferred until Full Disk Access is granted

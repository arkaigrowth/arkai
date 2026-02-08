# Session Handoff: OpenClaw Email Integration Planning

Date: 2026-02-07
Session: Security audit continuation + cross-agent coordination

## What Happened This Session

1. **Verified Codex's openclaw-local work** — all 10 claims checked out. Container healthy, security audit clean, model split working, bridge auth working.

2. **Discovered arkai-gmail is complete** — full 5-layer email pipeline at ~/AI/arkai-gmail/. Reader (Sonnet), Critic (10 rules), Actor (dry-run default). 27 tests, all passing. DO NOT REBUILD.

3. **Apple CLI bridge interface mismatches found**:
   - remindctl: interface matches, ready to install
   - memo: bridge calls `memo list`/`show`/`new`, real tool uses `memo notes`/`notes -s`/`notes -a`
   - imsg: bridge calls `imsg read --contact`, real tool uses `imsg history --chat-id`. Also requires Full Disk Access. Deferred.

4. **Cross-agent coordination system established**:
   - openclaw-local's `docs/ai/` protocol (research, decisions, handoffs)
   - ADR-0001: cross-repo coordination pattern (canonical+pointer)
   - ADR-0002: email triage integration — Option C (daemon, classify-on-host, results-to-container)
   - Templates created for research/ADR/handoff reports
   - arkai-gmail research report + API surface doc committed to openclaw-local

5. **Sibling Repo section added to .claude/CLAUDE.md** — describes openclaw-local architecture, integration boundary, ADR pointers.

## Immediate Next Steps (Next Session)

1. **Write daemon wrapper script** — `~/AI/arkai-gmail/scripts/triage_daemon.py`
   - Imports Python API (GmailIngester → PreAIGate → ReaderLLM → Critic)
   - Writes one JSON per email to configurable output dir
   - Uses Pydantic .model_dump_json()

2. **Publish wrapper contract** — entrypoint, env vars, output schema, failure semantics
   - Codex is waiting on this before building the launchd plist

3. **Reduce OAuth scope** to gmail.readonly for classify-only

4. **Test remindctl E2E** — Codex should be doing this now, verify results

## Key Files Modified This Session

- `.claude/CLAUDE.md` — added Sibling Repo section + ADR pointers + arkai-gmail awareness
- `~/AI/openclaw-local/docs/ai/research/2026-02-07-arkai-gmail-pipeline-assessment.md` (created)
- `~/AI/openclaw-local/docs/ai/research/2026-02-07-arkai-gmail-api-surface.md` (created)
- `~/AI/openclaw-local/docs/ai/handoffs/2026-02-07-arkai-agent-daemon-wrapper.md` (created)

## Cross-Repo Pointers

- ADR-0001: `~/AI/openclaw-local/docs/ai/decisions/0001-cross-repo-coordination-pattern.md`
- ADR-0002: `~/AI/openclaw-local/docs/ai/decisions/0002-email-triage-integration-pattern.md`
- LinkedIn inbox is SEPARATE from Gmail triage — different feature, different scope

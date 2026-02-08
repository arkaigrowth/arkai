# Handoff: OpenClaw Local Security Audit + Codex Coordination

**Date**: 2026-02-06
**Session**: Claude Opus 4.6 (security audit + advisory)
**Parallel Session**: Codex (building MVP in ~/AI/openclaw-local/)

---

## What Was Done This Session

### 1. Comprehensive Security Audit of ~/AI/openclaw-local/

Ran 3 parallel audit agents (security-auditor, explore x2) against the full repo.

**Container hardening: 9/10** — read-only FS, cap_drop ALL, non-root, no-new-privileges, tmpfs with noexec, localhost-only port binding.

**Findings addressed**:
| Finding | Severity | Status |
|---------|----------|--------|
| Apple CLI Bridge bound to 0.0.0.0 with zero auth | CRITICAL | Fixed by Codex (IP allowlist + bearer token) |
| Gateway token in plaintext in config/.env | MEDIUM | Fixed by us (replaced with LOADED_FROM_KEYCHAIN) |
| Logs in /tmp world-readable | MEDIUM | Fixed by Codex (moved to ~/Library/Logs/openclaw/, 700) |
| No request body size limit | MEDIUM | Fixed by Codex (MAX_BODY_BYTES=1MB, 413 before read) |
| Exception messages leak internals | MEDIUM | Fixed by Codex (_send_internal_error returns generic msg) |
| No pids_limit | MEDIUM | Not yet fixed |
| No .dockerignore | MEDIUM | Not yet fixed |
| Base images not pinned to SHA256 | MEDIUM | Not yet fixed |

### 2. Reviewed & Approved Codex's MVP Plan (Twice)

**First review**: Flagged hallucinated model ID, wrong plugin format, overscoped RPC rewrite.
**Second review (revised plan)**: Approved with 2 minor notes. Codex had verified plugin format inside container.
**Key correction**: I was wrong about model ID — `anthropic/claude-opus-4-5-20251101` IS real in OpenClaw's catalog; `claude-opus-4-6` is NOT available yet.

**Approved MVP scope**:
- Brave Search via Keychain (user added BRAVE_API_KEY)
- Apple CLI Bridge hardened (keep REST, add auth + IP allowlist + absolute paths + write gate)
- Email file-drop (read-only .eml input, draft output)
- Sonnet-can't-web / Opus-subagent-can-web safety boundary
- OpenClaw plugin/extension for bridge access
- Gmail OAuth stubbed as design doc only

### 3. Investigated arkai ↔ openclaw-local Integration

**Current state**: Barely connected. Only link is Telegram for voice memos (one-way).
**Missing**: Task queue, result delivery, bidirectional file sync, unified memory.
**Not blocking MVP** — integration comes after local deployment works.

### 4. Verified Codex's Implementation (7 files, +91/-24)

All 3 medium security fixes verified correct. Version updated to 2026.2.3-1. LaunchAgent configured for bridge auto-start with proper umask. WEB-SUBAGENT-PROMPT.md added for hallucination-resistant web research.

---

## Current State of ~/AI/openclaw-local/

### Working
- Docker container running OpenClaw 2026.2.3-1 (healthy)
- Apple CLI Bridge live (PID managed by launchd, port 19789)
- Bridge auth: bearer token (Keychain-backed) + IP allowlist
- Bridge writes disabled by default (ALLOW_APPLE_WRITES=0)
- All secrets in macOS Keychain (3 entries): openclaw-anthropic, openclaw-gateway-token, openclaw-apple-bridge-token
- BRAVE_API_KEY added to Keychain by user
- `openclaw security audit`: 0 critical findings
- Logs private at ~/Library/Logs/openclaw/

### Not Yet Built
- `extensions/apple-bridge/` plugin (Codex building this)
- Email file-drop directories and docs
- Opus subagent web search config wiring
- Brave Search integration into container env
- Apple CLI tools not installed (memo, imsg, remindctl) — bridge correctly returns "Missing executable"

### Not Yet Fixed (Low Priority)
- No pids_limit in docker-compose.yml
- No .dockerignore
- Base images not pinned to SHA256 digest
- No rate limiting on bridge (accepted for MVP)

---

## Key Files Modified This Session

| File | What Changed |
|------|-------------|
| `config/.env` | Removed plaintext gateway token, replaced with LOADED_FROM_KEYCHAIN placeholder |
| `scripts/apple-cli-bridge.py` | Codex: added IP allowlist, bearer auth, body size limit, error sanitization, write gate, absolute binary paths |
| `scripts/start-apple-bridge.sh` | Codex: Keychain-backed token injection, umask 077, Docker-compatible bind |
| `~/Library/LaunchAgents/ai.openclaw.apple-cli-bridge.plist` | Codex: logs moved to ~/Library/Logs/openclaw/, umask 63 |
| `docker-compose.yml` | Codex: APPLE_CLI_BRIDGE_TOKEN env, version bump |
| `docs/WEB-SUBAGENT-PROMPT.md` | Codex: hallucination-resistant web subagent template |
| `docs/CLI-BRIDGE.md` | Codex: updated bridge security docs |

---

## Key Learnings

1. **Docker Desktop macOS networking**: Bridge MUST bind 0.0.0.0 (not 127.0.0.1) because container reaches host via `host.docker.internal` which arrives from 192.168.65.0/24. IP allowlist is the correct mitigation.

2. **OpenClaw model catalog**: Does NOT include claude-opus-4-6. Opus 4.5 (`anthropic/claude-opus-4-5-20251101`) is the latest available. Discovery-based selection is the right pattern.

3. **OpenClaw plugin format**: `openclaw.plugin.json` + `package.json` with `openclaw.extensions` field + `index.ts`. Verified against 32 bundled extensions in container. Tool plugins use `api.registerTool()` with `{ optional: true }`.

4. **Web subagent hallucination**: When agents summarize web search results, they fill in plausible facts from training data. JSON output contract with `verified_facts` + `unknowns` fields forces auditability.

---

## Keychain Entries (Complete List)

| Entry | Purpose | Added By |
|-------|---------|----------|
| `openclaw-anthropic` | Anthropic API key | Previous session |
| `openclaw-gateway-token` | Gateway auth token | Previous session |
| `openclaw-apple-bridge-token` | Bridge bearer token | Codex |
| `openclaw-brave` | Brave Search API key | User (manual) |

---

## Decisions Made

- **Approved**: Keep REST endpoints (no RPC rewrite)
- **Approved**: 0.0.0.0 bind + IP allowlist (Docker Desktop requirement)
- **Approved**: ALLOW_APPLE_WRITES=0 default (double-lock on writes)
- **Approved**: Sonnet no-web / Opus subagent web-only boundary
- **Approved**: Plugin approach for bridge (not web_fetch, which has SSRF guard)
- **Corrected**: My earlier recommendation to bind 127.0.0.1 was wrong for Docker Desktop

---

## Next Steps (Post-Compaction)

1. **Review Codex's latest work** — he's building the extensions/apple-bridge plugin and wiring Brave Search
2. **Decide on fuzzing/red-teaming** — bridge auth is testable now (valid/invalid tokens, oversized payloads, unknown endpoints). Apple tool calls will fail gracefully (CLIs not installed)
3. **Consider**: Install Apple CLI tools (memo, imsg, remindctl) — Codex correctly refused to install unverified packages; need to research safe options
4. **Low-priority fixes**: pids_limit, .dockerignore, image pinning

---

## Reference Paths

- **Local repo**: `~/AI/openclaw-local/`
- **Backup**: `~/AI/clawdbot-backups/vps-backup-20260201-clean.tar.gz`
- **VPS secure deployment**: `~/AI/openclaw-secure-deployment/`
- **Container**: `openclaw-local` (docker, port 127.0.0.1:18789)
- **Bridge**: launchd-managed, port 19789
- **Bridge logs**: `~/Library/Logs/openclaw/`
- **Keychain**: login.keychain-db (4 openclaw-* entries)

# Handoff: Search Bug Fixes + Module C Minimal (today/done/snooze)

Date: 2026-03-23
From: Claude Code
To: Any session (Claude Code or Codex)
Supersedes: none

## What was done

### Bug fixes (triggered by post-chunking Codex testing)

1. **FTS5 query crash** — Date-like text ("2026-03-15") caused `Error: no such column: 03`.
   - Root cause: raw user text passed directly to FTS5 MATCH, which has its own operator syntax.
   - Fix: `sanitize_fts_query()` wraps all queries in FTS5 phrase quotes (`"..."`), escaping internal double-quotes. Applied in `fts_search()` (search.rs) and `search_items()` (queries.rs).
   - Tradeoff: FTS5 now does exact-phrase matching only. Multi-word non-contiguous keywords won't match via FTS. This is acceptable — FTS is a precision boost inside hybrid search, and vector/cosine carries recall.

2. **Reranking regression** — Chunk matches burying exact title hits.
   - Root cause (3 compounding bugs in `multi_level_search()`):
     a. Item's original hybrid score was discarded and replaced by positional RRF.
     b. Chunk results iterated from HashMap (nondeterministic rank order).
     c. Chunk boost was on same scale as item-level score.
   - Fix: Preserve original hybrid score, sort chunks by score DESC then item_id ASC (deterministic), scale chunk boost by `CHUNK_WEIGHT = 0.5`.

### Module C minimal — triage commands

Shipped `arkai today`, `arkai today --json`, `arkai done`, `arkai snooze`.

- **today**: Groups active captures by horizon (Do Today / Heads Up / Inbox count), sorts by priority then due_date, caps at 5 per section, shame-free wording. JSON output for OpenClaw/scripting.
- **done/snooze**: Accept full ID or unique prefix (as displayed by `arkai today`). Ambiguous prefix returns clear error with candidates. Nonexistent prefix returns clear warning.
- **snooze parsing**: YYYY-MM-DD (interpreted in local timezone, stored as UTC) or RFC3339. Natural-language input ("tomorrow 9am") is NOT implemented.
- **Query layer**: `list_active_captures()` uses COALESCE for missing metadata fields. `update_capture_status()` uses `json_set()` in-place (no fetch roundtrip). `resolve_capture_id()` for prefix matching.

## What is NOT done

- **`arkai triage` interactive** (C2) — Deferred. Build after today/done/snooze prove useful.
- **OpenClaw capture sync** (B4) — No `--sync-openclaw` flag, no `capture-inbox.jsonl` write, no `arkai_capture` bridge tool.
- **Eval framework** (D1) — Not started.
- **Proactive connections** (E1, E2) — Not started.
- **Natural-language snooze** — Only YYYY-MM-DD and RFC3339 accepted.

## Files changed

| File | Change |
|------|--------|
| `src/store/search.rs` | `sanitize_fts_query()`, `CHUNK_WEIGHT`, fixed `multi_level_search()` scoring |
| `src/store/queries.rs` | FTS sanitization in `search_items()`, `list_active_captures()`, `count_snoozed_captures()`, `update_capture_status()`, `resolve_capture_id()` |
| `src/cli/triage.rs` | NEW — `execute_today()`, `execute_done()`, `execute_snooze()`, `parse_snooze_until()`, `truncate_chars()` (Unicode-safe) |
| `src/cli/mod.rs` | Added `pub mod triage`, Today/Done/Snooze command variants + dispatch |
| `docs/ai/ARKAI_V1_BUILD_SPEC.md` | Updated build status, acceptance criteria, snooze docs |

## Test count

238 tests passing, 0 failures, 2 ignored (unchanged doctests).

## Store state

- 20 items (14 content, 6 capture), 20/20 embedded
- 153 chunks, 153/153 chunk-embedded
- Schema v3 (no migration changes in this pass)

## Next priorities (suggested)

1. Build eval framework (D1) — Codex requested, measures search quality objectively
2. OpenClaw capture sync (B4) — Makes captures visible to OpenClaw agent
3. Interactive triage (C2) — After today/done/snooze are proven useful
4. Natural-language snooze parsing — "tomorrow 9am", "next monday"

# Codex Next Priorities (from arkai session, 2026-03-22)

> **Context**: arkai semantic search is LIVE. 14 items in SQLite store, all embedded
> with mxbai-embed-large (1024d). Binary at `~/AI/arkai/target/release/arkai`.
> Tested and working: `arkai search --semantic "query"`, `arkai store status`.

---

## Priority 1: Wire arkai as a First-Class OpenClaw Tool

`scripts/arkai_phase1.py` exists but is NOT exposed as an OpenClaw tool yet.
This is the #1 gap — agents can't call arkai search.

### What to do

Make OpenClaw agents able to call these arkai commands:

```bash
# These all work RIGHT NOW from the host shell:
~/AI/arkai/target/release/arkai search --semantic "cold outreach pricing"
~/AI/arkai/target/release/arkai search "pricing"          # keyword fallback
~/AI/arkai/target/release/arkai store status               # 14 items, 14 embeddings
~/AI/arkai/target/release/arkai store import --library --embed  # re-import + embed
```

### Implementation options (pick whichever fits OpenClaw's tool model)

**Option A — Shell tool**: Add `arkai_search` as a shell-based tool that agents can invoke.
The agent passes a query string, gets back ranked results.

**Option B — Extend arkai_phase1.py**: Add a `search` subcommand to the existing wrapper:
```bash
python3 scripts/arkai_phase1.py search "query here"
# → calls arkai search --semantic, captures output, returns to agent
```

**Option C — HTTP endpoint**: If OpenClaw prefers HTTP tools, we can build `arkai serve`
later. But for now, shell is faster to wire.

### Acceptance criteria
- [ ] An OpenClaw agent can call arkai search and get results in its context
- [ ] Test: ask the main agent "search my library for videos about AI pricing"
- [ ] The tool returns structured results (not raw terminal output)

---

## Priority 2: Test Capture Inbox End-to-End

`scripts/capture_inbox.py` is built. Verify:

1. Manual test: write a capture entry, verify it appears in `workspace/output/memory/capture-inbox.jsonl`
2. Apple Notes bridge test: create a note via bridge, verify it's capturable
3. Confirm the JSONL format is compatible with arkai's future ingest path

### Why this matters
This is the ADHD capture path: quick thought → captured → searchable later.
It doesn't need to be wired to arkai yet — just verify it works reliably.

---

## Priority 3: Classifier Output Quality (Low Priority)

The malformed historical JSONs in email-classified/ are handled safely (worker skips them).
Don't spend time fixing historical data. Focus on:
- Ensuring NEW classifier output is well-formed
- If the classifier is still producing bad JSON, fix the prompt/output parsing

---

## What NOT to do

- Don't build transcript chunking/embedding — arkai session is handling this
- Don't modify arkai contracts or store schema — they're stable
- Don't build arkai serve (HTTP) yet — shell tool is sufficient for Phase 1
- Don't re-run the full email backlog scan — the bounded worker handles it

---

## Coordination

- arkai session is building transcript-level embeddings (full content, not just titles)
- arkai binary is pre-built at `~/AI/arkai/target/release/arkai`
- Store at `~/.arkai/store.db` — 14 items, 14 embeddings, schema v2
- Once arkai is an OpenClaw tool, agents can: search library, check store status, trigger re-import

# Arkai v1 Build Spec: ADHD-Optimized Life ERP

> **Purpose**: Comprehensive build spec for transforming arkai from a content pipeline
> into a daily-driver personal ERP. Designed to be executed by Claude Code sessions
> and Codex agents without additional prompting.
>
> **Date**: 2026-03-22
> **Status**: APPROVED FOR BUILD
> **Prereqs**: SQLite store (done), FTS5 search (done), embeddings (done), hybrid search (done)
> **Repo**: `~/AI/arkai/`
> **Related**: `~/AI/iron-ledger-atc/` (ADHD patterns), `~/AI/openclaw-local/` (runtime)

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Module A: Transcript Chunking](#2-module-a-transcript-chunking)
3. [Module B: Capture System](#3-module-b-capture-system)
4. [Module C: Triage & Surfacing](#4-module-c-triage--surfacing)
5. [Module D: Retrieval Eval Framework](#5-module-d-retrieval-eval-framework)
6. [Module E: Proactive Connections](#6-module-e-proactive-connections)
7. [Integration Map](#7-integration-map)
8. [Build Order & Dependencies](#8-build-order--dependencies)
9. [Acceptance Criteria](#9-acceptance-criteria)
10. [Patterns Borrowed from iron-ledger-atc](#10-patterns-borrowed-from-iron-ledger-atc)

---

## 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│  USER INPUT                                                      │
│  • Telegram message → Claudia/OpenClaw                           │
│  • Terminal: arkai capture "thought"                              │
│  • Voice memo → transcript → arkai                               │
│  • YouTube URL → ingest pipeline                                 │
│  • Apple Reminders (synced via bridge)                            │
└────────┬────────────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────────────┐
│  ARKAI STORE (SQLite, ~/.arkai/store.db)                         │
│                                                                   │
│  items          ← content, emails, captures, voice memos          │
│  chunks         ← transcript segments (sentence-group split)      │
│  entities       ← people, orgs, concepts (cross-content linked)   │
│  evidence       ← grounded claims with SHA256 provenance          │
│  embeddings     ← item-level vectors (mxbai-embed-large, 1024d)   │
│  chunk_embeds   ← chunk-level vectors for deep transcript search  │
│  items_fts      ← FTS5 BM25 keyword index                        │
│  store_config   ← embedding model, provider, dimensions           │
└────────┬────────────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────────────┐
│  SEARCH & RETRIEVAL                                               │
│  • Hybrid: FTS5 (BM25) + Vector (cosine) + RRF merge             │
│  • Multi-level: item embeddings + chunk embeddings                │
│  • Results include: item title, chunk text, provenance            │
└────────┬────────────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────────────┐
│  TRIAGE & SURFACING                                               │
│  • arkai today → what needs attention                             │
│  • arkai triage → classify uncategorized captures                 │
│  • Daily briefing (OpenClaw) → Do Today / Heads Up / Health       │
│  • Proactive: "You captured X, related to video Y"                │
└─────────────────────────────────────────────────────────────────┘
```

### Design Principles (from iron-ledger-atc)

- **Defaults > questions**: Pre-classify captures, don't ask the user to categorize
- **Deterministic > arbitrary**: Same inputs produce same triage/surfacing
- **Shame-free**: Rollovers are normal, snooze is not failure
- **Low-friction capture**: 2 seconds to capture, 0 seconds to classify
- **Provenance always**: Every surfaced fact is traceable to source

---

## 2. Module A: Transcript Chunking

### Purpose
Embed full transcript content so searches like "that story about the pizza shop owner"
find the right video even when the title doesn't mention it.

### Schema (Migration 003)

```sql
CREATE TABLE chunks (
    id          TEXT PRIMARY KEY,     -- SHA256(item_id + chunk_index)[0:16]
    item_id     TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    text        TEXT NOT NULL,
    byte_start  INTEGER NOT NULL,     -- offset in original transcript
    byte_end    INTEGER NOT NULL,
    word_count  INTEGER NOT NULL,
    metadata    TEXT DEFAULT '{}',     -- JSON: source_file, timestamp hints
    UNIQUE(item_id, chunk_index)
);
CREATE INDEX idx_chunks_item ON chunks(item_id);

CREATE TABLE chunk_embeddings (
    chunk_id   TEXT PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
    model      TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    vector     BLOB NOT NULL,          -- f32 little-endian
    created_at TEXT NOT NULL
);
```

### Chunking Strategy

**Algorithm**: Sentence-group splitting (SOTA per FloTorch Feb 2026 benchmark, 69% accuracy)

```
Input: continuous transcript text (no paragraph breaks)

1. Split on sentence boundaries: ". " | "? " | "! " | ".\n"
   (Handle edge cases: "Dr. ", "U.S.", numbers like "3.14")

2. Group sentences until chunk reaches TARGET_WORDS (400)
   - MIN_WORDS: 200 (merge small trailing chunks with previous)
   - MAX_WORDS: 600 (hard split at word boundary if needed)

3. Overlap: prepend last sentence of previous chunk (~15-30 words)
   - Just enough for context recovery, not 20% redundancy

4. Record byte_start and byte_end (byte offsets in original file)

5. Compute chunk ID: SHA256(item_id + chunk_index)[0:16]
```

**Strategy pattern** (for different content types):

```rust
pub enum ChunkStrategy {
    /// For transcripts, articles — sentence-group splitting
    SentenceGroup {
        target_words: usize,  // 400
        min_words: usize,     // 200
        max_words: usize,     // 600
    },
    /// For emails, notes, claims — embed as-is, no splitting
    WholeDocument,
}

impl ChunkStrategy {
    pub fn for_item_type(item_type: &str, word_count: usize) -> Self {
        match item_type {
            _ if word_count < 500 => Self::WholeDocument,
            "content" => Self::SentenceGroup {
                target_words: 400, min_words: 200, max_words: 600,
            },
            _ => Self::WholeDocument,
        }
    }
}
```

### Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/store/chunking.rs` | CREATE | Sentence-group chunker + ChunkStrategy enum |
| `src/store/migrations.rs` | MODIFY | Add Migration 003 |
| `src/store/queries.rs` | MODIFY | Add chunk CRUD + chunk_embeddings CRUD |
| `src/store/search.rs` | MODIFY | Search chunks alongside items, RRF merge |
| `src/store/mod.rs` | MODIFY | Declare chunking module |
| `src/cli/mod.rs` | MODIFY | Update `store import --embed` to chunk + embed transcripts |

### Tests (chunking.rs)

- `test_sentence_split_basic` — "Hello world. How are you? Fine." → 3 sentences
- `test_sentence_split_abbreviations` — "Dr. Smith went to U.S. embassy." → 1 sentence
- `test_sentence_split_numbers` — "The cost is $3.14 per unit." → 1 sentence
- `test_group_sentences_target_words` — groups until ~400 words
- `test_group_merges_small_trailing` — last chunk <200 words merges with previous
- `test_chunk_overlap` — last sentence of chunk N appears at start of chunk N+1
- `test_whole_document_strategy` — short text returns single chunk
- `test_strategy_selection` — content >500 words gets SentenceGroup, short gets WholeDocument
- `test_byte_offsets_correct` — byte_start/byte_end map back to original text
- `test_chunk_ids_deterministic` — same input → same IDs

### Done When

- [ ] 12 library transcripts are chunked (~250 chunks total)
- [ ] All chunks have embeddings
- [ ] `arkai search --semantic "that story about..."` returns chunk text, not just title
- [ ] Search results show: item title + best matching chunk snippet

---

## 3. Module B: Capture System

### Purpose
Low-friction capture of thoughts, reminders, and todos. 2 seconds to capture,
0 seconds to classify.

### CLI Commands

```bash
# Quick capture (defaults to kind=note)
arkai capture "call dentist about insurance"

# With kind
arkai capture --kind reminder "call dentist about insurance"
arkai capture --kind todo "review PR for client X"
arkai capture --kind link "https://interesting-article.com"

# With tags
arkai capture "meeting notes from standup" --tag work --tag standup

# With due date (for reminders)
arkai capture --kind reminder --due "2026-03-25" "submit tax documents"
```

### Data Model

Captures are stored as items in the existing `items` table with `item_type = "capture"`.

```rust
// In UpsertItem, metadata JSON contains:
{
    "kind": "note|reminder|todo|link|voice-memo|reference",
    "due_date": "2026-03-25",           // optional, ISO date
    "source": "cli|telegram|voice|openclaw",
    "horizon": "now|week|later",        // auto-classified or explicit
    "priority": "must|should|could",    // auto-classified or explicit
    "status": "inbox|triaged|done|snoozed",
    "snoozed_until": "2026-03-24T09:00:00Z",  // optional
    "captured_at": "2026-03-22T05:30:00Z"
}
```

### Auto-Classification (no user friction)

On capture, automatically:
1. If text contains a URL → `kind: "link"`
2. If text contains time words ("tomorrow", "next week", date patterns) → `kind: "reminder"`, extract `due_date`
3. If text starts with verb ("call", "buy", "review", "send") → `kind: "todo"`, `horizon: "now"`
4. Otherwise → `kind: "note"`, `horizon: "later"`
5. Always: embed immediately, add to FTS index

### Integration with OpenClaw Capture Inbox

When `--sync-openclaw` flag is passed (or configured as default):
1. Also write entry to `~/AI/openclaw-local/workspace/output/memory/capture-inbox.jsonl`
2. Match the schema from `scripts/capture_inbox.py` (schema_version, id, kind, title, text, tags, fanout)
3. Set fanout flags based on kind:
   - `reminder` → `fanout.appleReminders: "requested"`
   - all → `fanout.arkaiIngest: "requested"`

### Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/cli/capture.rs` | CREATE | Capture CLI subcommand |
| `src/store/capture.rs` | CREATE | Auto-classification logic + store integration |
| `src/cli/mod.rs` | MODIFY | Add Capture command |

### Tests

- `test_auto_classify_url` — "https://example.com" → kind=link
- `test_auto_classify_reminder` — "tomorrow call dentist" → kind=reminder, due_date=tomorrow
- `test_auto_classify_todo` — "review the PR" → kind=todo
- `test_auto_classify_note` — "interesting thought about AI" → kind=note
- `test_capture_creates_item_in_store` — item exists after capture
- `test_capture_embeds_immediately` — embedding exists after capture
- `test_capture_with_tags` — tags stored correctly
- `test_capture_searchable` — captured text found via semantic search

### Done When

- [ ] `arkai capture "text"` creates item + embedding in <500ms
- [ ] Captured items appear in `arkai search --semantic` results
- [ ] Auto-classification works for URLs, reminders, todos, notes
- [ ] `arkai store status` shows capture count

---

## 4. Module C: Triage & Surfacing

### Purpose
Help user stay on top of captures without overwhelm. Inspired by iron-ledger-atc's
horizon/MoSCoW/snooze patterns.

### CLI Commands (shipped)

```bash
# What needs attention today
arkai today
# Output:
#   Today -- 4 active  (1 snoozed)
#
#   Do Today (2)
#     [35f90d8] review the v1 build spec with Alex     should
#     [b0eba79] submit tax documents  due 2026-03-25    must
#
#   Heads Up (1)
#     [81e8eb0] interesting link about RAG               could
#
#   2 in inbox

# Machine-readable output for OpenClaw / scripting
arkai today --json

# Mark done (accepts full ID or unique prefix as shown by today)
arkai done <id-or-prefix>

# Snooze a capture (accepts full ID or unique prefix)
# Supported formats: YYYY-MM-DD (local timezone) or RFC3339
# Natural-language ("tomorrow 9am") is NOT yet supported.
arkai snooze <id-or-prefix> --until 2026-04-01
arkai snooze <id-or-prefix> --until 2026-04-01T09:00:00+00:00
```

### Deferred: Interactive Triage

`arkai triage` (interactive inbox classification with n/w/l/z/d/s/q keybindings)
is deferred until today/done/snooze prove useful in daily use.

### Triage Model (borrowed from iron-ledger-atc)

```
horizon:  NOW | WEEK | LATER       (when should this be done?)
priority: MUST | SHOULD | COULD    (how important is it?)
status:   INBOX | TRIAGED | DONE | SNOOZED

Auto-triage rules:
- Has due_date within 48h → horizon=NOW, priority=MUST
- Has due_date within 7d → horizon=WEEK, priority=SHOULD
- kind=reminder → horizon=NOW (unless snoozed)
- kind=todo → horizon=WEEK (default)
- kind=note → horizon=LATER
- kind=link → horizon=LATER
```

### Surfacing Logic (`arkai today`) — shipped

```
1. list_active_captures(): status != done AND (status != snoozed OR snooze expired)
   Uses COALESCE for missing metadata fields (resilient to old captures).
2. Group by metadata.horizon:
   NOW  → "Do Today" section (sorted by: priority weight ASC, due_date ASC)
   WEEK → "Heads Up" section (same sort)
   LATER / missing → counted as "inbox" (not displayed individually)
3. Cap at 5 items per section (reduce overwhelm)
4. Show snooze count in header: "Today -- N active  (M snoozed)"
```

### Files (shipped)

| File | Status | Description |
|------|--------|-------------|
| `src/cli/triage.rs` | SHIPPED | today/done/snooze handlers, parse_snooze_until, truncate_chars |
| `src/store/queries.rs` | SHIPPED | list_active_captures, update_capture_status, resolve_capture_id, count_snoozed_captures |
| `src/cli/mod.rs` | SHIPPED | Today, Done, Snooze command variants + dispatch |
| `src/store/migrations.rs` | No change needed — uses items.metadata JSON |

### Tests (shipped)

- `test_today_groups_by_horizon` — items sorted into correct sections
- `test_today_caps_at_5` — query returns all, display caps at 5
- `test_priority_sorting` — must > should > could
- `test_list_active_captures_excludes_done` — done items filtered
- `test_list_active_captures_excludes_snoozed_future` — future snooze hidden
- `test_list_active_captures_includes_expired_snooze` — expired snooze resurfaces
- `test_update_capture_status_to_done` / `_to_snoozed` — json_set works
- `test_resolve_capture_unique_prefix` / `_ambiguous` / `_nonexistent`
- `test_truncate_chars_unicode_safe` — emoji/curly quotes don't panic
- `test_parse_snooze_until_accepts_date` / `_rfc3339` / `_rejects_garbage`

### Done When

- [x] `arkai today` shows a useful summary — SHIPPED
- [x] Snooze/done commands work with prefix-ID resolution — SHIPPED
- [x] Snoozed items reappear when snooze expires — SHIPPED
- [ ] `arkai triage` interactive classification — DEFERRED

---

## 5. Module D: Retrieval Eval Framework

### Purpose
Measure search quality before and after improvements. Codex requested this
and it's the right call.

### Eval Set (15-30 queries)

```yaml
# eval/queries.yaml
queries:
  # Title-level (should work with current item embeddings)
  - query: "videos about AI replacing jobs"
    expected_top3: ["410f70ce3e7bfe39"]  # 85% Unemployable
    category: title_match

  - query: "how to build a personal AI assistant"
    expected_top3: ["1d27dd9de65bf69c"]  # Building Your Own Unified AI Assistant
    category: title_match

  - query: "prompting techniques and skills"
    expected_top3: ["BpibZSMGtdY"]  # Prompting Just Split Into 4 Skills
    category: title_match

  # Content-level (REQUIRES transcript chunking to work)
  - query: "stripe's approach to AI agent testing"
    expected_top3: ["V5A1IU8VVp4"]  # I Studied Stripe's AI Agents
    category: content_match

  - query: "Eric Weinstein wormhole physics"
    expected_top3: ["xnxasfyHtfo"]  # Eric Weinstein
    category: content_match

  - query: "vibe coding is dead"
    expected_top3: ["V5A1IU8VVp4"]  # Stripe's AI Agents
    category: content_match

  # Cross-content (tests entity/concept linking)
  - query: "what did different videos say about automation"
    expected_items_contain: ["410f70ce3e7bfe39", "hnwM01CpzmA"]
    category: cross_content

  # Capture retrieval (tests capture items)
  - query: "dentist appointment"
    expected_type: capture
    category: capture_match
```

### Eval Runner

```bash
arkai eval run                    # run all queries, report hit@3 and hit@5
arkai eval compare before after   # compare two eval runs
```

### Metrics

- **Hit@3**: Is the expected item in the top 3 results?
- **Hit@5**: Is the expected item in the top 5 results?
- **Category breakdown**: title_match, content_match, cross_content, capture_match

### Files to Create

| File | Action | Description |
|------|--------|-------------|
| `src/cli/eval.rs` | CREATE | Eval runner CLI |
| `eval/queries.yaml` | CREATE | Eval query set |
| `eval/baselines/` | CREATE | Saved eval results for comparison |

### Done When

- [ ] `arkai eval run` executes 15+ queries and reports hit@3 and hit@5
- [ ] Baseline captured BEFORE chunking
- [ ] Post-chunking eval shows improvement on content_match queries

---

## 6. Module E: Proactive Connections

### Purpose
When capturing or searching, automatically surface related items.

### How It Works

```bash
# When you capture something:
arkai capture "cold outreach pricing strategies"
# Output:
#   Captured as: todo (horizon: WEEK)
#   Related items:
#     [0.72] "45 People, $200M Revenue..." (video)
#     [0.68] "Stop Competing With 400 Applicants..." (video)

# When you search:
arkai search --semantic "pricing" --related
# Shows: search results + "You might also want:" with items
# connected via entities or high cosine similarity
```

### Implementation

On capture:
1. Embed the captured text
2. Run vector search against existing embeddings
3. If any result has cosine > 0.65, print "Related: {title} ({score})"
4. If a matching entity exists, print "Mentions: {entity_name} (also in: {other_items})"

This is ~50 lines of code on top of existing search infrastructure.

### Done When

- [ ] Capturing shows related items when they exist
- [ ] Related items are genuinely relevant (manual spot-check)

---

## 7. Integration Map

```
┌─────────────────────────────────────────────────────────────────┐
│  ARKAI (Rust CLI, ~/.cargo/bin/arkai)                            │
│                                                                   │
│  Commands:                                                        │
│    search --semantic "query"      ← hybrid FTS5 + vector          │
│    capture "text" [--kind] [--tag]← quick capture + embed         │
│    today                          ← what needs attention          │
│    triage                         ← classify inbox items          │
│    snooze <id> --until "..."      ← defer item                    │
│    done <id>                      ← mark complete                 │
│    store import --library --embed ← import + chunk + embed        │
│    store status                   ← item/embedding counts         │
│    eval run                       ← retrieval quality eval        │
│    ingest <url> [--tags]          ← content pipeline              │
└────────┬───────────────────┬────────────────────────────────────┘
         │                   │
         ▼                   ▼
┌────────────────┐  ┌──────────────────────────────────────────┐
│  OPENCLAW      │  │  DIRECT APPLE CLIs                        │
│  (port 18789)  │  │  (fallback if OpenClaw is down)           │
│                │  │                                           │
│  arkai bridge  │  │  notes list / notes add                   │
│  (port 19889)  │  │  remindctl list / remindctl add           │
│                │  │  (installed at ~/.local/bin/)              │
│  Tools:        │  └──────────────────────────────────────────┘
│  arkai_search  │
│  arkai_ingest  │  ┌──────────────────────────────────────────┐
│  arkai_store_* │  │  TELEGRAM (Claudia on VPS)                │
│                │  │  User sends message → Claudia → arkai     │
│  apple bridge  │  │  (Not yet wired for search/capture)       │
│  (port 19789)  │  └──────────────────────────────────────────┘
│                │
│  capture inbox │  ┌──────────────────────────────────────────┐
│  (JSONL)       │  │  FABRIC (Go binary)                       │
│                │  │  AI pattern execution (240+ patterns)      │
│  heartbeat     │  │  video_to_wisdom, extract_claims, etc.    │
│  (4h cycle)    │  └──────────────────────────────────────────┘
│                │
│  daily brief   │
│  (9:05 AM)     │
└────────────────┘
```

### Contract: arkai ↔ OpenClaw

arkai CLI is the ONLY interface. OpenClaw calls it via the bridge (port 19889).
No direct SQLite access from OpenClaw.

```
OpenClaw agent → arkai_search(query="pricing", mode="semantic")
  → bridge HTTP → arkai search --semantic "pricing"
  → parsed results returned to agent

OpenClaw agent → arkai_ingest_url(url="youtube.com/...", embed=true)
  → bridge HTTP → arkai ingest + store import
  → receipt returned to agent
```

### Contract: arkai ↔ capture_inbox (OpenClaw side)

When arkai captures with `--sync-openclaw`:
```json
// Appended to ~/AI/openclaw-local/workspace/output/memory/capture-inbox.jsonl
{
    "schema_version": "1.0.0",
    "id": "a1b2c3d4e5f6",
    "captured_at": "2026-03-22T05:30:00Z",
    "kind": "reminder",
    "title": "call dentist about insurance",
    "text": "call dentist about insurance",
    "source": "arkai-cli",
    "tags": [],
    "fanout": {
        "appleNotes": "skipped",
        "appleReminders": "requested",
        "arkaiIngest": "skipped"
    },
    "metadata": {"arkai_item_id": "a1b2c3d4e5f6"}
}
```

---

## 8. Build Order & Dependencies

```
Phase 1 (search quality — INDEPENDENT, can parallelize)
├── A1: Transcript chunker (store/chunking.rs)           SHIPPED 2026-03-22
├── A2: Migration 003 (chunks + chunk_embeddings)         SHIPPED 2026-03-22
├── A3: Chunk queries (store/queries.rs additions)        SHIPPED 2026-03-22
├── A4: Chunk-aware search (store/search.rs update)       SHIPPED 2026-03-22
│       multi_level_search: item+chunk RRF with CHUNK_WEIGHT=0.5
│       FTS5 sanitize_fts_query: phrase-quoting for safety (see note below)
├── A5: CLI: store import chunks + embeds transcripts     SHIPPED 2026-03-22
└── D1: Eval framework + query set                        NOT STARTED

Phase 2 (capture — INDEPENDENT of Phase 1)
├── B1: Capture CLI (cli/capture.rs)                      SHIPPED 2026-03-22
├── B2: Auto-classification (store/capture.rs)            SHIPPED 2026-03-22
├── B3: Capture store integration + embedding             SHIPPED 2026-03-22
└── B4: OpenClaw capture_inbox sync                       NOT STARTED
        No --sync-openclaw flag. No arkai_capture bridge tool.

Phase 3 (triage/surfacing — DEPENDS on Phase 2)
├── C1: Today command (cli/triage.rs)                     SHIPPED 2026-03-23
│       arkai today, arkai today --json
├── C2: Triage interactive (cli/triage.rs)                DEFERRED
│       No interactive arkai triage command yet.
├── C3: Snooze/Done commands                              SHIPPED 2026-03-23
│       arkai done <id-or-prefix>, arkai snooze <id-or-prefix> --until <date>
│       Prefix-ID resolution: unique prefix works, ambiguous errors clearly.
│       Snooze accepts YYYY-MM-DD (local timezone) or RFC3339.
│       Natural-language input ("tomorrow 9am") is NOT supported yet.
└── C4: Capture queries (by status, horizon, priority)    SHIPPED 2026-03-23
        list_active_captures, update_capture_status, resolve_capture_by_prefix

Phase 4 (proactive — DEPENDS on Phase 1 + 2)
├── E1: Related items on capture                          NOT STARTED
└── E2: Related items on search (--related flag)          NOT STARTED
```

**Note on FTS5 sanitization (A4):** All user queries are phrase-quoted before
passing to FTS5 MATCH, which prevents crashes on date-like text, hyphens, and
FTS5 operator characters. This is a deliberate safety-over-recall tradeoff:
FTS5 now acts as a precision boost (exact phrase match) while semantic/vector
search carries recall. Multi-word non-contiguous keyword queries will not match
via FTS5 — only via the vector path. Acceptable for this pass.

**Parallelization**: Phase 1 and Phase 2 can be built by separate agents simultaneously.
Phase 3 depends on Phase 2 (needs captures to triage). Phase 4 depends on both.

**Estimated total**: ~1,420 lines of new Rust code + ~200 lines of YAML/fixtures.

---

## 9. Acceptance Criteria

### Must Have (ship-blocking)

- [x] `arkai search --semantic "story about..."` returns transcript chunks, not just titles
- [x] `arkai capture "text"` works in <500ms with auto-classification
- [x] `arkai today` shows a useful 5-item summary
- [ ] Eval framework reports hit@3 on 15+ queries — NOT STARTED (D1)
- [x] All existing tests still pass — 238 tests, 0 failures (as of 2026-03-23)
- [x] New modules have >80% test coverage

### Should Have (v1 polish)

- [ ] `arkai triage` interactive flow works — DEFERRED (C2)
- [x] Snooze/done commands work — SHIPPED with prefix-ID resolution
- [ ] Capture syncs to OpenClaw capture_inbox.jsonl — NOT STARTED (B4)
- [ ] Related items shown on capture — NOT STARTED (E1)

### Won't Have (v2 / later)

- [ ] Reminder scheduling daemon (use launchd/heartbeat for now)
- [ ] Notion integration
- [ ] Voice memo real-time capture (use existing voice pipeline)
- [ ] Dark Dev Factory self-testing loop
- [ ] Multi-user support
- [ ] Entity resolution (merging)
- [ ] Graph queries

---

## 10. Patterns Borrowed from iron-ledger-atc

These patterns from `~/AI/iron-ledger-atc/` inform our design but are NOT
directly imported (iron-ledger is Python, arkai is Rust):

| Pattern | Source | How We Use It |
|---|---|---|
| **Horizon (NOW/WEEK/LATER)** | `src/core/enums.py` | Capture auto-classification + triage |
| **MoSCoW (MUST/SHOULD/COULD)** | `src/core/enums.py` | Priority in `arkai today` sorting |
| **Snooze as expiry** | `src/services/triage_svc.py` | `snoozed_until` field in metadata |
| **Shame-free framing** | `README.md` philosophy | No guilt language in triage/today output |
| **Deterministic surfacing** | `src/services/scheduler_svc.py` | Same captures → same `arkai today` output |
| **Field precedence** | `docs/FIELD_PRECEDENCE.md` | Multi-source capture reconciliation (later) |
| **3-tier identity** | `src/services/project_svc.py` | Content dedup (SHA256 → anchor → hash) |
| **Event-sourced audit** | `src/services/ledger_svc.py` | JSONL event log (already in arkai) |

### What We DON'T Borrow

- Full DailyPlan/PlanItem/WorkLog models — too complex for v1
- Todoist connector — not needed (Apple Reminders via bridge)
- Obsidian connector — not needed for v1
- CLI triage keybindings (n/w/l/z/d/s/q) — implement simpler version first
- Optimistic locking / checkpoints — overkill for single-user SQLite

---

## Appendix: Key File Reference (Existing)

| File | Purpose |
|------|---------|
| `src/store/db.rs` | Store connection, config, open/close |
| `src/store/migrations.rs` | Schema migrations (v1: tables, v2: mxbai-embed-large) |
| `src/store/queries.rs` | Item/entity/evidence/embedding CRUD |
| `src/store/search.rs` | Hybrid FTS5+vector search with RRF |
| `src/store/embedding.rs` | Ollama provider, cosine similarity |
| `src/store/import.rs` | Catalog + library import |
| `src/cli/mod.rs` | CLI commands (search, store, ingest, etc.) |
| `~/.arkai/store.db` | Live database (14 items, 14 embeddings, schema v2) |
| `~/.cargo/bin/arkai` | Installed binary (working from any cwd) |

---

*This spec is designed to be executed by Claude Code sessions without additional
prompting. Each module has: purpose, schema, files to create/modify, tests with
names, and "done when" criteria. Build from Phase 1 forward.*

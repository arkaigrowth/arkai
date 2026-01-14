# AI OS Architecture Brief

> **Purpose**: Canonical reference for any AI working on components of this system. Paste this into new conversations for instant context.

---

## System Overview

**AI OS** = A multi-component system for building persistent, resumable, auditable AI workflows.

```
┌─────────────────────────────────────────────────────────────────┐
│                    NATURAL LANGUAGE LAYER                        │
│              (Claude Code / LLM interface)                       │
│              Translates intent → CLI commands                    │
└───────────────────────────────┬─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│                    arkai (Rust Spine)                            │
│              Orchestration • State • Reliability                 │
│                                                                  │
│   Event Store ──── Catalog ──── Pipelines ──── Library          │
└───────────────────────────────┬─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│                    fabric (Go Patterns)                          │
│              AI Transformation • 240+ Prompts                    │
│              Stateless • One pattern = one transformation        │
└───────────────────────────────┬─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│                    LLM Provider                                  │
│              Claude / GPT / Ollama / Local                       │
└─────────────────────────────────────────────────────────────────┘
```

---

## Component Boundaries

| Component | Language | Responsibility | Does NOT Do |
|-----------|----------|----------------|-------------|
| **arkai** | Rust | Orchestration, state, storage, pipelines, event sourcing, library management | AI transformations, LLM calls |
| **fabric** | Go | Individual AI transformations, 240+ patterns, YouTube transcript fetching | Chaining, state, storage, pipelines |
| **Graph DB** (future) | TBD | Relationship queries, cross-content connections, entity linking | Content storage, AI transformations |
| **Vector DB** (future) | TBD | Semantic search, embeddings | Primary storage (indexes only) |

### Decision Rule
- **Workflow/chaining logic** → arkai (YAML pipelines)
- **AI prompt execution** → fabric (patterns)
- **Relationship queries** → Graph DB
- **Semantic search** → Vector DB
- **Primary content storage** → Files (library/)

### Intent Routing: 3-Layer Brain

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 1: REFLEXES (Instant, Deterministic)                       │
│ • URL regex matching → route to correct pipeline                 │
│ • youtube.com/* → youtube-wisdom pipeline                        │
│ • podcasts.apple.com/* → podcast pipeline (future)               │
│ • Cost: 0 tokens, <1ms                                           │
└─────────────────────────────────────────────────────────────────┘
                              ↓ (if no match)
┌─────────────────────────────────────────────────────────────────┐
│ Layer 2: LEARNED PATTERNS (Fast, Low-cost)                       │
│ • Keyword matching from past successful routes                   │
│ • "summarize" → summarize pattern                                │
│ • "wisdom" → extract_wisdom pattern                              │
│ • Cost: 0 tokens, <10ms                                          │
└─────────────────────────────────────────────────────────────────┘
                              ↓ (if ambiguous)
┌─────────────────────────────────────────────────────────────────┐
│ Layer 3: LLM FALLBACK (Accurate, Higher-cost)                    │
│ • Claude Code translates natural language → CLI                  │
│ • "learn from this podcast" → arkai ingest <url> --tags ...      │
│ • Used for edge cases and discovery                              │
│ • Cost: ~100-500 tokens                                          │
└─────────────────────────────────────────────────────────────────┘
```

**Design Bias**: Prefer Layer 1 > Layer 2 > Layer 3. Deterministic beats probabilistic.

---

## Storage Architecture

### Core Principle: Files as Source of Truth

```
Files (library/)         →  Human-readable, git-trackable, portable
Indexes (.arkai/)        →  Derived, regenerable, optional
```

### Canonical Library Location

```
~/AI/fabric-arkai/library/      # PRIMARY content storage (visible, git-trackable)
~/.arkai/                       # DERIVED data (catalog, runs, indexes)
```

### Directory Structure

```
~/AI/fabric-arkai/
├── library/                    # SOURCE OF TRUTH (git-track)
│   ├── youtube/
│   │   └── Video Title (XvGeXQ7js_o)/   # Human-readable + source ID
│   │       ├── metadata.json   # URL, title, tags, timestamps
│   │       ├── fetch.md        # Raw transcript
│   │       └── wisdom.md       # AI-extracted insights
│   ├── articles/
│   └── podcasts/
│
├── custom-patterns/            # Your fabric patterns
│   └── my_pattern/system.md
│
└── scripts/                    # Automation tools

~/.arkai/                       # DERIVED DATA (can regenerate)
├── config.yaml                 # Global config
├── catalog.json                # Quick lookup index
├── vectors.lance               # Future: semantic search
└── runs/                       # Event logs
    └── <run-id>/events.jsonl
```

### Content Addressing

- **Content ID** = `SHA256(canonical_url)[0:16]` (for catalog lookups)
- **Folder Name** = `"Title (source_id)"` (human-readable)
  - YouTube: `source_id` = video ID (e.g., `XvGeXQ7js_o`)
  - Web: `source_id` = first 8 chars of content hash
- Same URL always produces same ID (deduplication)
- Deterministic, collision-resistant

---

## Data Formats

### Event Log (events.jsonl)

```jsonl
{"type":"RunStarted","timestamp":"2024-01-02T10:00:00Z","run_id":"abc123","pipeline":"youtube-wisdom"}
{"type":"StepStarted","step":"fetch","timestamp":"2024-01-02T10:00:01Z"}
{"type":"StepCompleted","step":"fetch","duration_ms":1234,"output_path":"library/youtube/abc123/transcript.md"}
{"type":"StepStarted","step":"extract_wisdom","timestamp":"2024-01-02T10:00:02Z"}
{"type":"StepCompleted","step":"extract_wisdom","duration_ms":5678}
{"type":"RunCompleted","timestamp":"2024-01-02T10:00:08Z","status":"success"}
```

### Catalog Entry (catalog.json)

```json
{
  "version": 1,
  "items": [
    {
      "id": "9cd097ea928aa2dc",
      "content_type": "youtube",
      "url": "https://youtube.com/watch?v=XvGeXQ7js_o",
      "title": "Run YOUR own UNCENSORED AI",
      "tags": ["ai", "learning"],
      "created_at": "2024-01-02T10:00:00Z",
      "path": "library/youtube/Run YOUR own UNCENSORED AI (XvGeXQ7js_o)"
    }
  ]
}
```

### Metadata (metadata.json)

```json
{
  "id": "9cd097ea928aa2dc",
  "url": "https://youtube.com/watch?v=...",
  "title": "Video Title",
  "content_type": "youtube",
  "tags": ["ai", "learning"],
  "created_at": "2024-01-02T10:00:00Z",
  "pipeline": "youtube-wisdom",
  "run_id": "abc123"
}
```

### Pipeline Definition (YAML)

```yaml
name: youtube-wisdom
description: Extract wisdom from YouTube videos

safety_limits:
  max_steps: 10
  step_timeout_seconds: 300

steps:
  - name: fetch
    action: __youtube__           # arkai built-in: calls fabric -y <url>
    input_from: pipeline_input

  - name: wisdom
    action: extract_wisdom        # fabric pattern name
    input_from: fetch

  - name: summary
    action: summarize             # fabric pattern name
    input_from: wisdom
```

---

## Integration Contracts

### For New Components to be Compatible

| Requirement | Details |
|-------------|---------|
| **File-based I/O** | Read/write Markdown files in library/ |
| **Content addressing** | Use SHA256(url)[0:16] for content IDs |
| **Metadata format** | Follow metadata.json schema above |
| **Event format** | Emit JSONL events with type, timestamp |
| **Index derivation** | Indexes must be rebuildable from files |
| **No storage duplication** | Files are source of truth; DBs are indexes |

### Graph DB Requirements (Future)

Must support:
- **Node types**: Content, Entity, Tag, Topic
- **Edge types**: MENTIONS, CITES, RELATED_TO, TAGGED_WITH
- **Index from files**: Scan library/ to build graph
- **Query interface**: Return content IDs, not raw content

### Vector DB Requirements (Future)

Must support:
- **Embed from files**: Generate embeddings from library/ content
- **Return content IDs**: Not raw content (content lives in files)
- **Incremental updates**: New files trigger embedding generation
- **Rebuild capability**: Full index rebuildable from files

---

## Provenance, Evidence, and Grounding (V1)

> Purpose: Prevent "AI said so" by attaching **receipts** to extracted entities/claims across the entire library (YouTube, podcasts, articles).
> Principle: **Files are source of truth** … DBs are derived indexes. Evidence is stored as append-only files, validated by hashes.

### Why this exists
LLMs can still hallucinate, but arkai makes every extracted assertion:
- **Auditable** … jump to the exact source span in `transcript.md`
- **Drift-detectable** … hashes break loudly if files change
- **Repairable later** … schema supports reanchoring, but V1 stays deterministic and honest

---

### Canonical Span Strategy

#### Grounding source
All provenance spans ground to artifact files under a content item, most commonly:
- `transcript.md` (canonical grounding target for text sources)

#### Span unit (canonical)
Offsets are **UTF-8 byte offsets into raw file bytes**:
- `utf8_byte_offset: [start, end]` where `start/end` are byte indices into `transcript.md` bytes.

#### Flexible mode (edits allowed, correctness protected)
Arkai allows transcript edits, but correctness is protected by:
- `slice_sha256` (per span, fine-grain validation)
- `artifact_digests` (per artifact, coarse drift detection)
- `status` + structured resolution info (never fail hard … record what happened)

---

### Matching Policy (V1)

#### V1 rule: do not generate spans unless exact match exists
Algorithm for quote → span resolution:
1. **Exact raw byte search** for the quote inside the artifact bytes
2. If 1 match … `status=resolved` and compute span + `slice_sha256`
3. If N>1 matches … `status=ambiguous`, choose deterministically (first match), record match_count/rank
4. If 0 matches … `status=unresolved`
   - Optional hint only: whitespace/NFC normalized check to tag `reason=normalized_match_only`
   - Do **not** attempt normalization offset mapping in V1

#### Normalization (V1)
- No lowercasing in V1 matching
- Normalized matching (whitespace collapse + Unicode NFC) is hint-only for `unresolved_reason`
- Fuzzy/case-insensitive matching is deferred behind flags (see "Deferred")

---

### Artifact Digests (metadata.json)

Add `artifact_digests` to `metadata.json` for drift detection.

```json
{
  "schema_version": 2,
  "id": "9cd097ea928aa2dc",
  "canonical_ref": "url:https://youtube.com/watch?v=...",

  "artifacts": {
    "transcript": "./transcript.md",
    "wisdom": "./wisdom.md",
    "entities": "./entities.json",
    "evidence": "./evidence.jsonl"
  },

  "artifact_digests": {
    "transcript.md": "sha256:abc123...",
    "wisdom.md": "sha256:def456..."
  }
}
```

Notes:
- If `artifact_digests["transcript.md"]` mismatches current file … arkai can quickly flag drift.
- Individual spans are still validated using `slice_sha256`.

---

### Shared Span Object (V1)

Used by evidence lines and entity mentions.

```json
{
  "artifact": "transcript.md",
  "utf8_byte_offset": [1234, 1456],
  "slice_sha256": "sha256:...",
  "anchor_text": "...~80 chars around span...",
  "video_timestamp": "00:12:34"
}
```

Rules:
- `slice_sha256` is REQUIRED when status in {resolved, ambiguous}.
- `anchor_text` is OPTIONAL. Default extraction window: 80 chars.
- `video_timestamp` is OPTIONAL. Only set when transcripts contain timestamps.

---

### Evidence Storage (evidence.jsonl)

#### File format
`evidence.jsonl` is append-only (one JSON object per line). It is canonical truth for extracted claims.

#### Evidence line schema (V1)

```json
{
  "id": "a1b2c3d4e5f6g7h8",
  "content_id": "9cd097ea928aa2dc",

  "claim": "Revenue grew 40% year over year",
  "quote": "our revenue grew by forty percent year over year",
  "quote_sha256": "sha256:...",

  "status": "resolved",
  "resolution": {
    "method": "exact",
    "match_count": 1,
    "match_rank": 1,
    "reason": null
  },

  "span": {
    "artifact": "transcript.md",
    "utf8_byte_offset": [1234, 1290],
    "slice_sha256": "sha256:...",
    "anchor_text": "...~80 chars...",
    "video_timestamp": "00:12:34"
  },

  "confidence": 0.92,
  "extractor": "extract_claims",
  "ts": "2026-01-12T10:00:00Z"
}
```

#### Status enum (V1)
- `resolved` … exact match found, span computed
- `ambiguous` … multiple exact matches, deterministic selection made
- `unresolved` … no exact match, no span

#### resolution.method enum (V1)
- `exact` (span computed)
- `none` (unresolved)
- `normalized_hint` (unresolved … hint found but no span)

#### resolution.reason enum (V1)
- `no_match`
- `multiple_matches`
- `normalized_match_only`

#### match_rank indexing
- `match_rank` is 1-indexed ("1st match of N")

#### Evidence IDs (deterministic, collision-safe)
Two-tier ID strategy:
- **Unresolved ID** (no span): `id = sha256(content_id + extractor + quote_sha256)[0:16]`
- **Resolved/Ambiguous ID** (span exists): `id = sha256(content_id + extractor + quote_sha256 + start + end)[0:16]`

---

### Entities Storage (entities.json)

Entities are canonical file artifacts. Graph DBs only index them.

```json
{
  "schema_version": 1,
  "extracted_by": "extract_entities",
  "extracted_at": "2026-01-12T10:00:00Z",
  "entities": [
    {
      "name": "Naval Ravikant",
      "type": "person",
      "confidence": 0.95,
      "mentions": [
        {
          "quote": "Naval said that",
          "quote_sha256": "sha256:...",
          "status": "resolved",
          "resolution": { "method": "exact", "match_count": 1, "match_rank": 1, "reason": null },
          "span": {
            "artifact": "transcript.md",
            "utf8_byte_offset": [1234, 1250],
            "slice_sha256": "sha256:...",
            "video_timestamp": "00:05:12"
          }
        }
      ]
    }
  ]
}
```

Entity type enum (extensible):
- `person`, `org`, `concept`, `product`, `location`, `event`

---

### Fabric Pattern Contract (Quote-based)

V1 extraction patterns output quotes … arkai resolves spans deterministically.

#### extract_claims (fabric)
Output JSON array:
```json
[
  { "claim": "...", "quote": "VERBATIM SUBSTRING FROM TRANSCRIPT", "confidence": 0.9 }
]
```

#### extract_entities (fabric)
Output JSON object (entities + mentions):
```json
{
  "entities": [
    { "name": "...", "type": "...", "confidence": 0.9, "mentions": [ { "quote": "VERBATIM..." } ] }
  ]
}
```

**Hard rule for prompts:**
- Quote must be verbatim substring of the transcript … no paraphrase.

---

### Evidence CLI Commands (V1)

#### `arkai evidence show <evidence_id>`
- Locate evidence line in `evidence.jsonl`
- Load artifact file (usually `transcript.md`)
- Slice bytes from `utf8_byte_offset`
- Display: claim, status, resolution, file path, computed line/col, snippet (the slice), timestamp if present

#### `arkai evidence open <evidence_id>`
- Same as show, then open editor at location: `code -g path/to/transcript.md:<line>:<col>`

#### `arkai evidence validate <content_id>`
- Validate `artifact_digests["transcript.md"]` vs current transcript hash
- For each evidence line with a span: recompute `slice_sha256` at stored offsets, report counts: valid, stale, unresolved
- Emit validation event

---

### Evidence Events (events.jsonl)

Emit lightweight audit events:
- `EvidenceAppended { content_id, evidence_id, status, extractor }`
- `EvidenceValidated { content_id, artifact, digest_ok, valid_count, stale_count, unresolved_count }`

Deferred:
- `EvidenceResolved { evidence_id, old_status, new_status, method }`

---

### Graph DB Indexing Guidance (Derived Data)

Graph DB is a derived index of file artifacts.

Recommended V1 throttle:
- Index evidence only when: `status == resolved`, `confidence >= 0.8`, cap top 50 evidence lines per content (by confidence)
- Always preserve full `evidence.jsonl` on disk as canonical.

---

### Deferred (V1.1+)

- `arkai evidence reanchor` (use anchor_text to repair offsets)
- Case-insensitive match (`--match-mode insensitive`)
- Fuzzy fallback (`--match-mode fuzzy`, threshold behind flag)
- PDF/Zotero ingestion and provenance over PDFs

---

## CLI Interface

### Core Commands

```bash
# Content ingestion
arkai ingest <url> [--tags "a,b"] [--pipeline name]

# Library operations
arkai library [--content-type youtube]
arkai search <query>
arkai show <id> [--full]
arkai reprocess <id>

# Pipeline operations
arkai run <pipeline> [--input <data>]
arkai status <run_id>
arkai resume <run_id>
arkai runs

# System
arkai config
arkai reindex
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Pipeline failure (resumable) |
| 4 | Resource limit exceeded |

---

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Rust for arkai** | Single binary, no runtime deps, memory safety, fast startup |
| **Go for fabric** | Community adoption, 240+ patterns, YouTube integration |
| **Files as source of truth** | Human-readable, git-trackable, portable, grep-able |
| **Event sourcing** | Resume from failure, full audit trail, replay capability |
| **Content-addressable storage** | Deduplication, stable references, deterministic IDs |
| **YAML pipelines** | Declarative, versionable, composable workflows |
| **Indexes are derived** | Can always rebuild from files; no data loss |

---

## Anti-Patterns to Avoid

| Avoid | Instead |
|-------|---------|
| Storing content in database only | Files are source of truth; DB is index |
| Fabric pipelines | arkai owns pipelines; fabric does single transformations |
| Multiple content locations | Single library/ directory |
| Hidden state | Event logs capture all state transitions |
| Tight coupling | Components communicate via files and CLI |

---

## Quick Reference

**arkai**: Rust spine, orchestration, state, storage
**fabric**: Go patterns, AI transformations, stateless
**library/**: Source of truth, git-trackable content
**.arkai/**: Derived data, indexes, event logs
**Content ID**: SHA256(url)[0:16]
**Events**: Append-only JSONL logs
**Pipelines**: YAML workflow definitions

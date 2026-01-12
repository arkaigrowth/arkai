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

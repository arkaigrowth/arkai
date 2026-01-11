# Architecture Clarity: arkai + fabric + Claude Code

> This document answers the key questions about how the system fits together.

---

## 1. UPSTREAM vs FORK — What Does It Mean?

**Terms:**
- **Upstream** = The original `danielmiessler/fabric` repo
- **Fork** = Your personal copy of fabric that you control

**Can you fork and add your own stuff?**

YES, absolutely. But here's why you probably don't need to:

### Option A: Fork Fabric
```
Pros:
- Full control over everything
- Can modify any pattern

Cons:
- You now maintain 240+ patterns
- When fabric updates (often), you must merge
- Your patterns are mixed with community ones
- More repos to manage
```

### Option B: Custom Patterns Directory (RECOMMENDED)
```
Fabric automatically looks for patterns in:
~/.config/fabric/patterns/

You can add YOUR patterns there without touching fabric at all!

Example:
~/.config/fabric/patterns/
├── my_podcast_analyzer/
│   └── system.md
├── my_custom_summarizer/
│   └── system.md
└── ... (your patterns)

Then run: fabric -p my_podcast_analyzer
```

### Option C: arkai Pipelines (WHAT WE'RE DOING)
```
Most "custom stuff" isn't a new PATTERN.
It's a new WORKFLOW (sequence of patterns).

Podcasts example:
- No new pattern needed
- New WORKFLOW needed: download → transcribe → extract → store
- That workflow lives in arkai pipelines, not fabric
```

**BOTTOM LINE:** Don't fork unless you need to modify fabric's CORE CODE. For custom patterns, use the patterns directory. For custom workflows, use arkai pipelines.

---

## 2. WHERE DOES LOGIC LIVE?

### The Clear Boundary

| Component | Does What | Example |
|-----------|-----------|---------|
| **fabric** | Individual AI transformations | `fabric -p extract_wisdom` takes text → outputs wisdom |
| **arkai** | Chains patterns + stores results | Runs fetch → extract → summarize → save to library |
| **Claude Code** | Natural language understanding | "learn from this" → `arkai ingest <url>` |

### Concrete Example: YouTube Video Ingestion

```
User: arkai ingest "https://youtube.com/watch?v=abc"

WHAT HAPPENS:

1. arkai detects: This is a YouTube URL
2. arkai loads pipeline: youtube-wisdom.yaml
3. arkai runs Step 1: fabric -y <url> --transcript-with-timestamps
   └── fabric fetches transcript (via yt-dlp internally)
4. arkai runs Step 2: echo <transcript> | fabric -p extract_wisdom
   └── fabric calls LLM, returns wisdom
5. arkai runs Step 3: echo <wisdom> | fabric -p summarize
   └── fabric calls LLM, returns summary
6. arkai stores: library/youtube/<hash>/
   └── source.md, wisdom.md, summary.md
7. arkai logs: ~/.arkai/runs/<run_id>/events.jsonl
8. arkai updates: catalog.json
```

**fabric does:** Steps 3, 4, 5 (AI transformations)
**arkai does:** Steps 1, 2, 6, 7, 8 (orchestration, routing, storage)

---

## 3. WHAT ARE YAML PIPELINES?

**Key insight:** Fabric has NO pipeline concept. It runs ONE pattern at a time.

**arkai ADDED pipelines** to chain multiple fabric calls together.

### Example Pipeline: `pipelines/youtube-wisdom.yaml`

```yaml
name: youtube-wisdom
description: Extract wisdom from YouTube video transcripts

steps:
  - name: fetch
    adapter: fabric
    action: __youtube__           # Special arkai action → fabric -y <url>
    input_from: pipeline_input

  - name: wisdom
    adapter: fabric
    action: extract_wisdom        # Fabric pattern
    input_from:
      previous_step: fetch

  - name: summary
    adapter: fabric
    action: summarize             # Fabric pattern
    input_from:
      previous_step: wisdom
```

### What This Gets You

| Feature | Without Pipelines | With arkai Pipelines |
|---------|-------------------|----------------------|
| Chain patterns | Manual `|` piping | Declarative YAML |
| Resume on failure | Start over | `arkai resume <run_id>` |
| Audit trail | None | Full event log |
| Retry logic | Manual | Built-in |
| Storage | Manual | Automatic library |

---

## 4. INTENT ROUTING — How It Works

### Current State (No Intent Routing)

You must know exact commands:
```bash
# You must know this exact syntax
arkai ingest "https://youtube.com/..."

# Or the fabric pattern name
fabric -p extract_wisdom
```

### Proposed State: Two Scenarios

**SCENARIO A: Running in Claude Code**
```
You: "learn from this podcast about AI consulting"

Claude Code:
1. Understands your intent (natural language)
2. Knows arkai exists and its capabilities
3. Generates: arkai ingest "https://..." --tags "ai,consulting"
4. Runs it

No extra LLM call needed — Claude IS the intent layer.
This is the PRIMARY use case.
```

**SCENARIO B: Running in Plain Terminal**
```bash
# Option 1: Explicit command (no LLM, instant)
$ arkai ingest "https://podcasts.apple.com/..."

# Option 2: Ask for help (uses fabric's LLM)
$ arkai ask "learn from this podcast"
# → arkai internally calls: echo "learn from podcast" | fabric -p suggest_pattern
# → fabric's LLM suggests patterns
# → arkai translates to command and runs
```

### Where Is the LLM Called?

| Context | LLM Location | Cost |
|---------|--------------|------|
| Claude Code | Claude (already running) | Included in session |
| Terminal explicit | None | Free, instant |
| Terminal `arkai ask` | fabric's configured LLM | Per-call API cost |

---

## 5. CLI vs CLAUDE CODE — The Difference

### Running in Claude Code (Recommended)

```
┌─────────────────────────────────────────────────────────────────┐
│                     CLAUDE CODE SESSION                          │
│                                                                  │
│  You: "extract wisdom from this video and save to my library"   │
│                                                                  │
│  Claude: [understands intent]                                   │
│          [knows arkai commands]                                 │
│          [runs]: arkai ingest "url" --tags "..."                │
│          [shows you the result]                                 │
│                                                                  │
│  BENEFITS:                                                      │
│  • Natural language (no commands to memorize)                   │
│  • Full context (Claude knows your conversation)                │
│  • Error recovery (Claude can debug and retry)                  │
│  • Pattern discovery (Claude can search fabric patterns)        │
└─────────────────────────────────────────────────────────────────┘
```

### Running in Plain Terminal

```
┌─────────────────────────────────────────────────────────────────┐
│                     TERMINAL (fish/bash/zsh)                     │
│                                                                  │
│  $ arkai ingest "url"        # Must know command                │
│  $ arkai search "topic"      # Must know syntax                 │
│  $ arkai show <id>           # Must know options                │
│                                                                  │
│  OR with LLM help:                                              │
│  $ arkai ask "what patterns help with summarizing?"             │
│    → Calls fabric's LLM                                         │
│    → Returns suggestions                                        │
│                                                                  │
│  BENEFITS:                                                      │
│  • Scriptable (can use in shell scripts)                        │
│  • Faster startup (no Claude handshake)                         │
│  • Works offline (for explicit commands)                        │
└─────────────────────────────────────────────────────────────────┘
```

### Where Are Logs/Conversations?

| Log Type | Location | Purpose |
|----------|----------|---------|
| arkai events | `~/.arkai/runs/<run_id>/events.jsonl` | Audit trail, resume |
| Claude Code | Claude's memory (session) | Conversation context |
| `arkai ask` | Not logged currently | Future: could add conversation log |

---

## 6. VECTOR SEARCH — How It Would Work

### Tier 1: Keyword Search (Current)

```bash
$ arkai search "pricing"

# What happens:
# 1. Grep through library/ files
# 2. Match files containing "pricing"
# 3. Return list of matches

# Pros: Fast, no setup, no dependencies
# Cons: Exact match only, no semantic understanding
```

### Tier 2: Embedded Vectors (Proposed)

```bash
$ arkai index   # One-time: create embeddings for all content

$ arkai search "how to price AI consulting services"

# What happens:
# 1. Convert query to embedding vector
# 2. Find nearest neighbors in vector DB
# 3. Return semantically similar content

# Even finds content that says "determining fees for ML work"
# (semantically similar, different words)
```

**Implementation (SQLite-vec or LanceDB):**
```rust
// On ingest:
let embedding = generate_embedding(content);  // Via OpenAI API or local model
db.insert(content_id, embedding);

// On search:
let query_embedding = generate_embedding(query);
let results = db.nearest_neighbors(query_embedding, k=10);
```

**Why SQLite-vec/LanceDB over RAGFlow:**
- Single binary (no Docker)
- Works offline (with local embeddings)
- Fast startup
- Lower friction for users

### Tier 3: Graph Database (Optional, Future)

```bash
$ arkai graph "connections between AI videos"

# Neo4j enables:
# - "Show videos that reference each other"
# - "Find experts mentioned across multiple sources"
# - "What topics connect these two videos?"
```

**Tradeoff:** Requires Docker or cloud Neo4j. Higher friction, but more powerful for relationship queries.

---

## 7. DECISION: What Should We Build?

### Clear Separation

| arkai | fabric | Why |
|-------|--------|-----|
| URL detection & routing | N/A | arkai's job to detect content types |
| YAML pipelines | N/A | Workflows are arkai's domain |
| Event logging | N/A | State is arkai's domain |
| Library storage | N/A | Persistence is arkai's domain |
| N/A | AI patterns | Transformations are fabric's domain |
| N/A | LLM access | AI is fabric's domain |
| N/A | YouTube transcript | Built into fabric |

### What To Build Next

1. **`__podcast__` action in arkai** — orchestrates yt-dlp + fabric --transcribe
2. **Custom patterns in ~/.config/fabric/patterns/** — for any new AI transformations
3. **Pipeline YAML files in pipelines/** — for custom workflows
4. **`arkai ask` command** — for terminal NL support (uses fabric's LLM)
5. **Vector search** — start with SQLite-vec for semantic search

---

## Summary: The Mental Model

```
┌─────────────────────────────────────────────────────────────────┐
│                        YOU (Human)                               │
│                                                                  │
│        "I want to learn from this podcast"                      │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                     CLAUDE CODE (The Brain)                      │
│                                                                  │
│   Understands intent → generates arkai command                  │
│   OR you use terminal directly with explicit commands           │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                     arkai (The Spine)                            │
│                                                                  │
│   • Routes URL to correct workflow                              │
│   • Runs pipeline steps                                         │
│   • Stores results in library                                   │
│   • Logs everything for audit/resume                            │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                     fabric (The Hands)                           │
│                                                                  │
│   • Individual AI transformations                               │
│   • Accesses your LLM (OpenAI, Anthropic, Ollama)               │
│   • Stateless — does one thing, returns result                  │
└─────────────────────────────────────────────────────────────────┘
```

**The key insight:** Claude Code is the ideal interface. You don't need to memorize commands. You just describe what you want, and Claude + arkai + fabric make it happen.

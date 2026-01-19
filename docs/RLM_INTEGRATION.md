# RLM Integration Design Document

> **Status**: Phase 0 Complete (MCP wired) | **Last Updated**: 2026-01-18
> **Base**: [richardwhiteii/rlm](https://github.com/richardwhiteii/rlm) fork at `~/AI/rlm-fork/`

---

## Overview

RLM (Recursive Language Model) enables analysis of massive contexts (files, repos, logs) that exceed LLM context windows. Instead of stuffing everything into the prompt, RLM gives the LLM a **REPL** (Read-Eval-Print Loop) that can load, filter, and recursively query external state.

**Key Principle**: RLM is a **skill/sidecar**, NOT an embedded LLM layer. arkai does NOT make LLM calls directly. RLM outputs artifacts; arkai validates them.

---

## Architecture

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CLAUDE CODE (Natural Language)                       │
│                                                                              │
│  User: "Analyze this repo for security issues"                               │
│                                                                              │
│  Claude calls MCP tools:                                                     │
│  1. rlm_load_context(name="repo", content=<repo files>)                     │
│  2. rlm_filter_context(pattern="auth|password|token")                       │
│  3. rlm_sub_query(query="Find security issues", provider="openrouter")      │
│  4. rlm_exec(code="count matches by file")  # Requires HITL approval        │
│                                                                              │
│  Outputs: ~/.claude/rlm/sessions/<session>/                                 │
│           ├── findings.json                                                  │
│           └── evidence_candidates.jsonl                                      │
│                                                                              │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │ (publish to library)
┌──────────────────────────────────▼──────────────────────────────────────────┐
│                         ARKAI (Rust Spine)                                   │
│                                                                              │
│  Does NOT call LLMs. Only:                                                   │
│  • Invokes RLM as: arkai tool rlm --json-input '...'                        │
│  • Validates evidence_candidates.jsonl → evidence.jsonl                     │
│  • Places artifacts in ~/AI/library/<type>/<id>/rlm/                        │
│  • Emits events to ~/.arkai/runs/<run_id>/events.jsonl                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Tool Surface (12 MCP Tools)

| Tool | Purpose | Type | HITL |
|------|---------|------|------|
| `rlm_load_context` | Load content as external variable | Deterministic | No |
| `rlm_inspect_context` | View metadata without loading | Deterministic | No |
| `rlm_chunk_context` | Split by strategy (lines/chars/paragraphs) | Deterministic | No |
| `rlm_get_chunk` | Retrieve specific chunk by index | Deterministic | No |
| `rlm_filter_context` | Regex-based filtering | Deterministic | No |
| `rlm_list_contexts` | List all loaded contexts | Deterministic | No |
| `rlm_sub_query` | LLM call on chunk/context | Budgeted | Cost approval |
| `rlm_sub_query_batch` | Parallel LLM calls on chunks | Budgeted | Cost approval |
| `rlm_auto_analyze` | One-step intelligent analysis | Budgeted | Strategy + cost |
| `rlm_exec` | Sandboxed Python execution | Dangerous | Code approval |
| `rlm_store_result` | Persist results for aggregation | Internal | No |
| `rlm_get_results` | Retrieve aggregated results | Internal | No |

---

## Model Routing (Two-Lane Pattern)

RLM uses the same routing pattern as the steelman engine:

### Lane 1: Claude Max (Subscription)

- **Models**: `anthropic/claude-opus-4.5`, `anthropic/claude-sonnet-4`
- **Routing**: `model.startswith("anthropic/")` → Claude CLI subprocess
- **Cost**: Included in Max subscription
- **Tool calls**: NOT supported in this lane (yet)

### Lane 2: OpenRouter (Paid API)

- **Models**: `openai/gpt-5.2`, `openai/gpt-4o-mini`, `google/gemini-flash`, `z-ai/glm-4.7`
- **Routing**: All other models → OpenRouter API
- **Cost**: Pay per token
- **Tool calls**: SUPPORTED via OpenAI-compatible API

```python
# OpenRouter integration (same pattern as steelman)
import openai

client = openai.OpenAI(
    base_url="https://openrouter.ai/api/v1",
    api_key=os.environ.get("OPENROUTER_API_KEY"),
)

response = client.chat.completions.create(
    model="z-ai/glm-4.7",  # or "openai/gpt-4o-mini"
    messages=[...],
    tools=[...],           # Tool definitions
    tool_choice="auto",    # Let model decide
)
```

### Lane 3: Ollama (Local, Free)

- **Models**: `gemma3:12b`, `qwen2.5:14b`, etc.
- **Routing**: `provider="ollama"` explicit parameter
- **Cost**: $0 (local compute only)
- **Tool calls**: Supported via Ollama's tool API

---

## Configuration

### MCP Server Config (`~/.claude/settings.json`)

```json
{
  "mcpServers": {
    "rlm": {
      "command": "uv",
      "args": [
        "--directory",
        "/Users/alexkamysz/AI/rlm-fork",
        "run",
        "python",
        "-m",
        "src.rlm_mcp_server"
      ],
      "env": {
        "RLM_DATA_DIR": "/Users/alexkamysz/.rlm-data",
        "OLLAMA_URL": "http://localhost:11434",
        "OPENROUTER_API_KEY": "${OPENROUTER_API_KEY}"
      }
    }
  }
}
```

### Budget Configuration (MVP Defaults)

```yaml
rlm:
  recursion:
    max_depth: 1              # Hard stop - no recursive sub-queries

  budgets:
    max_subquery_calls: 50    # Per-run hard limit
    max_tokens: 100000        # Per-run token limit
    max_dollars: 1.00         # Per-run spend limit
    warning_threshold: 0.80   # 80% triggers HITL approval

  exec:
    enabled: false            # OFF by default
    timeout_seconds: 30
    memory_limit_mb: 256
    blocked_imports:
      - os
      - subprocess
      - socket
      - requests
      - urllib
```

---

## Sandbox Hardening (MVP)

The `rlm_exec` tool runs Python code in a sandboxed subprocess. Hardening requirements:

### Resource Limits (RLIMIT)

```python
import resource

# Memory limit (256MB)
resource.setrlimit(resource.RLIMIT_AS, (256 * 1024 * 1024, 256 * 1024 * 1024))

# CPU time limit (30s)
resource.setrlimit(resource.RLIMIT_CPU, (30, 30))

# Output file size limit (10MB)
resource.setrlimit(resource.RLIMIT_FSIZE, (10 * 1024 * 1024, 10 * 1024 * 1024))

# Open files limit (32)
resource.setrlimit(resource.RLIMIT_NOFILE, (32, 32))
```

### Environment Scrubbing

```python
# SCRUB these env vars (API keys, secrets)
SCRUB_VARS = [
    "OPENROUTER_API_KEY",
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "AWS_SECRET_ACCESS_KEY",
    "GITHUB_TOKEN",
]

safe_env = {
    "PATH": "/usr/bin:/bin",
    "HOME": "/tmp",
    "LANG": "en_US.UTF-8",
}
```

### Import Blocklist

```python
BLOCKED_IMPORTS = {
    "os",
    "subprocess",
    "socket",
    "requests",
    "urllib",
    "http",
    "ftplib",
    "smtplib",
    "multiprocessing",
    "threading",
}

ALLOWED_IMPORTS = {
    "re",
    "json",
    "collections",
    "math",
    "datetime",
    "itertools",
    "functools",
    "typing",
}
```

---

## Evidence Integration

### Output Format (`evidence_candidates.jsonl`)

```json
{
  "id": "rlm-abc123",
  "claim": "API endpoint lacks authentication",
  "quote": "router.get('/admin', handler)",

  "artifact_sha256": "sha256:...",
  "span_type": {
    "line_range": [42, 42],
    "byte_range": [1234, 1290]
  },

  "chunk_strategy": "lines:100@v1",
  "chunk_id": "a3f8b2c1d9e4f567",

  "confidence": 0.85,
  "verification_status": "unverified",
  "source_kind": "repo",

  "rlm_metadata": {
    "model_used": "openai/gpt-4o-mini",
    "provider": "openrouter",
    "tokens_used": 1500,
    "cost_usd": 0.003,
    "recursion_depth": 0
  }
}
```

### Chunk ID Formula

Deterministic, stable, strategy-versioned:

```python
import hashlib

def compute_chunk_id(artifact_sha256: str, chunk_strategy: str, start: int, end: int) -> str:
    """
    Compute deterministic chunk ID.

    If chunk strategy changes, chunk IDs change → no mixing apples and oranges.
    """
    data = f"{artifact_sha256}:{chunk_strategy}:{start}:{end}"
    return hashlib.sha256(data.encode()).hexdigest()[:16]

# Example:
# artifact_sha256 = "sha256:abc123..."
# chunk_strategy = "lines:100@v1"
# chunk_id = compute_chunk_id(artifact_sha256, "lines:100@v1", 0, 1000)
# → "a3f8b2c1d9e4f567"
```

---

## Storage Layout

### Scratch (Claude Code Sessions)

```text
~/.claude/rlm/sessions/<session_id>/
├── contexts/           # Loaded contexts
├── chunks/             # Cached chunks
├── results/            # Aggregated results
├── findings.json       # Structured analysis
└── evidence_candidates.jsonl
```

### Canonical (arkai Library)

```text
~/AI/library/<type>/<id>/
├── metadata.json
├── transcript.md (or source files)
├── rlm/
│   ├── analysis.md     # Human-readable summary
│   ├── findings.json   # Structured findings
│   └── chunks/         # Cached chunks (optional)
└── evidence.jsonl      # arkai-validated evidence
```

---

## Event Schema

```jsonl
{"type":"RLMAnalysisStarted","timestamp":"...","content_id":"...","path":"/path/to/repo","strategy":"lines","chunk_count":20}
{"type":"RLMCheckpointApproved","checkpoint":"strategy","approved_by":"human","timestamp":"..."}
{"type":"RLMCheckpointApproved","checkpoint":"batch_cost","approved_by":"human","estimated_cost":0.40,"timestamp":"..."}
{"type":"RLMChunkProcessed","chunk_index":3,"model":"gpt-4o-mini","duration_ms":1234,"findings_count":2}
{"type":"RLMAnalysisCompleted","timestamp":"...","total_chunks":20,"total_findings":15,"models_used":["claude-sonnet","gpt-4o-mini"]}
```

---

## Implementation Roadmap

### Phase 0: MCP Integration ✅ DONE

- [x] Fork richardwhiteii/rlm
- [x] Wire MCP in ~/.claude/settings.json
- [x] Configure data directory
- [ ] Smoke test

### Phase 1: Budget + Sandbox (MVP)

- [ ] CostTracker middleware
- [ ] BudgetManager
- [ ] Sandbox hardening (RLIMIT, env, imports)

### Phase 2: OpenRouter Integration

- [ ] Add OpenRouter provider
- [ ] Two-lane routing
- [ ] Tool call support

### Phase 3: Evidence Integration

- [ ] evidence_candidates.jsonl output
- [ ] Enhanced schema
- [ ] arkai evidence resolve

### Phase 4: HITL + Skill

- [ ] Strategy checkpoint
- [ ] Cost checkpoint
- [ ] Exec checkpoint
- [ ] /rlm command

### Phase 5: arkai Integration

- [ ] __skill__:rlm in pipelines
- [ ] Event logging
- [ ] Scratch → Publish

---

## References

- MIT RLM Paper: https://arxiv.org/html/2512.24601v1
- richardwhiteii/rlm: https://github.com/richardwhiteii/rlm
- steelman-engine router: `~/AI/steelman-engine/steelman/models/router.py`
- OpenRouter API: https://openrouter.ai/docs

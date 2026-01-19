# Session 6: RLM Integration

> **Date**: 2026-01-18
> **Status**: Phase 0 Complete, Smoke Tests Partial Pass
> **Next**: Fix sandbox security, test OpenRouter, then Phase 1

---

## Executive Summary

Integrated MIT's Recursive Language Model (RLM) paper concepts into arkai via richardwhiteii/rlm MCP server fork. Core functionality works. Security hardening needed before production.

---

## Key Decisions Made

| Decision | Rationale | Source |
|----------|-----------|--------|
| **RLM = skill/sidecar, NOT LLM layer** | arkai doesn't call LLMs directly; RLM outputs artifacts | Chad's steelman |
| **Base: richardwhiteii/rlm** | True REPL-over-context MCP, not map/reduce orchestration | Validated via code inspection |
| **NOT BowTiedSwan/rlm-skill** | That's chunking utility only, no exec/subquery | Code comparison |
| **Two-lane model routing** | Claude Max (subscription) + OpenRouter (paid API) | Steelman engine pattern |
| **Chunk IDs = sha256(artifact + strategy + offsets)** | Deterministic, stable, strategy-versioned | Chad's feedback |
| **OpenRouter via openai SDK** | Same pattern as steelman:consensus | Existing infrastructure |

---

## Files Created/Modified

### NEW FILES

| Path | Purpose |
|------|---------|
| `/Users/alexkamysz/AI/rlm-fork/` | Fork of richardwhiteii/rlm (cloned) |
| `/Users/alexkamysz/AI/rlm-fork/src/__init__.py` | Missing init file (created) |
| `/Users/alexkamysz/AI/rlm-fork/run_rlm.sh` | Bash wrapper for MCP server |
| `/Users/alexkamysz/AI/arkai/docs/RLM_INTEGRATION.md` | Full design doc (300+ lines) |
| `/Users/alexkamysz/AI/arkai/scripts/rlm_smoke_test_a.md` | Tiny deterministic test |
| `/Users/alexkamysz/AI/arkai/scripts/rlm_smoke_test_b.md` | Realistic adversarial test |
| `/Users/alexkamysz/.rlm-data/` | RLM data directory |

### MODIFIED FILES

| Path | Changes |
|------|---------|
| `/Users/alexkamysz/AI/rlm-fork/src/rlm_mcp_server.py` | Added OpenRouter provider (~70 lines at line 350-420), updated PROVIDER_SCHEMA enum |
| `/Users/alexkamysz/AI/rlm-fork/pyproject.toml` | Added `openai>=1.0.0` dependency |
| `/Users/alexkamysz/.claude/settings.json` | Added RLM MCP config (also used `claude mcp add`) |
| `/Users/alexkamysz/AI/arkai/docs/AIOS_BRIEF.md` | Added RLM Integration section (~100 lines) |
| `/Users/alexkamysz/AI/arkai/docs/ROADMAP.md` | Added v1.3 RLM phases + 4 decision log entries |
| `/Users/alexkamysz/AI/arkai/README.md` | Added RLM feature section + 2 comparison rows |

---

## Smoke Test Results

| Test | Status | Details |
|------|--------|---------|
| `rlm_load_context` | ✅ PASS | Loaded 5655 chars, 183 lines |
| `rlm_filter_context` | ✅ PASS | Filtered 183 → 17 lines |
| `rlm_exec` | ⚠️ PARTIAL | Runs but stdin context empty |
| `rlm_sub_query (claude-sdk)` | ✅ PASS | Got coherent response from Haiku |
| `rlm_sub_query (openrouter)` | 🔄 PENDING | Schema fixed, needs restart |
| **Sandbox blocks os** | ❌ FAIL | CRITICAL: `os.getcwd()` worked |
| **Sandbox blocks socket** | ❌ FAIL | CRITICAL: `socket` imported |

---

## What Works

1. **MCP Server Connected**: `claude mcp list` shows `rlm: ✓ Connected`
2. **Context Loading**: `rlm_load_context` stores files externally
3. **Filtering**: `rlm_filter_context` with regex patterns
4. **Chunking**: `rlm_chunk_context` with lines/chars/paragraphs
5. **Sub-queries**: `rlm_sub_query` with claude-sdk provider
6. **OpenRouter Code**: Added but schema needs restart to take effect

---

## What Needs Fixing

### CRITICAL: Sandbox Security (Phase 0.5)

The `rlm_exec` sandbox does NOT block dangerous imports:
- `import os` → ALLOWED (should be blocked)
- `import socket` → ALLOWED (should be blocked)

**Fix needed in**: `/Users/alexkamysz/AI/rlm-fork/src/rlm_mcp_server.py`

Per Chad's feedback, need to add:
```python
BLOCKED_IMPORTS = {"os", "subprocess", "socket", "requests", "urllib", "http"}
# Plus: RLIMIT_*, env var scrubbing, output size limits
```

### OpenRouter Schema

Fixed the PROVIDER_SCHEMA enum at line 152 to include "openrouter".
**Restart required** to test `provider="openrouter"`.

### rlm_exec stdin

Context not being passed to Python subprocess stdin. Low priority - exec works, just empty context.

---

## Architecture Overview

```
Claude Code (Natural Language)
    │
    ├── rlm_load_context     → Load files as external variables
    ├── rlm_filter_context   → Regex filtering (deterministic)
    ├── rlm_chunk_context    → Split by strategy
    ├── rlm_sub_query        → LLM call on chunk (budgeted)
    ├── rlm_exec             → Sandboxed Python (HITL)
    │
    ▼
~/.claude/rlm/sessions/     → Scratch outputs
    │
    ▼ (publish)
arkai (Rust spine)          → Validates evidence, places in library/
```

---

## Model Routing (Two-Lane)

| Tier | Provider | Models | Use Case |
|------|----------|--------|----------|
| 1 | claude-sdk | claude-haiku-4-5 | Root queries, synthesis |
| 2 | openrouter | gpt-4o-mini, glm-4.7, gemini-flash | Batch chunk processing |
| 3 | ollama | gemma3:12b, qwen2.5:14b | Local/offline |

---

## Next Steps (In Order)

1. **Restart Claude Code** - Pick up OpenRouter schema fix
2. **Test OpenRouter** - `rlm_sub_query(provider="openrouter", model="openai/gpt-4o-mini")`
3. **Fix Sandbox** - Add import blocklist to rlm_mcp_server.py
4. **Run Smoke Test B** - Full directory analysis with chunking
5. **Phase 1: Budget/CostTracker** - Token counting + spend limits
6. **Phase 1: Artifact Writer** - findings.json + evidence_candidates.jsonl

---

## Config Locations

| Config | Path |
|--------|------|
| MCP Settings (global) | `~/.claude/settings.json` |
| MCP Settings (project) | `~/.claude/.claude.json` (used by `claude mcp add`) |
| RLM Data | `~/.rlm-data/` |
| RLM Fork | `~/AI/rlm-fork/` |
| RLM Wrapper | `~/AI/rlm-fork/run_rlm.sh` |

---

## References

- MIT RLM Paper: https://arxiv.org/html/2512.24601v1
- richardwhiteii/rlm: https://github.com/richardwhiteii/rlm
- Design Doc: `/Users/alexkamysz/AI/arkai/docs/RLM_INTEGRATION.md`
- Roadmap: `/Users/alexkamysz/AI/arkai/docs/ROADMAP.md` (v1.3 section)
- Steelman router reference: `~/AI/steelman-engine/steelman/models/router.py`

---

## Resume Prompt

When resuming, use:
```
Continue RLM integration. We completed Phase 0 (MCP wired, smoke tests partial pass).
Priority: 1) Test OpenRouter after restart, 2) Fix sandbox security (block os/socket imports), 3) Run full Smoke Test B.
See: scout_outputs/SESSION_6_RLM_INTEGRATION.md
```

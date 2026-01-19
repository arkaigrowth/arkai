# Rolling Summary

> This file is updated after each session with cumulative context.
> Keep it under 1000 words. Compress aggressively.

---

## Current State (as of 2026-01-18)

### Project: arkai

**What it is**: Event-sourced AI pipeline orchestrator written in Rust.

**Core architecture**:
- `EventStore`: Append-only event log for durability
- `Run`: Represents a pipeline execution session
- `Artifact`: Outputs from pipeline steps (transcripts, summaries, etc.)
- `Pipeline`: Sequential step execution with safety limits
- `Evidence`: Claim verification system with source anchoring

**Recent additions**:
- RLM integration (MCP tools for recursive context analysis)
- `.ralph/` folder structure for session memory

### Active Work

**RALPH Loop Implementation** (started 2026-01-18):
- Goal: External session memory that persists across Claude Code sessions
- Approach: Standalone Python script wrapping Claude CLI
- Status: Templates created, CLI script in progress

**Obsidian Vault Reorganization** (started 2026-01-18):
- Location: `vault-sandbox/` (copy of user's Obsidian vault)
- Goal: ADHD-friendly, LLM-ready vault with minimal friction
- Approach: RALPH loop for iterative improvement (isolated .ralph/ in vault-sandbox)
- Status: Phase 0 complete, Session 1 complete, awaiting smoke test
- Key artifacts: `vault-sandbox/.ralph/runs/2026-01-18-session1/`

**Scoring Rubric** (planned):
- Goal: Evaluate note-taking systems objectively
- Criteria: Capture friction, retrieval, surfacing, modularity, maintainability, mobile

---

## Key Files

| Purpose | Path |
|---------|------|
| Rust entry point | `src/lib.rs` |
| Domain model | `src/domain/` |
| Event store | `src/core/event_store.rs` |
| RLM integration docs | `docs/RLM_INTEGRATION.md` |
| RALPH templates | `.ralph/templates/` |

---

## Recent Decisions

1. **RALPH as standalone Python** (not Rust): Faster iteration, matches common practice
2. **File-based persistence**: Simple, portable, grep-able
3. **Distillation at session end**: Compress transcript → artifacts

---

*Updated after session: 2026-01-18T23:26Z*

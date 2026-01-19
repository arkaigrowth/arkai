# RALPH Design Document

> **Status**: MVP Implementation | **Last Updated**: 2026-01-18
> **Location**: `.ralph/` in any project directory

---

## Overview

RALPH (Retrieval-Augmented Loop for Persistent History) provides durable session memory for Claude Code. It solves the "stateless LLM" problem by persisting artifacts to disk and retrieving them via tools.

**Key Principle**: Claude doesn't need "memory" if it can grep + read previous artifacts deterministically.

---

## Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                    RALPH Session Lifecycle                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ralph run "task"     ralph close      ralph finalize            │
│       │                    │                │                    │
│       ▼                    ▼                ▼                    │
│  ┌─────────┐         ┌─────────┐      ┌─────────┐               │
│  │ Create  │         │ Capture │      │ Update  │               │
│  │ session │         │ git diff│      │ memory  │               │
│  │ folder  │         │ + status│      │ files   │               │
│  └────┬────┘         └────┬────┘      └────┬────┘               │
│       │                   │                │                     │
│       ▼                   ▼                ▼                     │
│  ┌─────────┐         ┌─────────┐      ┌─────────┐               │
│  │ Render  │         │ Prompt  │      │ Archive │               │
│  │bootstrap│         │distill  │      │ session │               │
│  └────┬────┘         └────┬────┘      └────┬────┘               │
│       │                   │                │                     │
│       ▼                   ▼                ▼                     │
│   [USER]             [USER/LLM]        [DONE]                   │
│   Copy into          Produce            Ready for               │
│   Claude Code        artifacts          ralph resume            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Domain Model: arkai ↔ RALPH Mapping

| arkai Primitive | RALPH Concept | Mapping |
|-----------------|---------------|---------|
| `Run` | Session | 1:1 - A RALPH session IS a Run |
| `Event` | Tool call / action | Each significant action = Event |
| `Artifact` | Distillation output | summary.md, decisions.md = Artifacts |
| `EventStore` | `.ralph/runs/` | File-based event log |
| `Pipeline` | Distillation steps | ingest → extract → route → persist |

### Artifact Type Extensions

If integrating with arkai's Rust core, these `ArtifactType` variants would be needed:

```rust
pub enum ArtifactType {
    // Existing
    StepOutput, Transcript, Wisdom, Summary, TaskList, DocumentReference,

    // RALPH additions
    SessionBootstrap,    // bootstrap.md
    SessionSummary,      // summary.md (distilled)
    DecisionLog,         // decisions.md
    OpenQuestions,       // open_questions.md
    NextPrompt,          // next_prompt.md
    RetrievalIndex,      // retrieval_index.json
    RepoDiff,            // repo_diff.patch
}
```

### Event Type Extensions

```rust
pub enum EventType {
    // Existing
    RunStarted, RunCompleted, RunFailed,
    StepStarted, StepCompleted, StepFailed, StepRetrying,
    SafetyLimitReached,

    // RALPH additions
    SessionCreated,         // ralph run
    SessionClosed,          // ralph close
    DistillationStarted,    // begin distillation
    DistillationCompleted,  // artifacts produced
    MemoryUpdated,          // rolling_summary/decisions.log updated
    SessionResumed,         // ralph resume
}
```

---

## Folder Structure

```
.ralph/
├── runs/                           # Session archives
│   └── 2026-01-18T2326Z/          # ISO 8601 timestamp
│       ├── bootstrap.md            # Initial prompt
│       ├── mission.txt             # Task description
│       ├── repo_diff.patch         # Git changes
│       ├── git_status.txt          # Git status at close
│       ├── outputs/                # Session outputs
│       └── distill/                # Distillation artifacts
│           ├── summary.md
│           ├── decisions.md
│           ├── open_questions.md
│           ├── next_prompt.md
│           └── retrieval_index.json
│
├── memory/                         # Persistent state
│   ├── constraints.md              # Hard rules (rarely changes)
│   ├── rolling_summary.md          # Compressed context
│   ├── decisions.log               # Append-only decision log
│   └── glossary.md                 # Project-specific terms
│
└── templates/                      # Prompt templates
    ├── bootstrap.md                # Session start template
    └── distill_prompt.md           # Distillation instructions
```

---

## CLI Interface

```bash
# Start a session
ralph run "Implement external RLM-style loop"
# → Creates session folder
# → Renders bootstrap prompt
# → Prints prompt to copy into Claude Code

# Close a session
ralph close
# → Captures git diff/status
# → Prompts for distillation
# → User runs distillation (manual or automated)

# Finalize after distillation
ralph finalize
# → Validates distillation artifacts exist
# → Updates rolling memory
# → Archives session

# Resume from previous
ralph resume
# → Prints next_prompt.md from last session
# → Starts new session linked to previous

# Check status
ralph status
# → Shows active session, last session, total count

# Show history
ralph history 5
# → Lists last 5 sessions with missions
```

---

## Retrieval Protocol

The bootstrap prompt instructs Claude to:

1. **Read before acting**: Always check constraints.md and rolling_summary.md
2. **Search when uncertain**: Use `rg "keyword" .ralph/runs/` for past context
3. **Log decisions**: Append to decisions.log during session
4. **Produce artifacts**: Create all 5 distillation files at session end

This ensures retrieval is **deterministic** (file reads) not **hallucinatory** (model memory).

---

## Integration Options

### Option A: Standalone (Current MVP)

- Python script in `scripts/ralph`
- No Rust integration needed
- Works with any project

### Option B: arkai Pipeline

- Define RALPH as a pipeline in `pipelines/ralph.yaml`
- Use arkai's EventStore for persistence
- Enables: resume from failure, event replay, evidence integration

### Option C: Claude Code Hook

- Use Claude Code's hook system
- Pre-tool hook: Inject retrieval reminder
- Post-session hook: Auto-run distillation

---

## Future Enhancements

1. **Auto-distillation**: Run distillation via LLM automatically at close
2. **Embedding index**: Add vector search to retrieval_index.json
3. **Cross-project memory**: Share constraints/glossary across projects
4. **arkai integration**: Full event-sourced session tracking

---

## References

- User's original plan: In-session discussion 2026-01-18
- arkai domain model: `src/domain/`
- RLM integration: `docs/RLM_INTEGRATION.md`

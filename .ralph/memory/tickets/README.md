# Ticket System

> **Machine-readable work queue for multi-agent coordination.**
> Ralph and other orchestrators can parse these YAML files to manage work.

---

## Purpose

This directory contains **active work tickets** in YAML format. Each ticket represents a discrete unit of work assigned to a worker session.

**Why YAML?**
- Human-readable (engineers can review)
- Machine-parseable (Ralph can orchestrate)
- Git-trackable (audit trail)
- Schema-validatable (consistent structure)

---

## Ticket Lifecycle

```
BLOCKED → IN_PROGRESS → REVIEW → DONE
                ↓
              FAILED (with error notes)
```

| Status | Meaning |
|--------|---------|
| `BLOCKED` | Waiting on dependencies (see `blockedBy`) |
| `IN_PROGRESS` | Worker is actively executing |
| `REVIEW` | Work complete, awaiting master validation |
| `DONE` | Validated and merged |
| `FAILED` | Could not complete (see `error` field) |

---

## Ticket File Structure

```yaml
# Required fields
id: TICKET_ID                    # Unique identifier (SCREAMING_SNAKE_CASE)
worker: session-name             # Which Claude session owns this
status: BLOCKED                  # Current lifecycle state
branch: feat/branch-name         # Git branch for this work
created: 2026-01-26T10:00:00Z    # ISO 8601 timestamp
updated: 2026-01-26T10:00:00Z    # Last modification

# Dependencies
blockedBy: []                    # List of ticket IDs that must complete first

# Context (files worker must read)
context:
  - .claude/CLAUDE.md
  - docs/SECURITY_POSTURE.md

# Task description (what to do)
task: |
  Multi-line description of the work...

# Acceptance criteria (machine-checkable where possible)
acceptance:
  - description: "Human-readable criterion"
    cmd: "shell command to verify"           # Optional
    expect: "expected output substring"      # Optional
    file: "path/to/file"                     # Optional
    validate: jsonschema                     # Optional validation type

# Expected outputs
deliverables:
  - path: path/to/artifact
    description: What this is

# Worker fills these in when complete
proofs:
  groups_clawdbot: ""            # Output of `groups clawdbot`
  sudo_check: ""                 # Output of `sudo -l -U clawdbot`
  test_passed: false             # Boolean for test results

# Risk assessment
risk: |
  What could go wrong...

rollback: |
  How to undo if needed...

# Error tracking (if FAILED)
error: null
```

---

## Worker Workflow

1. **Read** `.claude/CLAUDE.md` for project context
2. **Read** your ticket file for task details
3. **Create branch**: `git checkout -b {branch}`
4. **Execute** task per acceptance criteria
5. **Update** ticket file:
   - Fill in `proofs` section with command outputs
   - Set `status: REVIEW`
   - Update `updated` timestamp
6. **Commit** changes to your branch
7. **Signal** master that work is ready for review

---

## Master Workflow

1. **Create** ticket files for planned work
2. **Monitor** status changes (or have Ralph poll)
3. **Validate** proofs against acceptance criteria
4. **Merge** branch if validation passes
5. **Update** status to `DONE` (or `FAILED` with `error`)
6. **Archive** completed tickets (move to `archive/` subdirectory)

---

## Automation Hooks (Future Ralph Integration)

Ralph can orchestrate workers by:

```bash
# Find available work
yq '.status' .ralph/memory/tickets/*.yaml | grep -l "BLOCKED\|IN_PROGRESS"

# Check dependencies
yq '.blockedBy' .ralph/memory/tickets/VOICE_INTAKE_V1.yaml

# Monitor for completion
watch -n 30 'yq ".status" .ralph/memory/tickets/*.yaml'

# Validate acceptance criteria
for ticket in .ralph/memory/tickets/*.yaml; do
  yq '.acceptance[].cmd' "$ticket" | while read cmd; do
    eval "$cmd"
  done
done
```

---

## Naming Conventions

- **Ticket IDs**: `SCREAMING_SNAKE_CASE` (e.g., `PHASE0_HARDEN`)
- **File names**: `{TICKET_ID}.yaml` (e.g., `PHASE0_HARDEN.yaml`)
- **Branches**: `feat/{ticket-id-lowercase}` (e.g., `feat/phase0-harden`)

---

## Relationship to Other Systems

| System | Purpose | Format |
|--------|---------|--------|
| `.ralph/memory/tickets/` | Active work queue | YAML (machine-readable) |
| `.ralph/memory/handoffs/` | Session context/history | Markdown (narrative) |
| `contracts/` | Agent-to-agent data schemas | JSON Schema |
| `docs/` | System documentation | Markdown (reference) |

---

## Validation

Tickets should conform to the schema (future: `contracts/ticket.schema.json`).

Minimal validation checklist:
- [ ] `id` is unique across all tickets
- [ ] `status` is valid enum value
- [ ] `blockedBy` references existing ticket IDs
- [ ] `context` files exist
- [ ] `acceptance` criteria are verifiable

---

*This system enables human oversight while supporting future automation.*

# ADR-0003: Queue Task Contract for Apple Service Handoff

**Status:** Accepted
**Date:** 2026-02-08
**Owner:** Arkai Claude (contract + semantics)
**Consumer:** OC Claude / Codex (execution worker in openclaw-local)

## Context

The OpenClaw agent needs to invoke Apple-native services (Notes, Reminders, iMessage) on the host Mac. The agent runs inside Docker and cannot access macOS APIs directly. We need a task handoff mechanism.

## Decision

File-based task queue with lease semantics, using the two-directory model.

### Why file-based (not SQLite/Redis/AMQP)

| Factor | File-based | Database |
|--------|-----------|----------|
| Dependencies | Zero | SQLite/Redis install |
| Docker compat | Native bind mount | Requires socket mount or network |
| Human debuggable | `cat task.json` | Requires client tool |
| Throughput | ~10-100 tasks/day | Unlimited |
| Atomicity | rename() only | Full ACID |
| Concurrency | Single worker | Multi-worker |

**Decision:** File-based. Our volume is <100 tasks/day. Complexity of a database is not justified. If throughput becomes an issue, migrate to SQLite (schema stays the same, transport changes).

### Why two directories (not single)

- `workspace/output/queue/` — agent creates requests (write direction: agent → host)
- `workspace/input/queue/` — host creates state+results (write direction: host → agent)
- Respects Docker mount semantics and ownership boundaries
- Request files are immutable; state files are mutable
- Clear audit trail: request = "what was asked", state = "what happened"

### Why lease-based (not lock-based)

- Leases auto-expire, preventing orphaned locks from crashed workers
- `lease_until` timestamp is self-healing — no manual intervention needed
- v1 assumes single worker, but lease protocol allows future multi-worker extension

## Lifecycle State Machine

```
pending ──→ leased ──→ done     (success)
   │           │
   │           ├──→ failed ──→ pending  (transient, attempts < max)
   │           │
   │           └──→ dead       (permanent error, or attempts >= max)
   │
   └──→ dead                   (pre-execution validation failure)
```

## Retry Classification

| Type | Codes | Behavior |
|------|-------|----------|
| Transient | TIMEOUT, RATE_LIMITED, CLI_ERROR, SERVICE_UNAVAILABLE | Retry after visibility_timeout |
| Permanent | INVALID_PARAMS, PERMISSION_DENIED, NOT_FOUND, UNSUPPORTED_OPERATION | Immediate dead-letter |

## Threat Model

### 1. Race condition (two workers lease same task)
- **Risk:** Low — v1 is single-worker by design
- **Mitigation:** lease_owner field enables detection; atomic rename prevents partial reads
- **Future:** If multi-worker needed, add flock() or compare-and-swap on lease_owner

### 2. Stale lease (worker crashes mid-execution)
- **Risk:** Medium — worker process could be killed
- **Mitigation:** lease_until auto-expires; next scan picks up the task
- **Residual risk:** Partially-completed actions (e.g., Note created but state not updated). Mitigated by idempotency_key on create operations.

### 3. Malicious task injection
- **Risk:** Low — producer is the OpenClaw agent, not external input
- **Mitigation:** Worker validates schema before execution; operations are allowlisted; params are validated per-service; Apple CLI bridge sanitizes inputs
- **Defense in depth:** Agent can only create files via safe_fs_write_text (no arbitrary path writes)

### 4. Queue flooding
- **Risk:** Low — agent operates under human supervision
- **Mitigation:** Max queue depth (configurable, default 100 pending); worker rejects beyond limit
- **Note:** Not implemented in v1 contract — worker-side concern

### 5. Result tampering
- **Risk:** Negligible — host is trusted
- **Mitigation:** Agent should still validate result schema before processing

## Versioning Strategy

- **v1 freeze:** Once accepted by Codex, `schema_version: "1.0.0"` is frozen
- **Additive changes:** New optional fields → minor bump (1.1.0), backwards-compatible
- **Breaking changes:** New required fields, removed fields, semantic changes → major bump (2.0.0)
- **Consumer behavior:** MUST check schema_version. Reject unknown major versions. Accept unknown minor versions (ignore new fields).
- **Change process:** Both Arkai Claude and OC Claude must agree via ADR update

## Idempotency Guidance

- **task_id:** If `idempotency_key` is set, `task_id = SHA256(key)[:12]`. Ensures same logical request always gets the same ID.
- **Duplicate detection:** Worker checks for existing state file before processing. If state file exists and is terminal (done/dead), skip.
- **Create operations:** MUST include idempotency_key to prevent duplicate Notes/Reminders. Worker checks key before executing.
- **Read/list/search:** Naturally idempotent — safe to re-execute.

## Retention Guidance

- **Settled tasks** (done/dead): Auto-delete both request + state files after `QUEUE_RETAIN_DAYS` (default 7)
- **Based on:** `completed_at` or `dead_at` timestamp, not file mtime
- **Active tasks** (pending/leased): Never cleaned up
- **Pattern match:** Only touches `{12-hex}.json` files — won't delete other files in the directory
- **Disable:** Set `QUEUE_RETAIN_DAYS=-1`

## Validation

```bash
# Validate request files
python3 contracts/validate_queue_task.py contracts/fixtures/queue_task_request_valid.json

# Validate state files
python3 contracts/validate_queue_task.py contracts/fixtures/queue_task_state_done.json

# Check expired lease
python3 contracts/validate_queue_task.py --check-lease contracts/fixtures/queue_task_state_expired_lease.json

# Validate invalid (should fail with 8 violations)
python3 contracts/validate_queue_task.py contracts/fixtures/queue_task_request_invalid.json
```

## Files

| File | Location | Purpose |
|------|----------|---------|
| Contract schema | `arkai/contracts/queue_task_contract.json` | JSON Schema definition |
| Validator | `arkai/contracts/validate_queue_task.py` | Schema validation script |
| Valid request fixture | `arkai/contracts/fixtures/queue_task_request_valid.json` | Notes read request |
| Valid state fixture | `arkai/contracts/fixtures/queue_task_state_done.json` | Completed task with result |
| Invalid fixture | `arkai/contracts/fixtures/queue_task_request_invalid.json` | 8 violations |
| Expired lease fixture | `arkai/contracts/fixtures/queue_task_state_expired_lease.json` | Crash recovery case |

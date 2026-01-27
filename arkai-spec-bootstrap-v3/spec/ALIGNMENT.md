# Spec ↔ Rust Alignment (Reality-Check)

This file prevents “parallel universe documentation”.

## Current Rust Event Envelope (src/domain/events.rs)

```rust
pub struct Event {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub run_id: Uuid,
    pub step_id: Option<String>,
    pub event_type: EventType,
    pub idempotency_key: String,
    pub payload_summary: String,
    pub status: StepStatus,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}
```

## Enums (serde JSON)

Both use `#[serde(rename_all = "snake_case")]`.

### EventType (8)
- run_started
- run_completed
- run_failed
- step_started
- step_completed
- step_failed
- step_retrying
- safety_limit_reached

### StepStatus (5)
- pending
- running
- completed
- failed
- skipped

## Storage (CRITICAL CORRECTION)

There is **no SQLite**.
Events are stored as JSONL at:

`~/.arkai/runs/{run_id}/events.jsonl`

Format = pure serde JSON. Drift risk is near-zero: what Rust serializes is exactly what is stored.

## Mapping: Target → Current

| Target Field (SYSTEM_MAP §4.1) | Current Rust Field | Status |
|---|---|---|
| event_id | id | ✅ |
| timestamp | timestamp | ✅ |
| trace_id | (none) | ❌ add |
| span_id | (none) | ❌ add |
| parent_span_id | (none) | ❌ add |
| actor | (none) | ❌ add |
| schema_version | (none) | ❌ add |
| payload_ref | (none) | ❌ future |
| inline payload | payload_summary | ✅ (MVP) |

## MVP Compatibility Policy

- **MVP envelope** = current Rust struct + optional future fields.
- **payload_ref** is v2+. MVP uses inline `payload_summary` and/or small inline `payload`.

## Recommended Migration (2–3 PRs)

1) Add nullable fields: trace_id, span_id, parent_span_id, actor, schema_version (non-breaking)
2) Populate + propagate per run/step
3) Introduce payload_ref + content-addressed payload storage; keep payload_summary for UI

## schema_id strategy

Current: implicit via `event_type` string.  
MVP rule: derive `schema_id = "arkai://events/<event_type>"`.

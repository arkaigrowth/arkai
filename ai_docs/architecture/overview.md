# arkai Architecture Overview

## System Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        arkai kernel                              │
├─────────────────────────────────────────────────────────────────┤
│  CLI (clap)                                                      │
│  ├── run <pipeline>     Execute pipeline                         │
│  ├── status <run_id>    Check run status                         │
│  ├── runs               List recent runs                         │
│  └── resume <run_id>    Resume failed run                        │
├─────────────────────────────────────────────────────────────────┤
│  Orchestrator Core                                               │
│  ├── Pipeline loader (YAML)                                      │
│  ├── Step execution with retry                                   │
│  ├── Idempotency checking                                        │
│  └── Safety limit enforcement                                    │
├─────────────────────────────────────────────────────────────────┤
│  Event Store (events.jsonl)                                      │
│  ├── Append-only event log                                       │
│  ├── Run reconstruction                                          │
│  └── Idempotency key tracking                                    │
├─────────────────────────────────────────────────────────────────┤
│  Adapters                                                        │
│  └── FabricAdapter (subprocess via `fabric -p <pattern>`)        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ stdin/stdout
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  fabric CLI                                                      │
│  └── Pattern execution (summarize, extract_wisdom, etc.)         │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

1. **Input**: User provides input via stdin or file
2. **Pipeline Load**: YAML pipeline definition parsed
3. **Safety Check**: Input validated against limits and denylist
4. **Event Log**: RunStarted event appended
5. **Step Execution**:
   - Check idempotency key
   - Log StepStarted
   - Execute via adapter (Fabric subprocess)
   - Handle success/failure/retry
   - Log StepCompleted or StepFailed
6. **Completion**: RunCompleted/RunFailed logged

## Storage Layout

```
~/.arkai/
├── config.toml             # User config
└── runs/
    └── <run_id>/
        ├── events.jsonl    # Append-only event log
        ├── artifacts/      # Step outputs (optional)
        └── state.json      # Cached reconstructed state
```

## Event Schema

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

## Idempotency

Format: `{run_id}:{step_name}:{sha256(input)[0:16]}`

Before executing any step, the orchestrator checks if a `StepCompleted` event
exists with the matching idempotency key. If found, the step is skipped.

## Safety Limits

- `max_steps`: Maximum steps per run (default: 50)
- `max_input_bytes`: Maximum input size (default: 10MB)
- `max_output_bytes`: Maximum output size (default: 10MB)
- `step_timeout_seconds`: Per-step timeout (default: 300)
- `run_timeout_seconds`: Total run timeout (default: 3600)
- `denylist_patterns`: Glob patterns to reject (secrets, keys)

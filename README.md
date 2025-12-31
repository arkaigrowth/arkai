# arkai

Event-sourced AI pipeline orchestrator using [Fabric](https://github.com/danielmiessler/fabric) as an external dependency.

## Installation

```bash
cargo install --path .
```

## Prerequisites

- [Fabric](https://github.com/danielmiessler/fabric) installed and available in PATH
- Rust 1.70+ with Cargo

## Usage

### Run a pipeline

```bash
# From stdin
echo "Claude is an AI assistant made by Anthropic." | arkai run hello

# From file
arkai run hello --input article.txt
```

### Check run status

```bash
arkai status <run_id>
```

### List recent runs

```bash
arkai runs --limit 10
```

### Resume a failed run

```bash
arkai resume <run_id>
```

## Pipeline Definition

Pipelines are defined in YAML format in the `pipelines/` directory:

```yaml
name: hello
description: Minimal pipeline that summarizes input via Fabric

safety_limits:
  max_steps: 1
  step_timeout_seconds: 60

steps:
  - name: summarize
    adapter: fabric
    action: summarize
    input_from: pipeline_input
    retry_policy:
      max_attempts: 2
      initial_delay_ms: 1000
```

## Event Storage

All runs are logged to `~/.arkai/runs/<run_id>/events.jsonl` as append-only event logs.

## License

MIT

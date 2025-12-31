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

## Content Ingestion

arkai can ingest and process content from YouTube videos and web pages, storing the results in a searchable library.

### Ingest a YouTube video

```bash
# Auto-detects YouTube URLs
arkai ingest "https://youtube.com/watch?v=dQw4w9WgXcQ" --tags "music,classic"

# Explicitly specify type
arkai ingest "https://youtube.com/watch?v=xyz" --content-type youtube
```

### Ingest a web page

```bash
arkai ingest "https://example.com/article" --tags "tech,ai"
```

### Browse your library

```bash
# List all items
arkai library

# Filter by type
arkai library --content-type youtube

# Search
arkai search "transformer architecture"

# Show details
arkai show <content_id>
arkai show <content_id> --full  # Include artifact contents

# Reprocess (if patterns improve)
arkai reprocess <content_id>
```

## Library Structure

```
~/.arkai/
├── catalog.json              # Searchable index
└── library/
    └── <content_id>/         # SHA256(url)[0:16]
        ├── metadata.json     # Title, URL, type, date
        ├── fetch.md          # Original content
        ├── wisdom.md         # Extracted wisdom
        └── summary.md        # Summary
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

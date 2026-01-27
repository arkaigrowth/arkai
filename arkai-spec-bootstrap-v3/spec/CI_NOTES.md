# CI Notes (Tool-Agnostic)

Goal: fail fast when schemas drift.

Minimum checks:
- validate YAML parses
- validate JSON Schema Draft 2020-12
- ensure `$id` uniqueness
- ensure event schemas reference base envelope
- optional: validate sample events from JSONL against schema

Implementation can be Rust (schemars tests), Python (jsonschema), or Node (ajv 2020-12).

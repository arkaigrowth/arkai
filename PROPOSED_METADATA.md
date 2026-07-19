# Proposed GitHub Metadata

These are suggestions for repository settings that live on GitHub and cannot be
set from the repo contents. Apply them in the repository settings after review.

## Description

> Rust orchestration layer for AI pipelines: event-sourced state, idempotent
> resume, and a full audit trail on top of Fabric.

## Topics

- rust
- ai
- fabric
- pipelines
- event-sourcing
- orchestration
- cli
- llm

## Homepage

Leave unset, or point to the AI OS architecture doc once a hosted docs page
exists. There is no live site to link yet.

## Notes

- Keep the default branch as `main`.
- The repository currently ships without CI. If you add one, a simple
  `cargo build` plus `cargo test` and the `services/inbox` pytest suite would
  cover the two language surfaces.

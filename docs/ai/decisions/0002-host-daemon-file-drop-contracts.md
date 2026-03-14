# ADR-0002: Host Daemon File-Drop Contracts for OpenClaw Inputs

Date: 2026-02-08
Status: Accepted
Owner: arkai coach thread

## Context

`arkai` owns the cross-repo contracts that describe how host-side Google/Gmail daemons hand data into `openclaw-local`. The daemon implementations live in `arkai-gmail`, but the schema and contract authority live in this repo. The daemon contracts already assume a common pattern: one-shot host processes run on a schedule, keep OAuth tokens on the host, and write JSON artifacts into bind-mounted `openclaw-local` input directories for read-only agent consumption.

Without a canonical ADR in `arkai`, the contracts reference ADR-0002 in prose but do not have an in-repo governing decision artifact.

## Decision

Adopt the host-daemon file-drop pattern as the governing contract model for daemon-fed OpenClaw inputs.

1. Host-side daemons run outside Docker as one-shot scheduled jobs, not as long-running in-container services.
2. OAuth tokens and other provider credentials stay on the host and never enter the OpenClaw container.
3. Daemons write structured JSON artifacts into `~/AI/openclaw-local/workspace/input/...` paths that are readable inside the container.
4. OpenClaw agents consume those artifacts through scoped read tools (`safe_fs_read_text` / `safe_fs_list_dir`) rather than direct network or provider API access.
5. Contract ownership stays in `arkai/contracts/`, while implementation ownership for Gmail/calendar daemon code stays in `arkai-gmail`.
6. Email and calendar daemon contracts both point to this ADR as the governing decision until a replacement ADR supersedes it.

## Consequences

- Cross-repo ownership stays explicit: `arkai` defines the contracts, `arkai-gmail` implements the daemons, `openclaw-local` consumes the results.
- The security boundary is cleaner because provider tokens remain on the host.
- OpenClaw input schemas are stable and testable without coupling contract evolution to daemon implementation details.
- Contract metadata must include explicit owner/semver/ADR pointers so operators can resolve authority without inference.

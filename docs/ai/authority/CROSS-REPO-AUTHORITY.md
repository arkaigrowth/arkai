# Cross-Repo Authority Map

Date: 2026-02-09
Status: Accepted
Owner: arkai coach thread

## Purpose

Define canonical ownership boundaries across `arkai`, `openclaw-local`, and `fabric-arkai`.
This is the only file that should contain the full authority matrix.

## Canonical Ownership Matrix

| Concern | Canonical Repo | Canonical Path | Non-Canonical Behavior |
| --- | --- | --- | --- |
| Coordination protocol for OpenClaw runtime + Docker security posture | `openclaw-local` | `~/AI/openclaw-local/docs/ai/` | Keep one-line pointers only |
| Arkai orchestration contracts and schemas | `arkai` | `~/AI/arkai/contracts/` | Keep one-line pointers only |
| Arkai repo-level coordination state | `arkai` | `~/AI/arkai/docs/ai/` | Keep one-line pointers only |
| Fabric pattern library content and pattern taxonomy | `fabric-arkai` | `~/AI/fabric-arkai/` | Keep one-line pointers only |
| OpenClaw queue worker runtime behavior and policy | `openclaw-local` | `~/AI/openclaw-local/docs/` and `~/AI/openclaw-local/scripts/` | Keep one-line pointers only |
| Gmail triage pipeline implementation | `arkai-gmail` | `~/AI/arkai-gmail/` | Keep one-line pointers only |

## Pointer Policy

When this repo is not canonical for a concern, use this one-line pointer format:

`Canonical authority: See /absolute/path/to/canonical/file-or-directory`

Do not duplicate tables or decision text outside this file.

## Contract Policy

Every cross-repo schema must declare:
- `owner_repo`: one of `arkai`, `openclaw-local`, `fabric-arkai`, `arkai-gmail`
- `semver`: contract version in `MAJOR.MINOR.PATCH`
- `adr_pointer`: absolute path to the governing ADR or decision doc

Change control:
- No contract changes without updating the authority map and ADR pointer in the same PR.
- Breaking changes require semver major bump and explicit migration notes.
- Non-breaking additive changes require semver minor bump.
- Patch changes are for clarifications/validation fixes only.

## Operational Guardrails

- `docs/ai/SHARED-STATE.md` remains single-writer (`Owner` field).
- State-sync commits should be isolated with `git commit --only ~/AI/arkai/docs/ai/SHARED-STATE.md ...`.
- Non-owner sessions must publish handoffs in `~/AI/arkai/docs/ai/handoffs/`.

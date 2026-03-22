# ADR-0001: Arkai Coordination Guardrails

Date: 2026-02-09
Status: Accepted
Author: codex coach thread

## Context

`arkai` is a large, multi-session repository with legacy coordination artifacts in `.ralph/` and active cross-repo work with `openclaw-local` and `fabric-arkai`.
Without a canonical governance layer in-repo, context drifts across handoffs and tools.

## Decision

1. `docs/ai/` is the canonical coordination workspace for this repo.
2. `docs/ai/SHARED-STATE.md` is single-writer, enforced by explicit `Owner` and verification checks.
3. Non-owner sessions must publish updates in `docs/ai/handoffs/YYYY-MM-DD-tool-summary.md`.
4. Cross-repo authority is centralized in `docs/ai/authority/CROSS-REPO-AUTHORITY.md` and referenced by one-line pointers elsewhere.
5. Docs state-sync commits for `SHARED-STATE.md` must be isolated.
6. `.ralph/` stays read-only during migration; only high-signal artifacts are promoted.

## Consequences

- Session startup becomes deterministic for Codex and Claude Code.
- Shared-state ownership collisions are reduced by policy and script checks.
- Cross-repo decisions should drift less due to canonical+pointer discipline.
- Legacy `.ralph` cleanup becomes controlled through inventory labels and TTL windows.
- Additional process overhead is introduced for non-owner updates and handoff discipline.

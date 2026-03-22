# Shared Context (Read First)

This file is the cross-thread context for work in `arkai`.

## Portal
- Thread tracker: `/Users/alexkamysz/AI/arkai/docs/THREADS.md`
- AI coordination protocol: `/Users/alexkamysz/AI/arkai/docs/ai/README.md`
- Canonical authority: See `/Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md`

## Ground Rules
- Default to read-only scans before edits on high-risk work.
- Keep changes scoped and reversible.
- Keep `.ralph` read-only during migration to `docs/ai`.
- State-sync updates to `SHARED-STATE.md` are owner-only and should be isolated commits.

## Current Decisions
- `docs/ai/SHARED-STATE.md` is single-writer by design.
- `docs/ai/handoffs/` is required for non-owner updates.
- `docs/ai/authority/CROSS-REPO-AUTHORITY.md` is the only authority matrix location.

## Open Questions
- Confirm whether `fabric-arkai` should mirror the same `docs/ai` protocol or keep pointer-only minimal docs.

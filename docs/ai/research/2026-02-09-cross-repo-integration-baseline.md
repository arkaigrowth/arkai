# Cross-Repo Integration Baseline

Date: 2026-02-09

## Summary

This report captures high-signal integration facts migrated from `.ralph` into canonical `docs/ai` context for ongoing `arkai` coordination.

## Key Findings

- `openclaw-local` already has a mature `docs/ai` protocol with single-writer shared state and ADR-backed cross-repo policy.
- Prior arkai handoffs confirm a canonical+pointer model and gateway-boundary integration pattern for `openclaw-local`.
- `arkai` should centralize authority mapping in one file and avoid matrix duplication in `AGENTS.md` or `CONTEXT.md`.
- Queue/task contracts are already schema-versioned in `/Users/alexkamysz/AI/arkai/contracts/queue_task_contract.json` and should follow explicit owner+ADR pointer policy.
- Legacy `.ralph` artifacts contain useful history but should be promoted selectively as distilled `docs/ai` artifacts.

## References

- `/Users/alexkamysz/AI/arkai/.ralph/memory/handoffs/2026-02-07-openclaw-email-integration.md`
- `/Users/alexkamysz/AI/arkai/.ralph/memory/handoffs/2026-02-07-daemon-contract-delivered.md`
- `/Users/alexkamysz/AI/arkai/.ralph/memory/handoffs/2026-02-08-calendar-daemon-and-bridge-research.md`
- `/Users/alexkamysz/AI/arkai/.ralph/memory/handoffs/2026-02-08-launchd-integration-spec.md`
- `/Users/alexkamysz/AI/openclaw-local/docs/ai/decisions/0001-cross-repo-coordination-pattern.md`
- `/Users/alexkamysz/AI/openclaw-local/docs/ai/decisions/0002-email-triage-integration-pattern.md`

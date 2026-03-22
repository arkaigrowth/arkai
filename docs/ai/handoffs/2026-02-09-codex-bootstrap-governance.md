# Codex Governance Bootstrap Handoff

Date: 2026-02-09

## State

- Bootstrapped `arkai` coordination scaffold:
  - `/Users/alexkamysz/AI/arkai/AGENTS.md`
  - `/Users/alexkamysz/AI/arkai/docs/CONTEXT.md`
  - `/Users/alexkamysz/AI/arkai/docs/THREADS.md`
  - `/Users/alexkamysz/AI/arkai/docs/ai/README.md`
  - `/Users/alexkamysz/AI/arkai/docs/ai/SHARED-STATE.md`
- Added hard checks in `/Users/alexkamysz/AI/arkai/scripts/verify_ai_docs.sh`:
  - fail if `SHARED-STATE.md` is a symlink
  - fail non-owner edits to `SHARED-STATE.md`
  - require non-owner updates to include a handoff file
- Created canonical authority map:
  - `/Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md`
- Produced migration inventory with labels and TTL:
  - `/Users/alexkamysz/AI/arkai/docs/ai/migration/2026-02-09-ralph-inventory.md`
- Migrated high-signal legacy context into canonical docs:
  - ADR: `docs/ai/decisions/0001-arkai-coordination-guardrails.md`
  - Research: `docs/ai/research/2026-02-09-cross-repo-integration-baseline.md`

## Next Steps

1. Add one-line canonical authority pointers in `openclaw-local` and `fabric-arkai`.
2. Decide whether to port `verify_ai_docs.sh` into pre-commit/pre-push checks for `arkai`.
3. Execute archive/drop TTL lifecycle for `.ralph` on schedule.
4. Promote additional high-signal `.ralph` records only when they directly affect active work.

## Open Questions

- Should `fabric-arkai` adopt full `docs/ai` protocol or remain pointer-only?
- Do you want cross-repo pointer updates applied now from this thread (requires write access outside `arkai`)?
- Should `docs/ai/SHARED-STATE.md` owner remain `coach`, or rotate per sprint/phase?

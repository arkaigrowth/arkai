# RALPH Migration Inventory

Date: 2026-02-09
Status: Drafted

## Scope

This inventory maps legacy `.ralph/` artifacts into `docs/ai` targets without editing `.ralph`.
Labels are exactly one of: `canonical`, `pointer`, `archive`, `drop`.

TTL policy:
- `archive`: retain until `Date + TTL`, then review for purge.
- `drop`: delete at `Date + TTL` unless explicitly rescued.

## Inventory Map

| Source Artifact | Target in `docs/ai` | Label | TTL |
| --- | --- | --- | --- |
| `.ralph/memory/constraints.md` | `docs/ai/decisions/0001-arkai-coordination-guardrails.md` | canonical | n/a |
| `.ralph/memory/decisions.log` | `docs/ai/migration/archive/decisions.log` (deferred) | archive | 180d |
| `.ralph/memory/rolling_summary.md` | `docs/ai/migration/archive/rolling_summary.md` (deferred) | archive | 90d |
| `.ralph/memory/handoffs/2026-02-07-openclaw-email-integration.md` | `docs/ai/handoffs/2026-02-09-codex-bootstrap-governance.md` | canonical | n/a |
| `.ralph/memory/handoffs/2026-02-07-daemon-contract-delivered.md` | `docs/ai/handoffs/2026-02-09-codex-bootstrap-governance.md` | canonical | n/a |
| `.ralph/memory/handoffs/2026-02-08-calendar-daemon-and-bridge-research.md` | `docs/ai/research/2026-02-09-cross-repo-integration-baseline.md` | canonical | n/a |
| `.ralph/memory/handoffs/2026-02-08-launchd-integration-spec.md` | `docs/ai/research/2026-02-09-cross-repo-integration-baseline.md` | pointer | n/a |
| `.ralph/memory/handoffs/2026-02-06-openclaw-local-security-audit.md` | `docs/ai/research/2026-02-09-cross-repo-integration-baseline.md` | pointer | n/a |
| `.ralph/memory/handoffs/2026-01-20-obsidian-session5.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-25-clawdbot-l1-voice-integration.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-25-master-coordination.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-26-gmail-layer-abc.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-26-master-session-2.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-26-triage-sidecar-phase0-gmail.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-27-gmail-pipeline-complete.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-27-master-session-3.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-27-safe-super-assistant-blueprint.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-29-voice-phase5-complete.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-30-clawdbot-websearch-complete.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-30-inbox-review-architecture-locked.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-30-openrouter-complete-voice-next.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-31-claudia-tools-fixed.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-01-31-inbox-complete-sentinel-planning.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-02-01-openclaw-session2-gmail-security.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/2026-02-01-openclaw-vps-deployment.md` | `docs/ai/migration/archive/` (deferred) | archive | 180d |
| `.ralph/memory/handoffs/arkai-voice-builder.md` | `docs/ai/migration/archive/` (deferred) | archive | 90d |
| `.ralph/memory/research/clawdbot-tool-logs.md` | `docs/ai/research/` (future distillation) | archive | 120d |
| `.ralph/memory/research/kimi-k2-openclaw-research.md` | `docs/ai/research/` (future distillation) | archive | 120d |
| `.ralph/memory/research/openrouter-tool-calling-research.md` | `docs/ai/research/` (future distillation) | archive | 120d |
| `.ralph/memory/reviews/PHASE_1_1.5_CODE_REVIEW.md` | `docs/ai/migration/archive/` (deferred) | archive | 120d |
| `.ralph/memory/specs/VOICE_PIPELINE_V2.1_BUILD_SPEC.md` | `docs/ai/research/2026-02-09-cross-repo-integration-baseline.md` | pointer | n/a |
| `.ralph/memory/prompts/PHASE_1.6_HARDENING.md` | stale prompt corpus | drop | 30d |
| `.ralph/memory/prompts/PHASE_2_3_BUILD_SESSION.md` | stale prompt corpus | drop | 30d |
| `.ralph/memory/prompts/PHASE_4_VPS_BUILD_SESSION.md` | stale prompt corpus | drop | 30d |
| `.ralph/memory/prompts/VOICE_PIPELINE_BUILD_SESSION.md` | stale prompt corpus | drop | 30d |
| `.ralph/memory/tickets/README.md` | `docs/THREADS.md` + `docs/ai/handoffs/` protocol | pointer | n/a |
| `.ralph/memory/tickets/GMAIL_CORE_PIPELINE.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/GMAIL_HYBRID_CRITIC.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/GMAIL_IMPL_LAYER_A.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/GMAIL_PROVIDER_ABSTRACTION.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/GMAIL_SCAFFOLD_V1.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/PHASE0_HARDEN.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/RALPH_VALIDATE_V1.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/TTS_ELEVENLABS_V1.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/VOICE_INTAKE_V1.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/VOICE_PHASE_2_3.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/VOICE_PHASE_4.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/memory/tickets/VOICE_PHASE_5_INTEGRATION.yaml` | superseded work queue artifact | drop | 45d |
| `.ralph/templates/bootstrap.md` | deprecated bootstrap template | drop | 30d |
| `.ralph/templates/distill_prompt.md` | deprecated distillation template | drop | 30d |

## Notes

- `.ralph` remains read-only during transition.
- High-signal migration is limited to explicitly selected artifacts in this inventory.
- Archive/drop TTL windows start on 2026-02-09.

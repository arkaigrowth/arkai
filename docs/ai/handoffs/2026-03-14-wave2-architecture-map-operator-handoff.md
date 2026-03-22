# Wave 2 Architecture Map Deliverables and Handoff

Date: 2026-03-14  
Generated (UTC): 2026-03-14T05:24:26Z  
Scope: Discovery/documentation synthesis only (no runtime/config/code behavior changes).

## Baseline Metadata
- Canonical repo path: `/Users/alexkamysz/AI/arkai`
- Canonical repo branch: `main`
- Canonical repo commit: `75237973894d5f6cd5202aa914e12ff928ade6e9`
- Coordination workspace path: `/Users/alexkamysz/intent/workspaces/code-config/arkai`
- Coordination workspace branch: `map-arkai-architecture-evidence`
- Coordination workspace commit: `990e4cb8539d2ac4459d9bd84bba2e50add33782`
- Wave 1 synthesis inputs: task notes `18922839-b5f7-44c8-8865-8d92e8e89fb3`, `55e4bfc3-7963-4903-9f9e-f5f309054be5`, `6b3e1b59-6399-4eed-950a-41cdb03962c9`, `671435a6-f52f-4960-ae7f-fb79cd04c06c`.

## 1) Repo Identity and Stack
### Observed Facts
- Canonical architecture/governance source repo is `/Users/alexkamysz/AI/arkai` on `main`.
- Major implementation roots are present at `src/` (Rust), `services/inbox/` (Python), `services/voice/` (Python), and `contracts/` (JSON contracts/schemas).
- `services/voice/validator.py` loads schemas from `contracts/` via `CONTRACTS_DIR`.

### Inferred
- Operational stack is a mixed Rust + Python system with JSON-schema contracts as a shared integration boundary.

### Unknown
- No single in-repo machine-readable stack manifest was found that authoritatively enumerates all runtimes and version constraints across all subsystems.

Confidence: High. Rationale: identity and stack roots are directly evidenced by repo paths and runtime code references.

## 2) Ownership Boundaries and Canonical Authority
### Observed Facts
- `AGENTS.md` and `docs/CONTEXT.md` both point to `docs/ai/authority/CROSS-REPO-AUTHORITY.md` as canonical authority.
- `docs/ai/SHARED-STATE.md` is single-writer with `Owner: coach thread (single writer)`.
- Authority matrix assigns canonical ownership lanes: `openclaw-local` (coordination/runtime policy), `arkai` (contracts/schemas), `fabric-arkai` (pattern library/taxonomy), `arkai-gmail` (Gmail triage pipeline implementation).

### Inferred
- Non-owner contributors should publish dated handoffs rather than editing shared state directly.

### Unknown
- External repos’ internal governance state is not verified in this synthesis pass.

Confidence: High. Rationale: canonical-owner and single-writer constraints are explicit in in-repo governance docs.

## 3) Structure Inventory (Top-Level + Major Subsystems)
### Observed Facts
- Top-level structure includes `src/`, `services/`, `contracts/`, `docs/`, `docs/ai/`, `patterns/`, `pipelines/`, `tests/`, and `ai_docs/`.
- Major subsystem directories under `src/` include `adapters`, `cli`, `config`, `core`, `domain`, `evidence`, `ingest`, and `library`.
- Service roots include `services/inbox/` and `services/voice/`.

### Inferred
- `src/`, `services/`, `contracts/`, and `docs/ai/authority/` are the primary operator-facing architecture surfaces.

### Unknown
- Lifecycle intent for several auxiliary roots (`vault-sandbox/`, `.ralph/`, ad hoc research logs) is not uniformly codified as active vs archival in one authoritative file.

Confidence: Medium. Rationale: directory inventory is complete at depth-2, but lifecycle intent outside governance docs requires interpretation.

## 4) Contracts Map (Schemas, ADR Pointers, Semver/Control Model)
### Observed Facts
- Contract artifacts are centralized under `/Users/alexkamysz/AI/arkai/contracts/`.
- Authority policy requires cross-repo schemas to declare `owner_repo`, `semver`, and `adr_pointer`.
- Cross-repo contract JSONs checked (`queue_task_contract.json`, `email_daemon_contract.json`, `calendar_daemon_contract.json`) do not contain those explicit keys.
- `version` fields are present in the three checked cross-repo contracts (`1.1.0`, `1.0.0`, `1.0.0`).
- `email_daemon_contract.json` and `calendar_daemon_contract.json` reference ADR-0002 in description text.
- `docs/ai/decisions/` currently contains `.gitkeep` and `0001-arkai-coordination-guardrails.md`.

### Inferred
- There is policy-to-implementation metadata drift: semver-like behavior exists via `version`, but required explicit metadata keys are absent.

### Unknown
- Canonical in-repo ADR artifact for referenced ADR-0002 is not located under `docs/ai/decisions/`.

Confidence: High. Rationale: policy requirements and contract key absence are directly reproducible with grep against explicit files.

## 5) Active vs Stale Classification (with Evidence)
### Observed Facts
- Governance docs are actively referenced by startup protocol (`AGENTS.md`, `docs/CONTEXT.md`) and are currently in-flight in canonical repo status scans.
- Architecture/design docs and key contract docs show recent commits within the sampled 180-day window.
- Voice runtime code actively consumes `contracts/`.

### Inferred
- `active`: `docs/ai` governance set, `contracts/` schemas used by runtime validators, and current subsystem design docs tied to current workflows.
- `reference-only`: files with recent edits but weak direct consumption signals (for example `docs/ARCHITECTURE.md`, `contracts/README.md`, `services/inbox/README.md`) and migration-marked legacy memory artifacts.

### Unknown
- `ai_docs/architecture/overview.md` canonical status remains ambiguous relative to `docs/AI_OS_ARCHITECTURE.md` and `docs/ARCHITECTURE.md`.
- No high-confidence stale classification is asserted for the sampled core artifact set.

Confidence: Medium. Rationale: classification is evidence-backed but still heuristic because activity is inferred from recency/reference signals rather than a single lifecycle registry.

## 6) Cross-Repo Consumption Map (`arkai` <-> `openclaw-local` <-> `fabric-arkai` <-> `arkai-gmail`)
### Observed Facts
- Authority matrix marks `openclaw-local`, `fabric-arkai`, and `arkai-gmail` as canonical owners for specific lanes.
- `email_daemon_contract.json` and `calendar_daemon_contract.json` bind host-side daemon scripts in `~/AI/arkai-gmail` to output directories under `~/AI/openclaw-local/workspace/input/...`.
- Queue-task contract/ADR evidence ties producer/consumer lane behavior to OpenClaw runtime context.
- No direct `fabric-arkai` references were observed in `contracts/` or `services/` scans.

### Inferred
- Runtime coupling is strongest between `arkai` contracts and `openclaw-local`/`arkai-gmail` execution paths.
- `fabric-arkai` is currently a governance/pointer dependency, not a direct runtime dependency in checked contracts/services surfaces.

### Unknown
- External repos were not inspected directly here, so external lane implementations are treated as pointers unless verified in this repo.

Confidence: Medium. Rationale: in-repo linkage evidence is strong, but external-side implementation was intentionally out of scope.

## 7) Operator Runbook (If X, Go Here First)
### Observed Facts
- Startup protocol and canonical authority pointers are documented in `AGENTS.md`, `docs/CONTEXT.md`, and `docs/ai/README.md`.
- SHARED-STATE single-writer rules and handoff workflow are documented in `docs/ai/SHARED-STATE.md` and authority docs.

### Inferred
- Recommended first-stop routing for operators:
1. If ownership/canonical-repo is unclear: open `docs/ai/authority/CROSS-REPO-AUTHORITY.md`.
2. If work coordination state is needed: open `docs/ai/SHARED-STATE.md` and confirm `Owner` before edits.
3. If non-owner and state updates are needed: write a dated file in `docs/ai/handoffs/`.
4. If a contract/runtime mismatch appears: inspect `contracts/*.json`, `contracts/README.md`, and `services/voice/validator.py`.
5. If cross-repo daemon path behavior is involved: inspect `contracts/email_daemon_contract.json` and `contracts/calendar_daemon_contract.json` first.
6. If docs protocol uncertainty exists: inspect `AGENTS.md`, `docs/CONTEXT.md`, `docs/ai/README.md`, then run `scripts/verify_ai_docs.sh`.

### Unknown
- No single runbook currently encodes escalation order across all external repos; this section is a synthesis of existing governance documents.

Confidence: High. Rationale: routing steps map directly to explicit canonical docs and repeatedly referenced operator entry points.

## 8) Index Table
### Observed Facts
- Key architecture/governance paths and owners are identifiable from authority docs, contract files, and runtime references.

| path | owner | status | consumers | where to edit | confidence |
| --- | --- | --- | --- | --- | --- |
| `/Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` | `arkai` | active | all operators; Wave 1/2 analysis | this file | High |
| `/Users/alexkamysz/AI/arkai/docs/ai/SHARED-STATE.md` | `coach thread (single writer)` | active (guarded) | all operators | owner only; non-owners use handoffs | High |
| `/Users/alexkamysz/AI/arkai/docs/ai/handoffs/` | `arkai` (multi-writer handoff lane) | active | non-owner contributors | add dated handoff files | High |
| `/Users/alexkamysz/AI/arkai/contracts/` | `arkai` | active | `services/voice`, daemon validators, operators | contract JSONs + contract docs | High |
| `/Users/alexkamysz/AI/arkai/contracts/queue_task_contract.json` | `arkai` (contract lane references `openclaw-local`) | active | OpenClaw queue worker lane | this file + related ADR docs | High |
| `/Users/alexkamysz/AI/arkai/contracts/email_daemon_contract.json` | `arkai` (implementation lane `arkai-gmail`) | active | `arkai-gmail` daemon; OpenClaw input consumers | this file; corresponding canonical repo impl | High |
| `/Users/alexkamysz/AI/arkai/contracts/calendar_daemon_contract.json` | `arkai` (implementation lane `arkai-gmail`) | active | calendar daemon; OpenClaw input consumers | this file; corresponding canonical repo impl | High |
| `/Users/alexkamysz/AI/arkai/services/voice/validator.py` | `arkai` | active | voice service validation path | this file | High |
| `/Users/alexkamysz/AI/arkai/docs/ARCHITECTURE.md` | `arkai` | reference-only | operators (orientation) | this file | Medium |
| `/Users/alexkamysz/AI/arkai/contracts/README.md` | `arkai` | reference-only | contract editors | this file | Medium |
| `/Users/alexkamysz/AI/arkai/services/inbox/README.md` | `arkai` | reference-only | inbox contributors | this file | Medium |
| `/Users/alexkamysz/AI/arkai/ai_docs/architecture/overview.md` | `arkai` | unknown | architecture readers | clarify canonical status first | Low |
| `/Users/alexkamysz/AI/openclaw-local/` | `openclaw-local` | external-pointer | queue worker + input directory runtime | edit in canonical external repo | Medium |
| `/Users/alexkamysz/AI/fabric-arkai/` | `fabric-arkai` | external-pointer | pattern/taxonomy governance pointer | edit in canonical external repo | Medium |
| `/Users/alexkamysz/AI/arkai-gmail/` | `arkai-gmail` | external-pointer | Gmail triage/calendar daemon implementation | edit in canonical external repo | Medium |

### Inferred
- Statuses in this table combine direct evidence and Wave 1 rubric outputs.

### Unknown
- External repo path health and internal branch/commit states were not validated in this pass.

Confidence: Medium. Rationale: high confidence for in-repo rows; medium for external-pointer rows due no direct external inspection.

## 9) Risks, Gaps, Recommendations (Minimal, High-Confidence Only)
### Observed Facts
- Authority policy requires `owner_repo`, `semver`, `adr_pointer`; current cross-repo contract JSONs checked do not include these keys.
- ADR-0002 is referenced by daemon contracts, but `docs/ai/decisions/` currently exposes ADR-0001 only.
- Canonical authority purpose sentence names three repos while matrix includes `arkai-gmail` as an explicit lane.

### Inferred
- Current governance is actionable but carries metadata and documentation consistency debt that can create operator confusion.

### Unknown
- Whether these gaps are intentionally deferred or accidental is not documented in a single, explicit waiver note.

### Recommendations
1. Add explicit `owner_repo`, `semver`, and `adr_pointer` fields to cross-repo contracts (or publish a documented exception policy).
2. Reconcile ADR-0002 references by either adding canonical ADR artifact path or updating contract references.
3. Align authority doc intro scope sentence with matrix rows (include `arkai-gmail` explicitly).

Confidence: High. Rationale: each recommendation maps to a directly observed, reproducible mismatch.

## Evidence Manifest (Key Claims)
Schema: `claim | label | path(s) | command`

| claim | label | path(s) | command |
| --- | --- | --- | --- |
| Canonical repo baseline is `main` at `75237973894d5f6cd5202aa914e12ff928ade6e9` (scan timestamp in this handoff). | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai` | `git -C /Users/alexkamysz/AI/arkai rev-parse --abbrev-ref HEAD; git -C /Users/alexkamysz/AI/arkai rev-parse HEAD; date -u +"%Y-%m-%dT%H:%M:%SZ"` |
| Coordination workspace identity is `/Users/alexkamysz/intent/workspaces/code-config/arkai` on `map-arkai-architecture-evidence` at `990e4cb8539d2ac4459d9bd84bba2e50add33782`. | VERIFIED-IN-REPO | `/Users/alexkamysz/intent/workspaces/code-config/arkai` | `pwd; git rev-parse --abbrev-ref HEAD; git rev-parse HEAD` |
| AGENTS/CONTEXT point to CROSS-REPO-AUTHORITY as canonical authority location. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/AGENTS.md`; `/Users/alexkamysz/AI/arkai/docs/CONTEXT.md` | `rg -n "Canonical authority|CROSS-REPO-AUTHORITY" /Users/alexkamysz/AI/arkai/AGENTS.md /Users/alexkamysz/AI/arkai/docs/CONTEXT.md` |
| SHARED-STATE owner is explicitly `coach thread (single writer)`. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/docs/ai/SHARED-STATE.md` | `rg -n "^Owner:" /Users/alexkamysz/AI/arkai/docs/ai/SHARED-STATE.md` |
| Authority matrix assigns OpenClaw coordination/runtime policy lane to `openclaw-local`. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` | `rg -n "Coordination protocol for OpenClaw runtime|OpenClaw queue worker runtime behavior" /Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` |
| Authority matrix assigns pattern library/taxonomy lane to `fabric-arkai`. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` | `rg -n "Fabric pattern library content and pattern taxonomy" /Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` |
| Authority matrix assigns Gmail triage pipeline implementation lane to `arkai-gmail`. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` | `rg -n "Gmail triage pipeline implementation" /Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` |
| External canonical lane paths are pointers to external repos and were not inspected in this pass. | EXTERNAL-POINTER | `/Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` | `nl -ba /Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md | sed -n '14,22p'` |
| Cross-repo schema policy requires `owner_repo`, `semver`, `adr_pointer`. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` | `rg -n "owner_repo|semver|adr_pointer" /Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` |
| Checked cross-repo contracts do not contain explicit `owner_repo`/`semver`/`adr_pointer` keys. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/contracts/queue_task_contract.json`; `/Users/alexkamysz/AI/arkai/contracts/email_daemon_contract.json`; `/Users/alexkamysz/AI/arkai/contracts/calendar_daemon_contract.json` | `for f in /Users/alexkamysz/AI/arkai/contracts/queue_task_contract.json /Users/alexkamysz/AI/arkai/contracts/email_daemon_contract.json /Users/alexkamysz/AI/arkai/contracts/calendar_daemon_contract.json; do rg -n '"owner_repo"|"semver"|"adr_pointer"' "$f" || echo NO_MATCH; done` |
| Email daemon contract binds `arkai-gmail` script/venv to OpenClaw workspace email input path. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/contracts/email_daemon_contract.json` | `rg -n "arkai-gmail|openclaw-local|version|ADR-0002" /Users/alexkamysz/AI/arkai/contracts/email_daemon_contract.json` |
| Calendar daemon contract binds `arkai-gmail` script/venv to OpenClaw workspace calendar input path. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/contracts/calendar_daemon_contract.json` | `rg -n "arkai-gmail|openclaw-local|version|ADR-0002" /Users/alexkamysz/AI/arkai/contracts/calendar_daemon_contract.json` |
| Queue-task contract publishes semver-like `version` field `1.1.0`. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/contracts/queue_task_contract.json` | `rg -n '"version"' /Users/alexkamysz/AI/arkai/contracts/queue_task_contract.json` |
| Decisions directory currently includes ADR-0001 and no ADR-0002 artifact. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/docs/ai/decisions/` | `find /Users/alexkamysz/AI/arkai/docs/ai/decisions -maxdepth 1 -type f | sort` |
| Runtime validator imports schemas from `contracts/` via `CONTRACTS_DIR`. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/services/voice/validator.py` | `rg -n "CONTRACTS_DIR|contracts/" /Users/alexkamysz/AI/arkai/services/voice/validator.py` |
| Top-level architecture surfaces include `src`, `services`, `contracts`, `docs`, `docs/ai`, `patterns`, `pipelines`, `tests`, and `ai_docs`. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai` | `find /Users/alexkamysz/AI/arkai -maxdepth 2 -type d | sort` |
| Governance entry points (`docs/ai/README`, `SHARED-STATE`, `CROSS-REPO-AUTHORITY`, verify script) are explicitly referenced by startup docs. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/AGENTS.md`; `/Users/alexkamysz/AI/arkai/docs/CONTEXT.md` | `rg -n "docs/ai/README.md|docs/ai/SHARED-STATE.md|CROSS-REPO-AUTHORITY|verify_ai_docs.sh" /Users/alexkamysz/AI/arkai/AGENTS.md /Users/alexkamysz/AI/arkai/docs/CONTEXT.md` |
| Recent commit samples support active/reference-only classification context for core docs and contracts docs. | VERIFIED-IN-REPO | `/Users/alexkamysz/AI/arkai/README.md`; `/Users/alexkamysz/AI/arkai/docs/ROADMAP.md`; `/Users/alexkamysz/AI/arkai/docs/ARCHITECTURE.md`; `/Users/alexkamysz/AI/arkai/contracts/queue_task_adr.md`; `/Users/alexkamysz/AI/arkai/contracts/README.md`; `/Users/alexkamysz/AI/arkai/services/inbox/README.md`; `/Users/alexkamysz/AI/arkai/ai_docs/architecture/overview.md` | `for p in README.md docs/ROADMAP.md docs/ARCHITECTURE.md contracts/queue_task_adr.md contracts/README.md services/inbox/README.md ai_docs/architecture/overview.md; do git -C /Users/alexkamysz/AI/arkai log -1 --format='%cs %h' -- "$p"; done` |
| No direct `fabric-arkai` references were found in `contracts/` and `services/` trees in this pass. | INFERENCE | `/Users/alexkamysz/AI/arkai/contracts`; `/Users/alexkamysz/AI/arkai/services` | `for d in /Users/alexkamysz/AI/arkai/contracts /Users/alexkamysz/AI/arkai/services; do rg -n "fabric-arkai" "$d" || echo NO_MATCH; done` |
| `ai_docs/architecture/overview.md` remains canonical-status ambiguous vs docs architecture files. | INFERENCE | `/Users/alexkamysz/AI/arkai/ai_docs/architecture/overview.md`; `/Users/alexkamysz/AI/arkai/docs/AI_OS_ARCHITECTURE.md`; `/Users/alexkamysz/AI/arkai/docs/ARCHITECTURE.md`; `/Users/alexkamysz/AI/arkai/README.md` | `rg -n "AI_OS_ARCHITECTURE|ai_docs/architecture/overview.md|ARCHITECTURE.md" /Users/alexkamysz/AI/arkai/README.md /Users/alexkamysz/AI/arkai/docs/ARCHITECTURE.md` |
| Authority intro scope and matrix rows are inconsistent about whether `arkai-gmail` is in scope. | INFERENCE | `/Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md` | `nl -ba /Users/alexkamysz/AI/arkai/docs/ai/authority/CROSS-REPO-AUTHORITY.md | sed -n '9,21p'` |

## Sync-Back Summary (For Coordination Workspace)
- what changed: Drafted Wave 2 synthesis handoff at `/Users/alexkamysz/AI/arkai/docs/ai/handoffs/2026-03-14-wave2-architecture-map-operator-handoff.md` with complete deliverables `1-9`, explicit `Observed Facts` vs `Inferred` vs `Unknown` separation, and per-section confidence+rationale.
- what was verified: Required section coverage, section-structure counts, and evidence-manifest schema/taxonomy were re-checked; labels remain only `VERIFIED-IN-REPO`, `EXTERNAL-POINTER`, `INFERENCE`.
- residual risks: Cross-repo contract metadata drift remains (`owner_repo`/`semver`/`adr_pointer` missing in checked contracts), ADR-0002 reference alignment remains unresolved, and external repos remain pointer-scoped in this pass.
- next-step options: 1) run a final holdout recheck of this updated summary block; 2) resolve metadata/ADR alignment in canonical contract governance lane; 3) if needed, perform separate external-repo verification pass for `openclaw-local`, `fabric-arkai`, and `arkai-gmail`.

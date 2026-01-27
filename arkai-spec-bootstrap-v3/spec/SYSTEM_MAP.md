# ARKAI Canonical System Map (Agent + Human Readable)

> **Purpose**: This document is the *single source of navigational truth* for the ARKAI ecosystem. All modules, specs, schemas, and backlog items should reference anchors in this file.
>
> **Design Goals**:
> - Optimized for **agentic use** (clear contracts, schemas, invariants)
> - Optimized for **human scanning** (short sections, cross-links, checklists)
> - Built for **parallel development** (multi-repo, worktrees, Safe Cards)

---

## 0. System Vision

**ARKAI** is a **local-first, event-sourced, agent-orchestrated Human Operating System**.

It provides:
- **Frictionless capture** (voice, photo, email, UI journeys)
- **Secure transformation** (untrusted → structured facts)
- **Intelligent assistance** (nudges, goals, mnemonics, workouts, triage)
- **Hard safety boundaries** (reader / critic / executor separation)
- **Full replayability** (flight recorder + annotations)

**Non-Goals**
- No monorepo for everything
- No single “god agent”
- No raw untrusted text driving tools

---

## 1. Architectural Laws (Non-Negotiable)

### 1.1 Separation of Powers

No component may combine:
- **Sense** (read untrusted input)
- **Think** (reason, plan, propose)
- **Act** (execute tools)

These must live in separate modules.

### 1.2 Event-First Design
All state = derived. All changes = events. Nothing mutates history.

### 1.3 Untrusted Content Quarantine
Voice, email, web, screenshots, transcripts are **never promoted**. Only **structured fact packets** may flow downstream.

### 1.4 Blast Radius Limits
- Rate limits
- Bulk-action approval
- Cross-domain confirmation

### 1.5 Cockpit UX Law
**Never browse. Always operate.**
Every interface implements: **SCAN → CYCLE → ZOOM → ACT → REMEMBER**

---

## 2. Canonical Repo Topology (Multi-Package Model)

### 2.1 arkai-spine (existing)
**Role**: Orchestration, state, storage, execution engine

Responsibilities:
- Event store
- Pipeline execution
- Safety limits
- Evidence / spans

Must remain:
- LLM-light
- Deterministic
- Policy-driven

### 2.2 arkai-spec-kernel (THIS `spec/`)
**Role**: Canonical contracts + schemas + scorecards

Houses:
- Event schemas
- Module interfaces
- Threat model
- UX laws
- Safe Card template

All other repos **pin this as a dependency**.

### 2.3 Module Packages (Parallel Dev)
Each module is its own repo or worktree:
- capsule-ingest
- sanitizer-facts
- critic-gate
- executor
- mnemo-layer
- nudge-engine
- goals-engine
- workout-mode
- user-journey-capture
- flight-recorder-ui
- gmail-connector (gated)

---

## 3. Data & Control Flow (End-to-End)

```
[Capture]
  ↓
[Capsule Created Event]
  ↓
[Sanitizer → Fact Packet]
  ↓
[Reader Agent → Proposal]
  ↓
[Critic Gate → Approve / Deny / Ask]
  ↓
[Executor → Whitelisted Tools]
  ↓
[Flight Recorder → Replay + Metrics]
```

---

## 4. Event System & Flight Recorder

### 4.1 Base Event Envelope (TARGET)

Target envelope fields (aspirational; see §4.4 for MVP reality):

```yaml
event_id
trace_id
span_id
parent_span_id
timestamp
actor
schema_version
payload_ref
```

### 4.2 Required Event Types (TARGET)

- capture.created
- fact.packet.created
- proposal.created
- proposal.approved / denied
- action.executed
- nudge.proposed / shown / responded
- goal.updated
- annotation.voice
- error.raised

### 4.3 Replay Invariants
- Same inputs + config → explainable outputs
- Full causal chain visible

### 4.4 Current vs Target (Alignment)

**Current Rust Event struct** uses:
- `id (uuid)`, `timestamp`, `run_id`, `step_id`, `event_type`, `idempotency_key`
- `payload_summary` (inline string), `status`, `duration_ms`, `error`

**Storage**: JSONL (`~/.arkai/runs/{run_id}/events.jsonl`) via pure serde JSON.  
There is **no SQLite** in the current spine.

**Target** adds:
- `trace_id/span_id/parent_span_id`
- `actor`
- `schema_version`
- `payload_ref` (content-addressed), with inline allowed for MVP

See: `spec/ALIGNMENT.md` for the authoritative mapping + migration plan.

---

## 5. Safety & Immune Model

### 5.1 Compartments
1. Quarantine
2. Sanitizer
3. Reader
4. Critic
5. Executor

### 5.2 Innate Defenses
- Instruction pattern detection
- Domain deny-lists
- Rate caps

### 5.3 Adaptive Defenses
- Pattern memory
- Correction learning

---

## 6. Capture Layer (Capsules)

### 6.1 Capsule Schema (conceptual)
```yaml
id
timestamp
device
raw_refs:
  - audio
  - image
  - text
tags_spoken
trust_level: untrusted
signature
```

### 6.2 Capture Primitives
- Voice (hardware button)
- Photo (+ voice tag)
- Quick domain tags

---

## 7. Fact Packet Layer

### 7.1 Deterministic Only
No LLM here.

### 7.2 Output Schema (conceptual)
```yaml
capsule_id
facts
signals
excerpt
confidence
```

---

## 8. Intelligence Layer

### 8.1 Reader Agents
- Summarize
- Classify
- Propose actions

### 8.2 Critic Gate
- Risk scoring
- Policy enforcement
- Approval workflows

### 8.3 Executor
- Dumb tool adapter
- Whitelist only

---

## 9. Cockpit UX Systems

### 9.1 Nudge Engine
- Opportunity states
- Energy dial
- Bundle system
- Nudge budgets

### 9.2 Goals & Trajectories
- Long-term goals
- Momentum tracking
- Seed steps
- Clarifying questions

### 9.3 Mnemo Layer
- Anchors
- Cues
- Sequences
- Domain palette

### 9.4 Workout Mode
- Set grammar
- Cycling loop
- Session schema

---

## 10. Procedural Knowledge Capture

### 10.1 User Journey Pipeline
- Screen record
- Narration
- Transcription
- Step extraction
- Workflow packaging

Outputs:
- Markdown
- YAML
- Graph edges

---

## 11. Parallel Agent Workflow

### 11.1 Safe Cards
Required for every change:
```yaml
id
summary
touches
risk_level
threats
guards
tests
rollback
```

### 11.2 Integration Loop
- Schema validation
- Replay validation
- Scorecard pass

---

## 12. Scorecards

### Security
- Separation intact
- No raw → tool paths
- Blast radius enforced

### UX
- One-gesture rule
- No browse flows

### Integration
- Schema compile
- Replay works

---

## 13. Sub-Agent Work Orders (Canonical)
A. Spine Audit  
B. Spec Kernel Bootstrap  
C. Capsule Capture  
D. Sanitizer  
E. Critic Gate  
F. Mnemo Layer  
G. Workout Mode  
H. Nudge Engine  
I. Replay UI  
J. Gmail Connector

---

## 14. Backlog Mapping

All tasks must reference: `spec/SYSTEM_MAP.md#section`

Example:
> E-NUDGE-01 → Section 9.1

---

## 15. Governance
- Weekly spec prune
- Schema versioning
- Module deprecation policy

---

## Status
This file is **canonical**. All other documentation is subordinate.

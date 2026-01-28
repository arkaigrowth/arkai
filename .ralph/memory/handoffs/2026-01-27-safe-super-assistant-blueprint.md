# Safe Super-Assistant Blueprint Handoff

> **Created**: 2026-01-27
> **Session**: Master Session 4 (compacted from Session 3)
> **Status**: BLUEPRINT READY - Implementation next session

---

## Context

Chad provided comprehensive blueprint for "Moltbot + Arkai Safe Super-Assistant".
Master session validated that **arkai-gmail already implements the core pattern**.
This handoff captures the blueprint for next session implementation.

---

## Key Validation

**The Reader/Critic/Actor pattern works.** We proved it with Gmail:

| Chad's Primitive | arkai-gmail Implementation |
|------------------|---------------------------|
| Reader (LLM, no tools) | `reader.py` (Layer C) |
| Critic (deterministic) | `critic.py` (Layer D) - typed rules, SQLite rate limits |
| Actor (allowlisted) | `actor.py` (Layer E) - dry_run default, safety checks |
| Audit (JSONL) | `~/.arkai-gmail/audit.jsonl` |
| Contracts | `contracts/email_triage.schema.json` |

**Implication:** Generalize this pattern into the arkai spine.

---

## Architecture Summary

```
LAYER 1: UX
  Clawdbot → Telegram/Control UI
  (notifications, commands, approvals)

LAYER 2: ORCHESTRATION (Arkai Spine)
  ~/.arkai/
  ├── events.jsonl     (event-sourced state)
  ├── artifacts/       (all outputs by feature)
  ├── contracts/       (JSON schemas)
  └── config.yaml      (trust zones, policies)

  Primitives:
  - GenericReader (LLM-only, strict JSON output)
  - GenericCritic (schema validation + policy + rate limits)
  - GenericActor (allowlisted actions only)
  - AuditLog (append-only JSONL)

LAYER 3: EXECUTION (Nodes)
  VPS Node: research, no exec
  Mac Node: Apple apps, allowlisted skills

LAYER 4: FEATURES (Modules)
  Each module = Reader + Critic + Actor + Artifacts + Audit
  - Gmail (DONE)
  - Meetings (Phase 2)
  - Groceries (Phase 2)
  - YouTube (Phase 3)
  - Shopping (Phase 3)
```

---

## Trust Zones (Chad's Model)

| Zone | Risk | Allowed | Forbidden |
|------|------|---------|-----------|
| Internet | HIGH | Web search, browse, APIs | Filesystem, exec |
| Workspace | MEDIUM | Read/write artifacts | Arbitrary exec, full disk |
| Execution | HIGH IMPACT | Allowlisted actions | Arbitrary commands |

**Rule:** Never mix zones. Reader in Internet zone, Actor in Execution zone, Critic bridges them.

---

## Agent Profiles (for Clawdbot)

```yaml
# research-agent (Internet Zone)
tools: [web_search, web_browse]
forbidden: [filesystem, exec, nodes]
outputs: artifacts/research/

# ops-agent (Workspace Zone)
tools: [read_artifacts, write_artifacts, node_dispatch]
forbidden: [arbitrary_exec, full_disk]
outputs: artifacts/ops/

# planner-agent (Low-risk)
tools: []  # No tools, reasoning only
outputs: artifacts/plans/
```

---

## Tickets to Create (Next Session)

### Phase 1: Primitives
1. `PRIMITIVES_GENERIC_READER_CRITIC_ACTOR.yaml`
   - Extract pattern from arkai-gmail
   - Make it reusable for any feature

2. `PRIMITIVES_ARTIFACTS_AND_SCHEMAS.yaml`
   - Directory structure: `~/.arkai/artifacts/{feature}/`
   - Universal schemas in `contracts/`

3. `PRIMITIVES_AUDIT_LOGGING.yaml`
   - Generalize from gmail audit.jsonl
   - Universal event format

4. `PRIMITIVES_AGENT_PROFILES.yaml`
   - Configure Clawdbot with trust zones
   - research-agent, ops-agent, planner-agent

### Phase 2: Flagship Workflows
5. `WORKFLOW_MEETING_PREP_MVP.yaml`
   - Calendar read (Mac node)
   - Context fetch (allowlisted folders)
   - Brief generation (planner-agent)

6. `WORKFLOW_GROCERIES_RECIPES_MVP.yaml`
   - Input: voice/text
   - Research: recipes from web
   - Output: Reminders list on Mac

### Phase 3: Expand
7. `PIPELINE_YOUTUBE_INGEST_MVP.yaml`
8. `WORKFLOW_SHOPPING_RESEARCH.yaml`

---

## Blockers / Prerequisites

1. **gog not installed** - Need for Gmail Pub/Sub integration
   ```bash
   # Install gog (Google OAuth CLI)
   # Check: https://github.com/clawdbot/gog
   ```

2. **Clawdbot agent profiles** - Need to configure in clawdbot.json

3. **Mac node setup** - Moltbot nodes for Apple apps

4. **Folder allowlisting** - Map which folders each agent can access

---

## Commands Cheat Sheet

```bash
# Gmail triage (already working)
cd ~/AI/arkai-gmail
python3.11 -m arkai_gmail.cli triage --execute --limit 10

# Check Clawdbot Control UI
open https://arkai-clawdbot.taila30487.ts.net/

# VPS gateway status
ssh clawdbot-vps "screen -ls && tail -20 ~/gateway.log"
```

---

## Key Decisions Made

1. **arkai = spine** (orchestration, events, artifacts)
2. **Clawdbot = UX layer** (Telegram, Control UI)
3. **Moltbot = node orchestration** (Mac, future devices)
4. **Reader/Critic/Actor = universal pattern** (proven with Gmail)
5. **Artifacts-first** (if it isn't on disk, it doesn't exist)
6. **Progressive hardening** (start Always Ask, graduate patterns)

---

## Reference Documents

- Chad's full blueprint: In conversation history
- Moltbot docs: https://docs.molt.bot/#start-here
- Clawdbot docs: https://docs.clawd.bot/
- arkai-gmail: `~/AI/arkai-gmail/`
- Existing contracts: `~/AI/arkai/contracts/`

---

## Next Session Plan

**Option A: Single focused session**
- Implement Phase 1 primitives
- ~2-3 hours

**Option B: Parallel sub-sessions**
- primitives-builder: Extract generic Reader/Critic/Actor
- schemas-builder: Create artifact schemas
- meeting-prep-builder: Build MVP workflow
- Master coordinates

**Recommendation:** Option A first (primitives), then Option B for features.

---

## TL;DR for Next Session

> "We proved Reader/Critic/Actor works with Gmail. Now generalize it into the arkai spine. Then build Meeting Prep and Groceries as the first two workflows using those primitives. No risky automations until stable."

---

*Handoff created 2026-01-27 by Master Session 4*

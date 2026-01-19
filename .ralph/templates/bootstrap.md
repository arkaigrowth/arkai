# RALPH Session Bootstrap

> This file is injected at the start of each new Claude Code session.
> It establishes the retrieval protocol and output contract.

---

## Context

**You have zero memory of past sessions.** Do not assume anything about prior work.

**Session ID**: {{SESSION_ID}}
**Started**: {{TIMESTAMP}}
**Mission**: {{MISSION}}

---

## Retrieval Protocol

Before taking any action, you MUST:

1. **Read constraints**: `.ralph/memory/constraints.md`
2. **Read rolling summary**: `.ralph/memory/rolling_summary.md`
3. **Check recent decisions**: `.ralph/memory/decisions.log` (last 20 lines)

When uncertain about past context:
- Search previous run artifacts with: `rg "keyword" .ralph/runs/`
- Check specific session: `.ralph/runs/{{PREVIOUS_SESSION}}/distill/`

---

## Output Contract

At session end, you MUST produce these artifacts in `.ralph/runs/{{SESSION_ID}}/distill/`:

| File | Purpose | Format |
|------|---------|--------|
| `summary.md` | What happened this session | Narrative, 200-500 words |
| `decisions.md` | Explicit choices made + rationale | Bullet list with reasoning |
| `open_questions.md` | Unresolved items, blockers | Prioritized list |
| `next_prompt.md` | Bootstrap for next session | Ready-to-paste prompt |
| `retrieval_index.json` | Where to find things | JSON: `{topic: [file_paths]}` |

---

## Constraints

{{CONSTRAINTS}}

---

## Active Context

### Rolling Summary
{{ROLLING_SUMMARY}}

### Recent Decisions
{{RECENT_DECISIONS}}

---

## Begin Session

Mission: {{MISSION}}

You may now proceed. Remember: read before acting, log decisions, produce artifacts at end.

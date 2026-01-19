# Note-Taking System Scoring Rubric

> **Purpose**: Objectively evaluate note-taking systems against measurable criteria.
> **Last Updated**: 2026-01-18
> **Status**: Draft - Pending multi-model consensus validation

---

## Scoring Methodology

Each criterion is scored 0-10:
- **0-2**: Severely lacking, major friction
- **3-4**: Below average, noticeable issues
- **5-6**: Adequate, works but not great
- **7-8**: Good, minor issues
- **9-10**: Excellent, minimal friction

**Final Score** = Weighted average of all criteria

---

## Criteria Definitions

### 1. Capture Friction (Weight: 20%)

> Can you land in today's note instantly?

| Score | Definition |
|-------|------------|
| 10 | One keystroke/tap from anywhere to capture |
| 8 | 2-3 actions, < 5 seconds |
| 6 | Requires app switching, 5-10 seconds |
| 4 | Multiple steps, 10-30 seconds |
| 2 | Complex workflow, > 30 seconds |
| 0 | Requires manual file navigation |

**Test**: Time yourself capturing a thought from:
- Desktop (working on something else)
- Mobile (screen off)
- Browser (reading article)

---

### 2. Retrieval Power (Weight: 20%)

> Can you find what you need via keyword + semantic search?

| Score | Definition |
|-------|------------|
| 10 | Instant full-text + semantic + graph traversal |
| 8 | Fast full-text + tags + basic semantic |
| 6 | Full-text search with good ranking |
| 4 | Basic search, often misses results |
| 2 | Manual browsing required for most lookups |
| 0 | No search functionality |

**Test**:
- Find a note from 6 months ago by vague keyword
- Find notes related to a concept (not exact match)
- Find notes linking to/from a specific note

---

### 3. Surfacing (Weight: 20%)

> Do tasks, snoozes, and time-bound items resurface automatically?

| Score | Definition |
|-------|------------|
| 10 | Smart surfacing: due items, snoozed, stale notes, low-energy queue |
| 8 | Due dates + reminders + periodic review prompts |
| 6 | Due dates work, manual review of rest |
| 4 | Basic reminders, no smart resurfacing |
| 2 | Must manually check for due items |
| 0 | No surfacing mechanism |

**Test**:
- Create a task due in 3 days - does it appear on day 3?
- Snooze a note for 1 week - does it resurface?
- Check for stale notes (untouched > 30 days)

---

### 4. Modularity & Permissions (Weight: 15%)

> Can AI tools touch some folders but not others?

| Score | Definition |
|-------|------------|
| 10 | Granular permissions: AI sandbox, private vault, shared, archive |
| 8 | Clear separation with enforced boundaries |
| 6 | Convention-based separation (not enforced) |
| 4 | All-or-nothing access |
| 2 | No meaningful separation possible |
| 0 | Single monolithic store |

**Test**:
- Can you mark folders as "AI can read" vs "AI cannot touch"?
- Can you share some notes without exposing others?
- Can you have work/personal separation?

---

### 5. Maintainability (Weight: 15%)

> Do upgrades break your workflow?

| Score | Definition |
|-------|------------|
| 10 | Plain files, no lock-in, decade-proof format |
| 8 | Standard format + stable plugin ecosystem |
| 6 | Proprietary but exportable, reasonable migration path |
| 4 | Vendor lock-in with export, plugins break often |
| 2 | Significant lock-in, painful migrations |
| 0 | No export, complete vendor dependency |

**Test**:
- Export all notes to plain markdown - how much is lost?
- After major version upgrade, what breaks?
- If company shuts down, can you continue?

---

### 6. Mobile Capture (Weight: 10%)

> Can you go voice → text → filed seamlessly?

| Score | Definition |
|-------|------------|
| 10 | Native voice capture → auto-transcribe → auto-file → sync |
| 8 | Voice capture works, auto-transcribe, manual file |
| 6 | Third-party voice capture, copy-paste to app |
| 4 | Text-only mobile, no voice |
| 2 | Mobile app exists but clunky |
| 0 | No mobile support |

**Test**:
- Capture a voice memo while walking
- How long until it appears in your main system?
- Is the transcription accurate?

---

## Scoring Template

| System | Capture | Retrieval | Surfacing | Modularity | Maintain | Mobile | **Total** |
|--------|---------|-----------|-----------|------------|----------|--------|-----------|
| Obsidian | ? | ? | ? | ? | ? | ? | ? |
| Notion | ? | ? | ? | ? | ? | ? | ? |
| Apple Notes | ? | ? | ? | ? | ? | ? | ? |
| Logseq | ? | ? | ? | ? | ? | ? | ? |
| Roam | ? | ? | ? | ? | ? | ? | ? |
| Custom Sidecar | ? | ? | ? | ? | ? | ? | ? |

---

## Weighted Score Calculation

```
Total = (Capture × 0.20) + (Retrieval × 0.20) + (Surfacing × 0.20) +
        (Modularity × 0.15) + (Maintainability × 0.15) + (Mobile × 0.10)
```

### Weight Rationale

- **Capture/Retrieval/Surfacing (60%)**: Core PKM loop - input, find, resurface
- **Modularity/Maintainability (30%)**: Long-term viability and AI integration
- **Mobile (10%)**: Important but secondary to core workflow

---

## Decision Framework

| Score Range | Recommendation |
|-------------|----------------|
| 8.0+ | Excellent choice, minor optimizations only |
| 6.5-7.9 | Good foundation, address weak areas |
| 5.0-6.4 | Workable but friction will accumulate |
| < 5.0 | Consider switching or heavy customization |

---

## Notes

- Scores should be **evidence-based** (timed tests, actual exports)
- Revisit scores after major updates or workflow changes
- The "Custom Sidecar" option is the AI triage sidecar being designed

---

*Pending: Multi-model consensus on weights from steelman analysis*

# Arkai Obsidian Integration Roadmap
## Backlog & Future Work

**Created:** 2026-01-17
**Status:** Planning (NOT for implementation now)
**Current Focus:** Vault organization + daily notes + contracts + minimal plugins

---

## Scope Boundaries

### ✅ IN SCOPE (Current Sprint)
- Obsidian vault reorganization (Phases 0-8)
- Daily notes workflow
- Properties/tags contracts
- Minimal plugin set (7 plugins)
- 8 seed MOCs
- Light-touch linking habit
- .arkai/ integration layer (basic)

### 🚫 OUT OF SCOPE (This Document = Backlog)
- Voice memo ingestion
- Task extraction pipelines
- Todoist integration
- Iron Ledger integration
- Advanced embeddings/graph

---

## Roadmap Overview

```
┌─────────────────────────────────────────────────────────────────┐
│  PHASE 1: VAULT FOUNDATION (Current)                            │
│  Obsidian reorg → Daily notes → Contracts → Minimal plugins     │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│  PHASE 2: CAPTURE BUS                                           │
│  Voice memos → Task extraction → 2-phase commit                 │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│  PHASE 3: INTEGRATIONS                                          │
│  Todoist (commit-only) → Iron Ledger → Calendar sync            │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│  PHASE 4: INTELLIGENCE LAYER                                    │
│  Embeddings → Graph indexing → Semantic search → AI review      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Backlog Items

### 🎙️ B1: Voice Memo Ingestion Pipeline

**Priority:** High (frequent use case)
**Complexity:** Medium
**Dependencies:** Vault foundation complete

**Description:**
Capture voice memos from iPhone → transcribe → append to daily note or dedicated inbox.

**Proposed Flow:**
```
Voice Memo (iPhone)
       │
       ▼
┌─────────────────┐
│  Sync to Mac    │  (iCloud or manual)
│  ~/Voice Memos/ │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Transcription  │  (Whisper local or API)
│  → transcript   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Daily Append   │  (Templater or script)
│  ## Voice Memo  │
│  - timestamp    │
│  - transcript   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Optional:      │
│  Task extract   │  (→ B2)
│  Topic tag      │
└─────────────────┘
```

**Contract (voice_memo_contract.md):**
```yaml
input:
  format: m4a | mp3 | wav
  location: ~/Library/Group Containers/.../Voice Memos/
  naming: Recording NNNN.m4a

output:
  target: Daily Note or 00-Inbox/voice-memo-{date}.md
  format: |
    ## 🎙️ Voice Memo - {time}
    **Duration:** {duration}
    **Transcript:**
    {transcript_text}

    **Extracted:**
    - Tasks: {tasks_if_any}
    - Topics: {topics_if_any}
```

**Open Questions:**
- Local Whisper vs API? (Privacy vs quality)
- Append to daily note vs separate file?
- Real-time processing vs batch?

---

### ✅ B2: Task Candidate Extraction + 2-Phase Commit

**Priority:** High (ADHD executive function support)
**Complexity:** High
**Dependencies:** B1 (voice memos), vault contracts

**Description:**
Extract potential tasks from any input (voice memos, notes, clipboard) and present for human approval before committing to task system.

**Why 2-Phase Commit:**
- LLM extraction is imperfect
- ADHD brains need agency over commitments
- Prevents "task explosion" where everything becomes a to-do

**Proposed Flow:**
```
Input (voice memo, note, clipboard)
       │
       ▼
┌─────────────────┐
│  Task Extract   │  (LLM: Haiku)
│  candidate list │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────┐
│  PHASE 1: STAGING              │
│  tasks_pending.json            │
│  [                             │
│    {                           │
│      "text": "Follow up X",    │
│      "source": "voice memo",   │
│      "confidence": 0.85,       │
│      "suggested_project": "..."│
│    }                           │
│  ]                             │
└────────┬────────────────────────┘
         │
         ▼
┌─────────────────────────────────┐
│  HUMAN REVIEW                  │
│  (CLI, Obsidian, or push)      │
│  ✓ Approve  ✗ Reject  ✎ Edit   │
└────────┬────────────────────────┘
         │
         ▼
┌─────────────────────────────────┐
│  PHASE 2: COMMIT               │
│  → Todoist (if approved)       │
│  → Daily Note task section     │
│  → Clear from staging          │
└─────────────────────────────────┘
```

**Contract (task_extraction_contract.md):**
```yaml
extraction:
  triggers:
    - voice memo transcript
    - note with #review-tasks tag
    - clipboard capture command

  patterns:
    - "I need to..."
    - "Don't forget to..."
    - "TODO:"
    - "Remind me to..."
    - Action verbs at start of sentence

staging:
  location: .arkai/tasks_pending.json
  max_age: 7 days (auto-expire if not reviewed)

commit_targets:
  - todoist (if integration enabled)
  - daily_note (always)
```

---

### 📋 B3: Todoist Integration (Commit-Only)

**Priority:** Medium
**Complexity:** Low
**Dependencies:** B2 (task extraction)

**Description:**
One-way sync: Approved tasks → Todoist. No Todoist → Obsidian sync (avoid complexity).

**Why Commit-Only:**
- Todoist is the "action" system
- Obsidian is the "thinking" system
- Two-way sync is fragile and creates confusion
- Keep it simple: ideas flow out to tasks, not back

**Contract (todoist_contract.md):**
```yaml
direction: obsidian → todoist (one-way)

commit_trigger:
  - Task approved in 2-phase commit
  - Note tagged #send-to-todoist

mapping:
  task_text: → Todoist task content
  project_hint: → Todoist project (if matches)
  due_date: → Todoist due date (if extracted)
  priority: → Todoist priority (default: 4)

NOT supported:
  - Todoist → Obsidian sync
  - Todoist completion → Note update
  - Two-way project sync
```

**API:**
```python
# Simple commit function
def commit_task_to_todoist(task: dict) -> bool:
    """
    Sends approved task to Todoist.
    Returns True if successful.
    """
    # Use Todoist REST API
    # POST https://api.todoist.com/rest/v2/tasks
```

---

### 🔄 B4: Iron Ledger Review Loop + Events

**Priority:** Medium
**Complexity:** Medium
**Dependencies:** Vault foundation

**Description:**
Integrate Iron Ledger (habit/accountability system) with Obsidian for:
- Daily review prompts
- Event logging
- Progress visualization

**Proposed Integration Points:**
1. **Morning:** Iron Ledger surfaces daily intentions → append to Daily Note
2. **Evening:** Daily Note accomplishments → feed back to Iron Ledger
3. **Weekly:** Iron Ledger metrics → Weekly Review note

**Contract (iron_ledger_contract.md):**
```yaml
sync_points:
  morning:
    source: iron_ledger.daily_intentions
    target: daily_note.morning_section

  evening:
    source: daily_note.accomplishments
    target: iron_ledger.completed_events

  weekly:
    source: iron_ledger.weekly_metrics
    target: periodic/week/{week}.md

event_types:
  - habit_completed
  - goal_progress
  - reflection_logged
  - streak_milestone
```

**Open Questions:**
- What is Iron Ledger's API/data format?
- Is this a Claude-based system or standalone?
- Where does Iron Ledger data live?

---

### 🧠 B5: Embeddings + Graph Indexing

**Priority:** Low (derived value, not core)
**Complexity:** High
**Dependencies:** Vault foundation, .arkai/ layer

**Description:**
Generate embeddings for semantic search and build relationship graph for AI-assisted navigation.

**Why Deferred:**
- Vault must be organized first
- High compute/cost
- Value emerges only with good base data

**Proposed Architecture:**
```
.arkai/
├── embeddings/
│   ├── embeddings.parquet    # Vector store
│   └── embedding_log.json    # Processing history
├── graph/
│   ├── nodes.json            # Note entities
│   ├── edges.json            # Relationships
│   └── clusters.json         # Topic clusters
└── search/
    └── index.json            # Search metadata
```

**Contract (embeddings_contract.md):**
```yaml
embedding:
  model: text-embedding-3-small  # or local
  dimensions: 1536
  batch_size: 100

  include:
    - All notes except .aiexclude
    - Frontmatter: title, topics, summary
    - Content: first 8000 chars

  exclude:
    - Attachments (non-text)
    - Daily notes older than 90 days (optional)

graph:
  node_types:
    - note
    - topic
    - entity (person, project, concept)

  edge_types:
    - links_to (explicit wikilink)
    - mentions (entity extraction)
    - similar_to (embedding cosine > 0.8)

  update_policy:
    - Full rebuild: monthly
    - Incremental: on note change
```

**Use Cases (Future):**
- "Find notes similar to this one"
- "What have I written about X?"
- "Show me the knowledge graph around this topic"
- AI-assisted weekly review ("You haven't touched these notes in 3 months...")

---

## Priority Matrix

| ID | Item | Priority | Complexity | Dependencies |
|----|------|----------|------------|--------------|
| -- | Vault Foundation | **NOW** | Medium | None |
| B1 | Voice Memo Pipeline | High | Medium | Vault done |
| B2 | Task Extraction | High | High | B1 |
| B3 | Todoist Integration | Medium | Low | B2 |
| B4 | Iron Ledger | Medium | Medium | Vault done |
| B5 | Embeddings/Graph | Low | High | Vault done |

---

## Recommended Sequence

```
NOW:     Vault Foundation (Phases 0-8)
         ↓
NEXT:    B1 (Voice Memos) → B2 (Task Extract)
         ↓
THEN:    B3 (Todoist) + B4 (Iron Ledger) [parallel]
         ↓
LATER:   B5 (Embeddings/Graph)
```

---

## Notes

- **Do not start any backlog item until vault foundation is complete**
- Each item should have its own contract before implementation
- Human review checkpoints for each integration
- Prefer simple, one-way flows over complex bidirectional sync
- All data stays local unless explicitly sent to trusted APIs

---

*This document is a living backlog. Update as priorities shift.*

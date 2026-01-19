# Obsidian Vault Reorganization Strategy v2.0
## Final Steelmanned Version (Claude + Chad Synthesis)

**Date:** 2026-01-17
**Vault:** `/Users/alexkamysz/AI/arkai/vault-sandbox/`
**Sources:**
- Chad (GPT 5.2) - Original plan + corrections
- Claude Opus 4.5 - Enhancements + synthesis
- ADHD Note System Research - Principles + best practices

---

## Executive Summary

This is the **final, corrected** strategy incorporating:
1. ✅ Chad's pricing corrections ($1/$5 for Haiku, not $0.25/$1.25)
2. ✅ Chad's date correction (2026-01-17)
3. ✅ Chad's "stop chasing frontmatter everywhere" principle
4. ✅ Chad's Obsidian-executed moves for link safety
5. ✅ Bases (core plugin) as Make.md replacement
6. ✅ ADHD research integration ("findability over perfection")
7. ✅ Stop conditions and contracts
8. ✅ Claude Cowork integration points

**Philosophy:** *"A messy collection of searchable notes beats a perfect system with nothing in it."*

---

## Critical Corrections Applied

| Issue | Original | Corrected |
|-------|----------|-----------|
| **Date** | 2025-01-17 | **2026-01-17** |
| **Haiku 4.5 pricing** | $0.25/$1.25 per MTok | **$1/$5 per MTok** |
| **Cost estimate** | $2-5 | **$15-20** |
| **Success criteria** | 80% notes with frontmatter | **100% new + 100% high-value** |
| **Move scope** | 847 files (aggressive) | **200-300 files (conservative)** |
| **Move method** | Script-based | **Obsidian-executed** |

---

## Core Principles (ADHD-Aligned)

From the ADHD Note System Research:

1. **"Limit the places a thing can be"** - PARA with 4-5 folders max
2. **"MOCs provide on-demand structure"** - Create maps when overwhelmed, not upfront
3. **"A messy collection of searchable notes beats a perfect system"** - Findability > perfection
4. **"Daily Notes as low-pressure inbox"** - Capture without organizing
5. **"Keep plugin list lean"** - 5-7 plugins, not 37
6. **"Novelty can be harnessed"** - Some customization keeps system engaging
7. **"Executive function scaffolds"** - Templates, breadcrumbs, weekly reviews

---

## Chad's Critical Question: Answer

**Q: "Do you want PARA folders to be the 'real' structure, or do you want MOCs to be the primary navigation layer with minimal moving?"**

**A: Hybrid (Option C) - PARA-lite folders + MOC navigation**

**Folder Structure (minimal):**
```
vault/
├── 00-Inbox/              # Quick captures, unsorted
├── 10-Active/             # Current projects + hot areas
├── 20-Reference/          # All resources, knowledge
├── 90-Archive/            # Completed, historical
├── Attachments/           # All media consolidated
├── System/                # Templates, contracts, config
├── Daily Notes/           # KEEP AS-IS
├── Periodic/              # KEEP AS-IS
└── CATSY/                 # KEEP AS-IS (work)
```

**Within folders:** MOCs + Bases views for navigation (not deep subfolders)

**What gets moved:**
- ✅ Root orphans → appropriate folder
- ✅ Obvious misplacements
- ✅ .trash items → Archive or delete

**What stays put:**
- ❌ Already-organized folders (Periodic, Daily Notes, CATSY)
- ❌ Notes requiring judgment calls (handled via MOC/search)

---

## Revised Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    PHASE 0: SAFETY + BASELINE                   │
│  • Git init  • Create .aiexclude  • Capture baseline stats      │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│              PHASE 0.5: PLUGIN RATIONALIZATION                  │
│  • Audit 37→7 plugins  • Disable make-md  • Enable Bases        │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                PHASE 1: ENHANCED INVENTORY                      │
│  • Manifest  • Backlinks  • Broken links  • Content hashes      │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                    ═══════════════════════
                    ║  HUMAN CHECKPOINT 1  ║
                    ═══════════════════════
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│              PHASE 2: SMART SHARDING (BFS Algorithm)            │
│  • Chaos-first priority  • Backlink graph clustering            │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│              PHASE 3: LABELING (Claude Haiku 4.5)               │
│  • Type  • Topics  • Quality  • Risk flags  • High-value flag   │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                    ═══════════════════════
                    ║  STOP CONDITION CHECK ║
                    ║ manual_review > 10%?  ║
                    ═══════════════════════
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                    PHASE 4: AGGREGATION                         │
│  • Topic clusters  • Hub notes  • High-value list  • Health     │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                    ═══════════════════════
                    ║  HUMAN CHECKPOINT 2  ║
                    ═══════════════════════
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│             PHASE 5: PLANNING (Claude Sonnet 4.5)               │
│  • Taxonomy  • MOCs  • Contracts  • Conservative move plan      │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│            PHASE 6: APPLY (Obsidian-Executed)                   │
│  • Dry-run  • Obsidian moves with "Update links"  • Validate    │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                    ═══════════════════════
                    ║  STOP CONDITION CHECK ║
                    ║ broken links up? > 1% ║
                    ═══════════════════════
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                 PHASE 7: MAINTENANCE SYSTEM                     │
│  • Templates  • Weekly review habit  • Bases dashboards         │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│               PHASE 8: ARKAI INTEGRATION                        │
│  • .arkai/ config  • Index  • Embeddings  • Graph               │
└─────────────────────────────────────────────────────────────────┘
```

---

## Plugin Strategy (Final)

### Minimum Viable Plugin Set (7 total)

| Plugin | Type | Why Keep |
|--------|------|----------|
| **Bases** | Core | Database views, no maintenance risk, mobile-compatible |
| **Daily Notes** | Core | Journaling backbone |
| **Templates** | Core | Basic templating |
| **Templater** | Community | Advanced automation, actively maintained |
| **Calendar** | Community | Daily notes navigation |
| **Dataview** | Community | Complex queries (optional if Bases sufficient) |
| **Auto-note-mover** | Community | Rule-based filing for Phase 6 |

### Plugins to Remove (30)

**Redundant:**
- todoist-sync-plugin + ultimate-todoist-sync (pick one or neither)
- custom-sort + manual-sorting + obsidian-sortable (Bases handles this)
- neo4j-graph-view + juggl (core Graph view sufficient)

**Problematic:**
- **make-md** - Creates .space folders, owns data model, complex ❌

**Cosmetic (optional removal):**
- iconic, colorizelt, colored-text, highlightr-plugin
- obsidian-file-color

**Low-value:**
- Many others (metaedit, multi-properties, notebook-navigator, etc.)

### Make.md Migration Steps

```bash
# 1. In Obsidian: Settings → Community Plugins → Disable make-md
# 2. In terminal (sandbox only):
find vault-sandbox -type d -name ".space" -exec rm -rf {} +
find vault-sandbox -type d -name ".makemd" -exec rm -rf {} +
# 3. Verify vault still opens correctly
# 4. Enable Bases in Settings → Core Plugins
```

---

## Sharding Algorithm (Concrete)

Chad requested a concrete algorithm. Here it is:

```python
def create_shards(manifest, backlink_graph, limits):
    """
    BFS-based sharding that keeps linked notes together.

    limits = {
        'max_notes': 100,
        'max_bytes': 2_000_000,
        'max_tokens': 100_000
    }
    """
    shards = []
    processed = set()

    # Priority order for seeding shards
    priority_seeds = [
        get_root_orphans(manifest),           # Highest chaos
        get_imported_onenote(manifest),       # Bulk backlog
        get_trash_items(manifest),            # Cleanup needed
        get_unstructured_folders(manifest),   # Mixed content
        get_structured_folders(manifest),     # Already organized (last)
    ]

    for seed_group in priority_seeds:
        for seed_note in seed_group:
            if seed_note in processed:
                continue

            # Start new shard with seed
            shard = Shard()
            queue = [seed_note]

            # BFS expand until limits
            while queue and not shard.at_limits(limits):
                note = queue.pop(0)
                if note in processed:
                    continue

                shard.add(note)
                processed.add(note)

                # Add linked notes, sorted by mutual link count
                linked = backlink_graph.get_neighbors(note)
                linked.sort(key=lambda n: mutual_link_count(note, n), reverse=True)

                for linked_note in linked:
                    if linked_note not in processed:
                        queue.append(linked_note)

            shards.append(shard)

    return shards
```

---

## Success Criteria (Revised)

### ✅ DO:

| Criterion | Target |
|-----------|--------|
| New notes conform to templates | 100% |
| High-value notes enriched with frontmatter | 100% |
| Notes searchable/indexed via .arkai | 100% |
| Orphan notes organized | 100% |
| Plugin count | ≤ 7 |
| Weekly maintenance time | ≤ 15 min |

### ❌ DON'T:

| Anti-Pattern | Why Avoid |
|--------------|-----------|
| "80% of all notes have frontmatter" | Generates thousands of useless diffs |
| "Move every note to perfect location" | High risk, link breakage |
| "Nested folder hierarchy" | Decision fatigue, ADHD unfriendly |
| "Complex tag taxonomy" | Will be abandoned |

---

## Stop Conditions (NEW)

**Before proceeding from each phase, check:**

### Phase 3 → Phase 4:
- If `manual_review.md` contains > 10% of shard notes → **HALT**
  - Refine labeling prompts
  - Re-run labeling
  - Do NOT proceed with low-confidence data

### Phase 5 → Phase 6:
- If plan.csv has > 1% conflicts → **HALT**
  - Resolve naming collisions manually
  - Update plan
  - Re-validate

### Phase 6 → Phase 7:
- If broken links increase after apply → **ROLLBACK**
  - `git checkout HEAD~1`
  - Fix move mechanism
  - Re-apply with corrected approach

---

## Gold Standard Contracts (NEW)

Create these in `System/` folder:

### naming_contract.md
```markdown
# Naming Contract

## Daily Notes
- Format: `YYYY-MM-DD` (e.g., `2026-01-17`)
- Location: `Daily Notes/`

## Attachments
- Format: `YYYYMMDD_descriptive-name.ext`
- Location: `Attachments/`

## Project Notes
- Format: `Project - Name`
- Location: `10-Active/` while active, `90-Archive/` when done

## MOCs
- Format: `MOC - Topic Name`
- Location: Wherever the topic lives
```

### properties_contract.md
```markdown
# Properties Contract (Frontmatter)

## Required for NEW notes (via Templater):
```yaml
---
type: note | project | area | resource | daily | meeting
created: {{date}}
---
```

## Optional (add when useful):
```yaml
status: active | someday | done | archived
topics: [topic1, topic2]
updated: {{date}}
```

## NOT required for existing notes
- Only enrich high-value notes
- Everything else indexed via .arkai without frontmatter
```

### tag_contract.md
```markdown
# Tag Contract

## Approved Tags (FLAT, not nested):

### Context
- #work
- #personal
- #business

### Status
- #active
- #someday
- #archived

### Type
- #project
- #meeting
- #idea
- #reference

## Anti-Patterns (AVOID):
- Nested tags like #work/catsy/import
- One-off tags for single notes
- Tags duplicating folder structure
```

### .arkai/index_contract.md
```markdown
# Arkai Index Contract

## index.json Schema
```json
{
  "version": 1,
  "vault_id": "alex-main-vault",
  "generated": "ISO8601",
  "notes": [
    {
      "id": "stable_hash",
      "path": "relative/path.md",
      "title": "Note Title",
      "type": "project|area|resource|...",
      "topics": ["array"],
      "summary": "1-line summary",
      "is_high_value": true|false,
      "embedding_id": "ref_to_parquet_row",
      "last_indexed": "ISO8601"
    }
  ]
}
```

## Update Policy
- Full re-index: Monthly
- Incremental: On file change (via watcher)
- High-value notes: Always include summary
- Other notes: Index without summary if no frontmatter
```

---

## Model Strategy (Corrected Pricing)

| Phase | Model | Cost/MTok | Est. Cost |
|-------|-------|-----------|-----------|
| 0-2 | None | - | $0 |
| 3 | Claude Haiku 4.5 | $1 in / $5 out | ~$11 |
| 4 | None | - | $0 |
| 5 | Claude Sonnet 4.5 | $3 in / $15 out | ~$3-5 |
| 6-8 | None | - | $0 |
| **Total** | | | **~$15-20** |

### Privacy Mode (for GLM-4.7 if used)
- Default to Haiku for any note with `risk_flags`
- GLM only for "safe shards" or truncated metadata
- Never send credentials, health, legal content externally

---

## Claude Cowork Integration Points

**Good uses for Cowork (Claude Desktop):**
1. **Interactive taxonomy brainstorming** - Ask Cowork to propose folder structures
2. **Spot-checking labels** - Review random samples from labeling output
3. **MOC generation** - Have Cowork help draft MOCs for specific topics
4. **Human review assistance** - Cowork presents batches for decision

**NOT for Cowork:**
- Core pipeline execution (keep in orchestrated script for consistency)
- Bulk operations (use CLI/API for automation)

---

## Slash Command Opportunities

Could create in arkai's `.claude/commands/`:

| Command | Purpose |
|---------|---------|
| `/vault-scan` | Run Phase 1 inventory |
| `/vault-label` | Process shards through labeling |
| `/vault-plan` | Generate reorganization plan |
| `/vault-apply` | Execute moves (dry-run default) |
| `/vault-health` | Generate health report |

**Implementation:** Deferred to after core pipeline works.

---

## Maintenance System (ADHD-Optimized)

### Daily (2 min)
- Open Daily Note (auto-created via Templater)
- Brain dump anything
- Optional: Add 1-2 tags to important items

### Weekly (15 min) - "The Only Required Habit"
- Open Bases view: "Inbox Review"
- Move/tag/link items that accumulated
- Check "Needs Attention" dashboard
- Update one MOC if overwhelmed by a topic

### Monthly (30 min)
- Archive completed projects
- Run vault health check
- Review plugin updates (but don't chase new plugins!)

### Templates

**Daily Note (Templater):**
```markdown
---
type: daily
created: <% tp.date.now("YYYY-MM-DD") %>
---

# <% tp.date.now("dddd, MMMM D, YYYY") %>

## Morning
- **Focus:**

## Log
-

## Capture
-
```

**Quick Capture (Inbox):**
```markdown
---
type: note
created: <% tp.date.now("YYYY-MM-DD") %>
---

# {{title}}

{{content}}
```

---

## Next Steps

**Ready to execute? Here's the order:**

### Phase 0 + 0.5 (Today)
1. Initialize git in sandbox
2. Create .aiexclude
3. Capture baseline stats
4. Audit plugins → generate report
5. Disable make-md, clean .space folders
6. Enable Bases

### Phase 1 (Today/Tomorrow)
1. Run manifest scanner
2. Generate all inventory artifacts
3. Compute priority sets

### Human Checkpoint 1
- Review manifests
- Approve sharding strategy
- Validate exclusions working

### Phase 2-4 (After checkpoint)
1. Generate shards
2. Run labeling (Haiku 4.5)
3. Check stop conditions
4. Aggregate results

### Human Checkpoint 2
- Review topic clusters
- Review high-value list
- Address manual review items

### Phase 5-6 (After checkpoint)
1. Generate taxonomy/MOCs/contracts (Sonnet 4.5)
2. Generate conservative move plan
3. Dry-run
4. Execute via Obsidian

### Phase 7-8 (Finalization)
1. Set up maintenance templates
2. Create Bases dashboards
3. Generate .arkai/ integration layer
4. Test incremental updates

---

## Appendix: Bases Quick Reference

Bases is a **core plugin** (no installation needed, just enable) that provides database-like views:

**Basic embedded Base:**
```markdown
```bases
filters:
  and:
    - file.hasTag("project")
    - file.inFolder("10-Active")
views:
  - type: table
    name: Active Projects
```
```

**Card view for visual browsing:**
```markdown
```bases
filters:
  file.inFolder("10-Active")
views:
  - type: cards
    name: Project Cards
```
```

**Features:**
- Table and card views
- Filtering by tags, folders, properties
- Sorting and grouping
- Property editing inline
- No custom data model (just standard frontmatter)
- Works on mobile
- Core plugin = always maintained

---

*Strategy complete. Ready for execution.*

---

## Addendum: Light-Touch Zettelkasten (Added 2026-01-17)

### Decision: Option 3 - Link When It's Easy

**Context:** User is coming from OneNote with folder-based organization. No existing linked notes.

**Approach:**
- ❌ NOT doing full Zettelkasten migration
- ❌ NOT requiring atomic notes refactor
- ✅ PARA-lite folders as primary structure
- ✅ "Link when it's easy" as a maintenance habit
- ✅ 8 seed MOCs as optional entry points

### Bootstrap Strategy

Since there are no existing links, we need a **bootstrap approach**:

1. **Create 8 Seed MOCs** (Phase 5 deliverable)
   - These are optional entry points, not required navigation
   - Start sparse, grow organically

2. **Weekly Linking Habit**
   - During weekly review: add 0-3 links per important note
   - Only when connections are obvious
   - No pressure to link everything

3. **No Atomic Notes Push**
   - Keep existing note structures
   - New notes can be atomic if natural
   - Don't refactor old notes to be atomic

### 8 Seed MOCs

Create these in Phase 5 (or manually):

| MOC | Description | Initial Content |
|-----|-------------|-----------------|
| **MOC - Work (Catsy)** | Work projects, clients, processes | Links to CATSY folder contents |
| **MOC - Arkai** | This project, AI tools, automation | Links to arkai-related notes |
| **MOC - Health** | Fitness, medical, wellness | Sparse initially |
| **MOC - Finance** | Money, investments, planning | Sparse initially |
| **MOC - Relationships** | People, contacts, social | Sparse initially |
| **MOC - Polish** | Polish language, culture, family | Sparse initially |
| **MOC - Content** | Content creation, marketing | Sparse initially |
| **MOC - Ideas** | Random ideas, someday/maybe | Catch-all for creative stuff |

**MOC Template:**
```markdown
---
type: moc
created: 2026-01-17
---

# MOC - {Topic}

## Overview
{Brief description of what this MOC covers}

## Key Notes
- [[Note 1]]
- [[Note 2]]

## Sub-Topics
### {Subtopic A}
- [[Related note]]

### {Subtopic B}
- [[Related note]]

## Questions / To Explore
- {Open questions about this topic}
```

### Updated Weekly Review Habit

```markdown
## Weekly Review (15 min)

1. **Inbox Processing** (5 min)
   - Clear 00-Inbox/
   - Tag or move items

2. **Linking Pass** (5 min) ← NEW
   - Open 3-5 important notes from this week
   - Add 0-3 obvious links per note
   - "Does this remind me of anything?"
   - Don't force it

3. **MOC Check** (5 min)
   - Pick ONE seed MOC
   - Add any new relevant notes
   - Don't do all 8 every week
```

### Linking Guidelines

**DO link when:**
- You explicitly reference another note ("as I wrote in...")
- Two notes are clearly about the same project/topic
- You're creating a series (Part 1 → Part 2)
- A note answers a question posed in another note

**DON'T force links:**
- Just because topics are vaguely related
- To hit a "link quota"
- For notes you'll never revisit

### Evolution Path

```
NOW:      Folder-based (PARA-lite)
          + 8 seed MOCs
          + Weekly linking habit (0-3 links)
          ↓
3 MONTHS: Review link density
          If useful → expand MOCs
          If not → simplify further
          ↓
6 MONTHS: Consider atomic notes for NEW content
          Keep old notes as-is
          ↓
1 YEAR:   Evaluate full Zettelkasten migration
          Only if organic linking feels natural
```

### Metrics to Track (Optional)

After 3 months, check:
- How many notes have ≥1 link? (target: 20% of active notes)
- Do you use MOCs to navigate? (subjective)
- Does linking feel helpful or tedious?

If tedious → simplify further. The goal is findability, not link count.

---

## Related Documents

- **ROADMAP.md** - Future work (voice memos, Todoist, embeddings)
- **ADHD_NOTE_SYSTEM_RESEARCH.md** - Full research on note-taking methods


---

## Addendum: Chad's Pre-Execution Safeguards (Added 2026-01-17)

### 1. New Stop Condition

**Added to Phase 3 → Phase 4 gate:**
- If >5% of notes have `move_candidate.confidence < 0.85` → **HALT**
- Prevents accidental reorg spree
- Forces prompt refinement before proceeding

### 2. Plugin Strategy: Disable-First

**Phase 0.5 revised approach:**
```
Step 1: DISABLE nonessential plugins in sandbox (don't delete)
Step 2: Verify vault stability (opens, no errors)
Step 3: Test core functionality (daily notes, search, links)
Step 4: ONLY THEN consider permanent removal
```

### 3. Privacy Gating (Inventory + Labeling)

**Hard-skip (never process):**
- `.aiexclude` patterns
- `.obsidian/`
- `.trash/`
- `Attachments/` (media files, not text)
- `.space/` and `.makemd/` (Make.md artifacts)

**Secret detection patterns (flag + skip LLM):**
```regex
api[_-]?key
api[_-]?secret
token
password
private[_-]?key
secret[_-]?key
bearer
credentials
```

If pattern detected → add to `risk_flags: ["credentials"]` → skip LLM labeling for that note.

### 4. Make.md Cleanup: Quarantine-First

**Revised cleanup steps:**
```bash
# Step 1: QUARANTINE (don't delete)
mkdir -p vault-sandbox/System/_quarantine/
mv vault-sandbox/.space vault-sandbox/System/_quarantine/
mv vault-sandbox/.makemd vault-sandbox/System/_quarantine/
find vault-sandbox -name ".space" -type d -exec mv {} vault-sandbox/System/_quarantine/ \;

# Step 2: VERIFY
# Open vault in Obsidian, check:
# - Vault opens without errors
# - Daily notes work
# - Search works
# - No broken links from make-md removal

# Step 3: ONLY THEN delete (after human confirmation)
# rm -rf vault-sandbox/System/_quarantine/
```

### 5. Daily Notes Contract (LOCKED)

**Filename format:** `MM-DD-yyyy-EEE.md`
- Example: `01-17-2026-Fri.md`
- Location: `Daily Notes/` only
- Title goes in H1 inside note, NOT in filename

**Template:**
```markdown
---
type: daily
created: {{date:YYYY-MM-DD}}
---

# {{date:dddd, MMMM D, YYYY}}

## Morning
- **Focus:**

## Log
-

## Capture
-
```

### 6. Seed MOCs: Empty Scaffolds

**Phase 5 MOC approach:**
- Create 8 empty scaffold MOCs
- Add Bases/Dataview query views
- Do NOT generate LLM content yet

**MOC Scaffold Template:**
```markdown
---
type: moc
created: 2026-01-17
---

# MOC - {Topic}

## Overview
{To be filled organically}

## Notes
```dataview
LIST
FROM ""
WHERE contains(topics, "{topic}")
SORT file.mtime DESC
LIMIT 20
```

## Recent Activity
```dataview
TABLE file.mtime as "Modified"
FROM ""
WHERE contains(file.path, "{related_folder}")
SORT file.mtime DESC
LIMIT 10
```
```

---

## Pre-Execution Checklist

Before starting Phase 0:

- [ ] Quarantine approach for make-md confirmed
- [ ] Privacy gating patterns documented
- [ ] Daily Notes contract locked (MM-DD-yyyy-EEE)
- [ ] Stop condition (<0.85 confidence) added
- [ ] Plugin disable-first strategy confirmed
- [ ] MOC scaffold approach (empty + queries) confirmed


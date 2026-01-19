# Obsidian Vault Reorganization Strategy
## Steelmanned & Enhanced from Chad's (GPT 5.2) Original Plan

**Date:** 2025-01-17
**Vault:** `/Users/alexkamysz/AI/arkai/vault-sandbox/`
**Original Source:** Chad (GPT 5.2)
**Enhanced by:** Claude Opus 4.5

---

## Executive Summary

Chad's plan is **architecturally sound** (8/10) with excellent phasing, safety measures, and cost-conscious model selection. This document enhances it with:

1. **Plugin rationalization** (addressing the 37-plugin brittleness problem)
2. **Enhanced metadata extraction** (backlinks, broken links, content hashing)
3. **Arkai AIOS integration layer** (embeddings, graph, config)
4. **Smart sharding** (process chaos first, preserve good structure)
5. **Link-safe file moves** (leveraging existing plugins)

**Estimated Cost:** $2-5 for full pipeline (Claude Haiku + Sonnet)
**Estimated Time:** 2-4 hours of compute, multiple human review checkpoints

---

## Vault Baseline Metrics

| Metric | Value | Notes |
|--------|-------|-------|
| Markdown Files | 1,555 | Substantial knowledge base |
| Total Lines | 82,256 | ~500 tokens/note average |
| Media/Attachments | 686 | Images, PDFs, audio |
| Installed Plugins | 37 | 🔴 Excessive |
| Root Orphans | 18+ | Need organization |
| Trash Items | 131 | Cleanup needed |
| Estimated Tokens | ~777,500 | For full labeling pass |

---

## Enhanced Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    PHASE 0: SAFETY + BASELINE                   │
│  • Git init sandbox  • Capture metrics  • Exclude sensitive     │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                 PHASE 0.5: PLUGIN RATIONALIZATION               │
│  • Audit 37→10 plugins  • Remove duplicates  • Disable make-md  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                   PHASE 1: ENHANCED INVENTORY                   │
│  • Manifest  • Backlinks  • Broken links  • Content hashes      │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                    ═══════════════════════
                    ║  HUMAN CHECKPOINT 1  ║
                    ═══════════════════════
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                    PHASE 2: SMART SHARDING                      │
│  • Priority: orphans → imports → chaos → structured             │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                PHASE 3: LABELING (Claude Haiku)                 │
│  • Type  • Topics  • Quality  • Risk flags  • Move candidates   │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                    PHASE 4: AGGREGATION                         │
│  • Topic clusters  • Hub notes  • Duplicates  • Health report   │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                    ═══════════════════════
                    ║  HUMAN CHECKPOINT 2  ║
                    ═══════════════════════
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│               PHASE 5: PLANNING (Claude Sonnet)                 │
│  • Taxonomy  • MOCs  • Frontmatter schema  • Move plan          │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                      PHASE 6: APPLY                             │
│  • Dry-run  • Link-safe moves  • Git commits  • Validation      │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                    PHASE 7: MAINTENANCE                         │
│  • Daily/Weekly/Monthly cadence  • Dataview dashboards          │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                 PHASE 8: ARKAI INTEGRATION                      │
│  • .arkai/ config  • Embeddings  • Graph  • Incremental sync    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Detailed Phase Specifications

### Phase 0: Safety + Baseline

**Objectives:**
- Create reversible sandbox environment
- Capture baseline metrics for before/after comparison
- Establish exclusion rules for sensitive content

**Actions:**
```bash
cd vault-sandbox
git init
echo ".obsidian/workspace*" >> .gitignore
echo ".obsidian/cache" >> .gitignore
echo ".DS_Store" >> .gitignore
echo "Memory Aids/" >> .aiexclude  # Sensitive content
git add -A
git commit -m "Baseline snapshot before reorganization"
```

**Artifacts:**
- `baseline_commit.txt` - Git hash for rollback
- `baseline_stats.json` - File counts, sizes, dates
- `.aiexclude` - Files to never process with LLMs

---

### Phase 0.5: Plugin Rationalization (NEW)

**Problem:** 37 plugins = brittleness, conflicts, cognitive overload

**Plugin Audit Results:**

| Category | Plugins | Action |
|----------|---------|--------|
| **ESSENTIAL** | dataview, templater-obsidian, calendar | Keep |
| **USEFUL** | obsidian-linter, nldates, auto-note-mover | Keep |
| **REDUNDANT** | todoist-sync + ultimate-todoist-sync | Remove one |
| **REDUNDANT** | custom-sort + manual-sorting + obsidian-sortable | Keep one |
| **REDUNDANT** | neo4j-graph-view + juggl | Keep one |
| **PROBLEMATIC** | make-md | Disable (creates .space complexity) |
| **COSMETIC** | iconic, colorizelt, colored-text, highlightr | Optional |

**Recommended Minimal Plugin Set (10):**
1. dataview - Queries and dashboards
2. templater-obsidian - Templates and automation
3. calendar - Daily notes integration
4. obsidian-linter - Consistent formatting
5. nldates-obsidian - Natural language dates
6. auto-note-mover - Rule-based filing
7. copilot - AI integration (if using)
8. meld-encrypt - Encryption (if needed)
9. obsidian-advanced-uri - External integrations
10. quick-explorer - Navigation

**Artifacts:**
- `plugins_audit.json` - Full plugin analysis
- `plugins_recommended.md` - Minimal set with rationale

---

### Phase 1: Enhanced Inventory Scan

**Objectives:**
- Complete manifest of all content
- Rich metadata beyond Chad's original spec
- Priority scoring for triage

**vault_manifest.jsonl schema (enhanced):**
```json
{
  "path": "string",
  "filename": "string",
  "bytes": "number",
  "lines": "number",
  "ctime": "ISO8601",
  "mtime": "ISO8601",
  "folder": "string",
  "depth": "number",
  "title_h1": "string|null",
  "frontmatter_keys": ["array"],
  "frontmatter_values": {"object"},
  "existing_tags": ["array"],
  "wikilinks_out": ["array"],
  "wikilinks_in": ["array"],  // NEW: backlinks
  "broken_links": ["array"],  // NEW: links to non-existent notes
  "content_hash": "string",   // NEW: for duplicate detection
  "is_empty": "boolean",      // NEW: stub detection
  "word_count": "number"
}
```

**attachments_manifest.jsonl schema:**
```json
{
  "path": "string",
  "type": "image|pdf|audio|video|other",
  "bytes": "number",
  "referenced_by": ["array"],  // NEW: notes that embed this
  "is_orphan": "boolean"       // NEW: no references
}
```

**Priority Sets (priority_sets/):**
- `orphans_root.txt` - Root-level markdown files
- `orphans_unlinked.txt` - Notes with no incoming links
- `imports_pending.txt` - IMPORTED FILES TO REVIEW folder
- `largest_notes.txt` - Top 50 by size
- `most_linked.txt` - Hub notes (most incoming links)
- `broken_links.txt` - Notes with dead references
- `duplicates_candidates.txt` - Same content hash

---

### Phase 2: Smart Sharding Strategy

**Principle:** Process chaos first, preserve good structure last.

**Shard Priority Order:**
1. **Root orphans** (18 files) - Highest chaos
2. **IMPORTED FILES TO REVIEW FROM ONENOTE** (289 files) - Bulk import backlog
3. **.trash contents** (131 items) - Decide keep/delete
4. **Unstructured folders** - Mixed content
5. **Daily Notes** (structured, low priority)
6. **Periodic** (structured, low priority)
7. **CATSY** (work-related, already organized)

**Shard Limits:**
```yaml
max_notes_per_shard: 100
max_bytes_per_shard: 2_000_000  # 2MB
max_tokens_estimate: 100_000
keep_linked_together: true  # Cluster by link relationships
```

**Artifacts:**
- `shards/shard_index.json`
- `shards/shard_001_paths.txt`
- `shards/shard_002_paths.txt`
- ...

---

### Phase 3: Labeling Pass (Claude Haiku 4.5)

**Model:** Claude Haiku 4.5
- Cost: ~$0.25/MTok input, ~$1.25/MTok output
- Speed: Fast
- Trust: Full (Anthropic, user's data stays with trusted provider)

**Enhanced Label Schema:**
```json
{
  "note_id": "hash",
  "path": "string",

  // Chad's original fields
  "type": "daily|project|area|meeting|reference|journal|import|scratch|unknown",
  "summary_1line": "string",
  "topics": ["array", "3-10 items"],
  "entities": ["optional array"],
  "actionability": "actionable|reference|archive",
  "move_candidate": {"path": "string|null", "confidence": 0.0-1.0},
  "risk_flags": ["private", "sensitive", "credentials", "health", "legal"],

  // NEW: Quality indicators
  "quality_score": 1-5,  // 1=chaotic dump, 5=well-structured
  "completeness": "complete|fragment|stub",
  "temporal_relevance": "current|historical|evergreen",

  // NEW: Arkai integration
  "embedding_priority": "high|medium|low|skip",
  "key_concepts": ["array"],  // For future graph nodes
  "relationship_hints": [{"target": "note", "type": "relates_to|extends|contradicts"}]
}
```

**Prompt Template:**
```
You are analyzing a note from a personal knowledge vault.
Provide structured metadata for organization and AI integration.

Note path: {path}
Existing frontmatter: {frontmatter}
Content:
---
{content}
---

Output JSON with the specified schema. Be conservative with move_candidate
unless confident. Flag any sensitive content in risk_flags.
```

---

### Phase 4: Aggregation (No LLM)

**Computed Outputs:**

1. **global_labels.jsonl** - Merged from all shards

2. **topic_analysis.json:**
```json
{
  "topic_frequencies": {"topic": count},
  "topic_cooccurrence": [["topic1", "topic2", count]],
  "topic_clusters": [{"name": "Work", "topics": [...]}]
}
```

3. **hub_notes.json** - Notes with highest incoming link count (natural MOC candidates)

4. **vault_health_report.md:**
```markdown
## Vault Health Report

### Overview
- Total notes: 1,555
- With frontmatter: XX%
- With tags: XX%
- Orphaned (no links): XX%
- Average links per note: X.X

### Quality Distribution
- High quality (4-5): XX%
- Medium quality (3): XX%
- Low quality (1-2): XX%

### Content Age
- Last 30 days: XX notes
- Last 90 days: XX notes
- Older than 1 year: XX notes

### Issues Found
- Broken links: XX
- Duplicate content: XX candidates
- Empty/stub notes: XX
```

5. **candidates/duplicates.csv**
6. **candidates/review_queue.csv** - Notes the LLM couldn't confidently categorize

---

### Phase 5: Planning Pass (Claude Sonnet 4.5)

**Model:** Claude Sonnet 4.5
- Better reasoning for taxonomy design
- Synthesis of patterns across entire vault

**Preserve Existing Good Structure:**
- `Daily Notes/` - Keep as-is
- `Periodic/` - Keep as-is (Week, Month, Quarter, Year, Half-Year)
- `CATSY/` - Keep as-is (work-related)
- `copilot-custom-prompts/` - Keep as-is (functional)

**Proposed Taxonomy (PARA-inspired but simpler):**
```
vault/
├── 00-Inbox/           # New captures, quick notes
├── 10-Projects/        # Active, time-bound work
├── 20-Areas/           # Ongoing responsibilities
│   ├── Work/
│   ├── Personal/
│   └── Business/
├── 30-Resources/       # Reference material
│   ├── Guides/
│   ├── Templates/
│   └── Snippets/
├── 40-Archive/         # Completed, historical
├── Daily Notes/        # PRESERVED
├── Periodic/           # PRESERVED
├── CATSY/              # PRESERVED (work)
├── Attachments/        # All media consolidated
└── System/             # Templates, scripts, config
```

**Minimal Frontmatter Schema:**
```yaml
---
type: project | area | resource | archive | daily | meeting | note
status: active | someday | done | archived
topics: [topic1, topic2, topic3]
created: 2025-01-17
updated: 2025-01-17
---
```

**Tag Taxonomy (Flat, not nested):**
- Context: #work #personal #business
- Status: #active #someday #done #archived
- Type: #project #meeting #reference #idea
- Priority: #urgent #important

**Artifacts:**
- `plan/taxonomy.md` - Folder structure with rationale
- `plan/frontmatter_schema.md` - Standard fields
- `plan/tag_contract.md` - Approved tags
- `plan/mocs.md` - Maps of Content proposals
- `plan/plan.csv` - Move operations with confidence
- `plan/manual_review.md` - Items needing human decision

---

### Phase 6: Apply (Link-Safe)

**Critical Requirement:** File moves MUST update wikilinks.

**Approach Options:**

1. **Use auto-note-mover plugin** (already installed)
   - Configure rules based on frontmatter
   - Let Obsidian handle link updates

2. **Script with link rewriting:**
   ```python
   # generate link_map.json
   # scan all .md for [[old_path]]
   # replace with [[new_path]]
   # handle aliases: [[old|display]]
   # handle headings: [[old#heading]]
   # handle blocks: [[old^block-id]]
   ```

3. **Hybrid (Recommended):**
   - Generate plan.csv
   - Create Obsidian-executable script
   - Run inside Obsidian for native link updating

**Dry Run Output:**
```
=== DRY RUN: 2025-01-17 ===
MOVES: 847
CONFLICTS: 12
SKIPPED (low confidence): 45

Sample moves:
  "Altech.md" → "20-Areas/Work/Clients/Altech.md" (0.92)
  "ROOM ORGANIZATION PLAN.md" → "10-Projects/Home/Room Organization.md" (0.87)

Conflicts:
  "Meeting Notes.md" exists in both source and target

Review required for 45 notes (see plan/manual_review.md)
```

**Git Commit Strategy:**
```bash
git commit -m "Phase 6: Pre-apply snapshot"
# Apply moves
git commit -m "Phase 6: Applied reorganization plan (847 files)"
```

---

### Phase 7: Maintenance System

**ADHD-Friendly Minimal Upkeep:**

**Daily (2 min):**
- Capture in Daily Note
- Add 1-3 tags/topics
- No organization required

**Weekly (15 min):**
- Clear Inbox → proper locations
- Review `review_queue` if any
- Check Dataview "Needs Attention" dashboard

**Monthly (30 min):**
- Archive completed projects
- Prune unused attachments
- Review orphaned notes
- Update MOCs if needed

**Dataview Dashboards:**

```dataview
TABLE WITHOUT ID
  file.link as Note,
  type,
  dateformat(file.mtime, "yyyy-MM-dd") as Updated
FROM ""
WHERE !type OR !topics
SORT file.mtime DESC
LIMIT 20
```

**Artifacts:**
- `System/maintenance.md` - Cadence documentation
- `System/dashboards/` - Dataview queries
- `System/Templates/` - Quick capture templates

---

### Phase 8: Arkai Integration (NEW)

**Create `.arkai/` integration layer:**

```
vault-sandbox/
└── .arkai/
    ├── config.yaml
    ├── index.json
    ├── embeddings/
    │   └── embeddings.parquet
    ├── graph/
    │   ├── nodes.json
    │   └── edges.json
    └── exclude.txt
```

**config.yaml:**
```yaml
vault_id: "alex-main-vault"
vault_path: "/Users/alexkamysz/Documents/Obsidian Vault"
version: 1

exclude_patterns:
  - "Memory Aids/**"
  - ".obsidian/**"
  - ".trash/**"
  - ".arkai/**"

embedding:
  model: "text-embedding-3-small"
  dimensions: 1536
  batch_size: 100

indexing:
  last_full_index: null
  incremental_enabled: true
  watch_for_changes: false

graph:
  extract_entities: true
  extract_relationships: true
  max_hops: 3
```

**index.json schema:**
```json
{
  "version": 1,
  "generated": "ISO8601",
  "notes": [
    {
      "id": "hash",
      "path": "relative/path.md",
      "title": "string",
      "type": "string",
      "topics": ["array"],
      "summary": "string",
      "embedding_id": "ref to parquet row",
      "last_indexed": "ISO8601"
    }
  ]
}
```

**Integration with arkai AIOS:**
- arkai can read `.arkai/index.json` for quick lookups
- Embeddings in parquet for efficient similarity search
- Graph data for relationship traversal
- Incremental updates via file watching

---

## Model Selection Summary

| Phase | Model | Rationale | Est. Cost |
|-------|-------|-----------|-----------|
| 0-2 | None | Pure scripting | $0 |
| 3 | Claude Haiku 4.5 | Fast, cheap, trusted | ~$1.20 |
| 4 | None | Aggregation only | $0 |
| 5 | Claude Sonnet 4.5 | Better synthesis | ~$0.50 |
| 6-8 | None | Apply + config | $0 |
| **Total** | | | **~$2-5** |

---

## Execution Checkpoints

### Checkpoint 1 (After Phase 1)
**Human reviews:**
- [ ] vault_manifest looks correct
- [ ] Priority sets make sense
- [ ] Exclusions working (Memory Aids not scanned)

### Checkpoint 2 (After Phase 4)
**Human reviews:**
- [ ] Topic clusters are meaningful
- [ ] Quality scores seem accurate
- [ ] Review queue is reasonable size
- [ ] No sensitive content in labels

### Checkpoint 3 (After Phase 5)
**Human reviews:**
- [ ] Taxonomy makes sense for your workflow
- [ ] Frontmatter schema is acceptable
- [ ] Move plan looks reasonable
- [ ] Manual review items addressed

### Checkpoint 4 (After Phase 6)
**Human reviews:**
- [ ] Dry run shows no critical conflicts
- [ ] Links updated correctly (spot check)
- [ ] Can rollback if needed

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Data loss | Git-backed, original vault untouched |
| Broken links | Link-safe move strategy |
| Plugin conflicts | Plugin rationalization first |
| Sensitive data exposure | `.aiexclude` + Claude-only for personal |
| Over-automation | Human checkpoints, conservative moves |
| Scope creep | Phased approach, can stop anytime |

---

## Success Criteria

1. **Organization:** ≥80% of notes in proper folder with frontmatter
2. **Discoverability:** MOCs for major topic areas
3. **Maintenance:** ≤15 min/week upkeep
4. **LLM-Ready:** `.arkai/` integration layer functional
5. **Stability:** ≤10 essential plugins, no brittleness
6. **Reversibility:** Full git history, can rollback any phase

---

## Next Steps

**Ready to execute Phase 0 + 0.5 + 1?**

This will:
1. Initialize git in sandbox
2. Create exclusion rules
3. Audit plugins
4. Generate full inventory manifests

No LLM calls yet, pure analysis. Then human review before proceeding.

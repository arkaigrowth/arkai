# Compaction Summary: Obsidian Vault Reorganization
## Session Handoff Document (2026-01-17)

**Purpose:** This document captures all context needed to continue execution after compaction.

---

## Project Overview

**Goal:** Systematically review, reorganize, and future-proof an Obsidian vault for ADHD-friendly use and LLM/arkai integration.

**Vault Location:** `/Users/alexkamysz/AI/arkai/vault-sandbox/`
**Original Vault:** `/Users/alexkamysz/Documents/Obsidian Vault` (DO NOT MODIFY)
**Symlink:** `~/Documents/Obsidian Vault SANDBOX (arkai)`

**Vault Stats:**
- 1,555 markdown files
- 82,256 total lines
- 686 media/attachments
- 37 plugins (to reduce to 7)
- Memory Aids folder: DELETED from sandbox (was sensitive)

---

## Key Documents (Read These First)

| Document | Purpose | Location |
|----------|---------|----------|
| **STRATEGY_FINAL_v2.md** | Complete execution plan with all corrections | `scout_outputs/research/obsidian-reorg/` |
| **ROADMAP.md** | Future work backlog (voice memos, Todoist, etc.) | `scout_outputs/research/obsidian-reorg/` |
| **ADHD_NOTE_SYSTEM_RESEARCH.md** | Research foundation | `scout_outputs/research/obsidian-reorg/` |

---

## Key Decisions Made

### Organization Approach
- **PARA-lite folders** as primary structure (not full PARA)
- **8 seed MOCs** as optional entry points (empty scaffolds + queries)
- **Light-touch Zettelkasten:** "Link when obvious" habit, NOT full migration
- **No atomic notes refactor** for existing content

### Folder Structure
```
vault/
├── 00-Inbox/
├── 10-Active/
├── 20-Reference/
├── 90-Archive/
├── Attachments/
├── System/
├── Daily Notes/     # KEEP AS-IS
├── Periodic/        # KEEP AS-IS
└── CATSY/           # KEEP AS-IS
```

### Plugin Strategy (37 → 7)
**Keep:** Bases (core), Daily Notes (core), Templates (core), Templater, Calendar, Dataview, Auto-note-mover
**Remove:** make-md and 30 others
**Approach:** DISABLE first, verify stability, then remove

### Contracts Locked
- **Daily Notes:** Filename = `MM-DD-yyyy-EEE.md` (title in H1, not filename)
- **Frontmatter:** Only required for NEW notes and HIGH-VALUE notes
- **Tags:** Flat taxonomy (#work, #personal, #project, etc.)

### Model Strategy
- **Labeling:** Claude Haiku 4.5 ($1/$5 per MTok) ≈ $11
- **Planning:** Claude Sonnet 4.5 ($3/$15 per MTok) ≈ $3-5
- **Total estimate:** ~$15-20

---

## Stop Conditions

1. If `manual_review.md` > 10% of shard notes → HALT
2. If >5% of notes have `move_candidate.confidence < 0.85` → HALT
3. If conflicts > 1% of moves → HALT
4. If broken links increase after apply → ROLLBACK

---

## Privacy Gating

**Hard-skip (never process):**
- `.aiexclude` patterns
- `.obsidian/`, `.trash/`, `Attachments/`
- `.space/`, `.makemd/`

**Secret detection (flag + skip LLM):**
- Patterns: `api[_-]?key`, `token`, `password`, `private[_-]?key`, `credentials`

---

## Current Status

**Phase:** Ready to execute Phase 0 + 0.5 + 1

**Pre-execution checklist:**
- [x] Strategy document complete
- [x] Roadmap/backlog created
- [x] Chad's safeguards added
- [x] Quarantine approach documented
- [x] Privacy gating documented
- [x] Daily Notes contract locked
- [ ] **NEXT:** Execute Phase 0 (git init, .aiexclude, baseline)
- [ ] **THEN:** Execute Phase 0.5 (plugin disable, make-md quarantine)
- [ ] **THEN:** Execute Phase 1 (inventory scan)
- [ ] **THEN:** Human checkpoint review

---

## Execution Order (Next Session)

### Phase 0: Safety + Baseline
```bash
cd /Users/alexkamysz/AI/arkai/vault-sandbox
git init
# Create .gitignore
# Create .aiexclude
# Capture baseline stats
git add -A && git commit -m "Baseline snapshot"
```

### Phase 0.5: Plugin Rationalization
```bash
# 1. QUARANTINE make-md artifacts
mkdir -p System/_quarantine/
# Move .space and .makemd folders to quarantine

# 2. In Obsidian: DISABLE (not delete) nonessential plugins
# 3. Verify vault stability
# 4. Document plugin audit results
```

### Phase 1: Enhanced Inventory
- Generate `vault_manifest.jsonl`
- Generate `attachments_manifest.jsonl`
- Generate `plugin_manifest.json`
- Generate priority sets
- Apply privacy gating (skip secrets)

---

## 8 Seed MOCs (Create Empty Scaffolds)

1. MOC - Work (Catsy)
2. MOC - Arkai
3. MOC - Health
4. MOC - Finance
5. MOC - Relationships
6. MOC - Polish
7. MOC - Content
8. MOC - Ideas

**Approach:** Empty scaffolds with Dataview/Bases queries, NOT LLM-generated content.

---

## Collaborators

- **Chad (GPT 5.2):** Original plan author, provided corrections and safeguards
- **Claude Opus 4.5:** Strategy enhancement, synthesis, execution
- **Claude Cowork:** Available for interactive review (not core execution)

---

## Key Principles (ADHD-Aligned)

1. "Findability over perfection"
2. "A messy collection of searchable notes beats a perfect system with nothing in it"
3. "Limit the places a thing can be"
4. "Link when it's easy, not everywhere"
5. "Disable first, verify, then remove"
6. "Quarantine before delete"

---

## Resume Instructions

After compaction, start with:

1. Read `STRATEGY_FINAL_v2.md` for full context
2. Confirm quarantine + privacy gating are understood
3. Execute Phase 0 → 0.5 → 1 in sequence
4. Stop at human checkpoint after Phase 1

**Do NOT proceed to Phase 2+ without human review of Phase 1 artifacts.**

---

*Compaction summary complete. Ready for /compact.*

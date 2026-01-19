# Olek's ADHD-Friendly Obsidian Vault

> Drop-in config. Zero friction daily notes. Auto-organization. Graph-aware.

---

## Quick Start (5 minutes)

### 1. Install Required Plugins
Open Settings → Community Plugins → Browse → Install & Enable each:

| Plugin | Purpose |
|--------|---------|
| **Calendar** | Click any day → instant daily note |
| **Periodic Notes** | Weekly/monthly aggregation |
| **Templater** | Smart templates with auto-frontmatter |
| **Dataview** | Query notes dynamically |
| **Recent Files** | Sidebar showing recently touched |
| **Omnisearch** | Fuzzy search everything |
| **Hover Editor** | Preview without opening |
| **Smart Connections** | AI-powered linking (local/BYOK) |
| **Auto Note Mover** | Rules-based auto-filing |
| **Tag Wrangler** | Rename/merge tags |
| **Colorful Folders** | Color-code folders |
| **Supercharged Links** | Visual link styling |
| **Copilot** | AI tagging (Claude/Ollama) |

### 2. Set Your Hotkeys
Settings → Hotkeys → Search and set:

| Action | Suggested Hotkey |
|--------|------------------|
| Daily note: Open today | `Cmd/Ctrl + D` |
| Quick switcher | `Cmd/Ctrl + O` |
| Omnisearch: Search | `Cmd/Ctrl + Shift + O` |
| Templater: Insert template | `Cmd/Ctrl + T` |

### 3. Configure Core Plugins
Copy the settings from the `.obsidian` folder (or configure manually):

**Daily Notes:**
- Date format: `YYYY-MM-DD`
- New file location: `01-Daily`
- Template: `Templates/Daily Note`

**Templater:**
- Template folder: `Templates`
- Trigger on new file creation: ON

---

## How It All Works Together

```
┌─────────────────────────────────────────────────────────────────┐
│                      YOUR WORKFLOW                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [Cmd+D] → Daily Note Created                                   │
│      │                                                           │
│      ▼                                                           │
│  ┌──────────────────────────────────────────────────┐           │
│  │ Daily Note (auto-filled frontmatter)             │           │
│  │                                                   │           │
│  │ ## Tasks                                          │           │
│  │ - [ ] #catsy Fix API bug                         │ ──┐       │
│  │ - [ ] Review PRD                                  │   │       │
│  │                                                   │   │       │
│  │ ## Notes                                          │   │       │
│  │ Meeting with CJ about [[Project X]]              │   │       │
│  │                                                   │   │       │
│  │ #arkai client wants automation                   │   │       │
│  └──────────────────────────────────────────────────┘   │       │
│      │                                                   │       │
│      │ Auto Note Mover (if full note has tag)           │       │
│      ▼                                                   │       │
│  ┌──────────────┐  ┌──────────────┐                     │       │
│  │ 02-Work/Catsy│  │ 02-Work/Arkai│                     │       │
│  └──────────────┘  └──────────────┘                     │       │
│                                                          │       │
│      Dataview queries pull tasks/items ◄─────────────────┘       │
│      from daily notes by tag                                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## The Triage System (Your Question Answered)

### The Core Problem You Had
> "What if I want just certain line items in daily notes to have a tag applied?"

**Answer: You DON'T move line items. You TAG them and QUERY them.**

### How It Works

1. **Daily notes stay in `01-Daily`** - They're your capture layer
2. **Tags on individual items** make them queryable across all daily notes
3. **Dataview queries** in project notes pull relevant items
4. **Links** (`[[Project X]]`) create graph connections

### Example Daily Note Entry

```markdown
## Tasks
- [ ] #catsy/urgent Fix the import bug @Jamie mentioned
- [ ] #arkai Draft proposal for new client
- [x] #personal Schedule dentist ✅ 2025-01-18

## Notes
Had a call with [[Ball & Doggett]] about PIM integration.
They need #catsy/api help with attribute mapping.

Idea for #arkai: offer "data health audit" as lead magnet.
```

### Then in `02-Work/Catsy/Dashboard.md`:

```dataview
TASK
FROM "01-Daily"
WHERE contains(tags, "#catsy") AND !completed
GROUP BY file.link
```

This **pulls all Catsy tasks from ALL daily notes** into one view.

---

## Auto Note Mover Rules (When to Actually Move)

Auto Note Mover is for **full notes**, not line items.

### Configure These Rules (Settings → Auto Note Mover):

| Tag | Destination Folder |
|-----|-------------------|
| `#meeting/catsy` | `02-Work/Catsy/Meetings` |
| `#meeting/arkai` | `02-Work/Arkai/Meetings` |
| `#project` | `03-Projects` |
| `#reference` | `04-Reference` |
| `#archive` | `05-Archive` |

### Title-based Rules (Regex):

| Pattern | Destination |
|---------|-------------|
| `^\d{4}-\d{2}-\d{2}$` | `01-Daily` (daily notes) |
| `^Weekly - ` | `01-Daily/Weekly` |

**When a note STAYS in the daily folder:**
- Quick captures, tasks, journal entries
- Items you reference via Dataview

**When a note MOVES:**
- Meeting notes (tag → auto-move)
- Reference docs (tag → auto-move)
- Project briefs (tag → auto-move)

---

## Frontmatter Configuration (Templater)

### My Recommendation: Minimal Frontmatter

Don't over-engineer. Start with:

```yaml
---
created: {{date:YYYY-MM-DD}}
type: daily | note | meeting | project
tags: []
---
```

### Why Minimal?
- Tags in frontmatter = queryable via Dataview
- Inline tags (`#catsy`) = quick capture, still queryable
- `type` = enables filtering by note type
- `created` = essential for time-based queries

### Advanced (If You Need It Later)

```yaml
---
created: {{date:YYYY-MM-DD}}
modified: {{date:YYYY-MM-DD}}
type: daily
status: active | complete | archived
project: 
related: []
tags: []
---
```

---

## Quick Navigation Patterns

### See Recent Files
- **Recent Files plugin** → Always visible in left sidebar
- Shows last 15 files touched (configurable)

### Jump Between Daily Notes
- **Calendar plugin** → Click any date
- **Hotkey**: `Cmd+D` for today
- **In daily note template**: Links to yesterday/tomorrow (see template)

### Weekly Aggregation
The Weekly template (below) auto-generates views of:
- All daily notes that week
- All tasks created that week
- All notes modified that week

---

## Dataview Queries You'll Actually Use

### All Incomplete Tasks (Anywhere)
```dataview
TASK
WHERE !completed
GROUP BY file.link
LIMIT 50
```

### Tasks from This Week
```dataview
TASK
FROM "01-Daily"
WHERE file.cday >= date(today) - dur(7 days)
WHERE !completed
```

### Notes Modified Today
```dataview
LIST
WHERE file.mday = date(today)
SORT file.mtime DESC
```

### Notes Modified This Week
```dataview
TABLE file.mtime as "Modified"
WHERE file.mday >= date(today) - dur(7 days)
SORT file.mtime DESC
```

### All Notes Linking to Current Note
```dataview
LIST
FROM [[]]
```

### Recent Catsy Work
```dataview
LIST
FROM "02-Work/Catsy" OR #catsy
WHERE file.mday >= date(today) - dur(7 days)
SORT file.mtime DESC
```

---

## Smart Connections Setup (Local AI)

### Option A: Fully Local (Ollama)
1. Install Ollama: `brew install ollama`
2. Pull embedding model: `ollama pull nomic-embed-text`
3. Pull chat model: `ollama pull llama3.2`
4. Start Ollama: `ollama serve`
5. In Smart Connections settings:
   - Notes Embedding Model: `nomic-embed-text`
   - Model Platform: Custom Local (OpenAI format)
   - Chat Model: `llama3.2`
   - API Base URL: `http://localhost:11434`

### Option B: Claude API (BYOK)
1. In Smart Connections settings:
   - Model Platform: Anthropic
   - API Key: Your Claude API key
   - Model: `claude-sonnet-4-20250514`

### What It Does
- **Connections View**: Shows semantically similar notes as you work
- **Smart Chat**: Ask questions about your notes
- **Auto-linking suggestions**: "This note relates to..."

---

## Graph View Strategy

### Making Links Useful

1. **Link liberally**: `[[Person Name]]`, `[[Project]]`, `[[Company]]`
2. **Use consistent naming**: Always `[[Ball & Doggett]]` not `Ball and Doggett`
3. **Create hub notes**: `[[Catsy Clients]]` that links to all client notes

### Graph Filters (Right-click graph)
- Filter by folder: Show only `02-Work`
- Filter by tag: Show only `#catsy`
- Depth: How many hops to show

---

## Folder Color Coding (Colorful Folders)

After installing, Settings → Colorful Folders:

| Folder | Color Suggestion |
|--------|-----------------|
| `00-Inbox` | Red (needs attention) |
| `01-Daily` | Blue (routine) |
| `02-Work` | Green (active) |
| `03-Projects` | Purple |
| `04-Reference` | Gray |
| `05-Archive` | Light gray |

---

## CSS Snippet for ADHD-Friendly UI

Already included in `.obsidian/snippets/adhd-friendly.css`:

- Softer colors
- Better visual hierarchy
- Reduced visual noise
- Color-coded tags

Enable: Settings → Appearance → CSS Snippets → Toggle ON

---

## Troubleshooting

### "Plugins aren't loading"
- Settings → Community Plugins → Turn off Restricted Mode
- Restart Obsidian

### "Dataview shows errors"
- Make sure code block uses `dataview` not `dataviewjs`
- Check for typos in field names

### "Auto Note Mover isn't moving notes"
- Check trigger is set to "Automatic"
- Tags need `#` prefix in rules
- Check for `AutoNoteMover: disable` in frontmatter

### "Smart Connections is slow"
- Large vaults take time to index initially
- Use local embedding model for privacy + speed

---

## Next Right Move

1. ✅ Drop this vault folder into your Obsidian vault location
2. ✅ Install the plugins listed above
3. ✅ Set `Cmd+D` → daily note hotkey
4. ✅ Create today's daily note, add 3 tasks with tags
5. ✅ Open Connections View → see AI suggestions

**That's it. Start using it. Tweak later.**

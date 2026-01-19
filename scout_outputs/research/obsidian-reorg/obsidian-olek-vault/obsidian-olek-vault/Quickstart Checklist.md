# Quickstart Checklist

## Your Pain Points → Solutions

| Pain Point | Solution | Plugin |
|------------|----------|--------|
| Too much friction opening Obsidian | Auto-opens to daily note | **Homepage** |
| Can't find things | Fuzzy search everything | **Omnisearch** |
| No auto-organization | Rules-based filing | **Auto Note Mover** |
| Can't preview files in folder | Hover to preview any file | **Hover Editor** + **Quick Explorer** |
| No quick way to see recent files | Sidebar with recent files | **Recent Files** |
| Navigate between daily notes | Click calendar dates | **Calendar** |

---

## 5-Minute Setup

### Step 1: Copy Vault (1 min)
```bash
# Unzip and move to your preferred location
unzip obsidian-olek-vault.zip -d ~/Documents/
```

### Step 2: Open in Obsidian (30 sec)
1. Open Obsidian
2. "Open folder as vault" → select `obsidian-olek-vault`

### Step 3: Enable Community Plugins (30 sec)
1. Settings → Community Plugins
2. Turn OFF "Restricted Mode"
3. Close settings

### Step 4: Install Plugins (3 min)
Settings → Community Plugins → Browse → Install + Enable each:

**Install in this order:**
1. ✅ **Homepage** — Auto-opens daily note on launch
2. ✅ **Calendar** — Click dates to navigate
3. ✅ **Periodic Notes** — Daily/weekly note management
4. ✅ **Templater** — Smart templates
5. ✅ **Dataview** — Query your notes
6. ✅ **Recent Files** — See recently edited
7. ✅ **Omnisearch** — Find anything fast
8. ✅ **Hover Editor** — Preview on hover
9. ✅ **Quick Explorer** — Better folder navigation
10. ✅ **Auto Note Mover** — Auto-file by tag

**Optional but nice:**
- Smart Connections (AI-powered linking)
- Tag Wrangler (bulk rename tags)
- Colorful Folders (color-code folders)

### Step 5: Restart Obsidian
Close and reopen. Your daily note should auto-open!

---

## How Each Pain Point Is Solved

### 🚀 "Too much friction to open and create daily note"
**Solution:** Homepage plugin auto-creates and opens today's daily note when you launch Obsidian. Zero clicks.

Config already set in `.obsidian/plugins/homepage/data.json`

### 🔍 "Can't find things"
**Solution:** `Cmd+Shift+O` opens Omnisearch. Type anything. Fuzzy matches content, titles, tags.

### 📁 "No auto-organization"  
**Solution:** Auto Note Mover. Tag a note `#meeting/catsy` → it moves to `02-Work/Catsy/Meetings` automatically.

Rules already configured:
- `#meeting/catsy` → Catsy Meetings
- `#meeting/arkai` → Arkai Meetings
- `#project` → Projects folder
- `#reference` → Reference folder

### 👁️ "Can't preview files without opening"
**Solution:** 
- **Hover Editor** — Hold `Cmd` and hover over any link = instant preview
- **Quick Explorer** — Click breadcrumbs at top = see folder contents with previews
- **Page Preview** (core) — Hover any internal link

### 📅 "Navigate between daily notes"
**Solution:** Calendar plugin in left sidebar. Click any date = opens that day's note.

Daily note template includes prev/next links:
```
<< [[2025-01-17]] | [[2025-01-19]] >>
```

### 📋 "See recently edited files"
**Solution:** Recent Files plugin shows in right sidebar. Always visible. Click to open.

---

## Verify It's Working

After setup, test each:

| Test | Expected Result |
|------|-----------------|
| Open Obsidian | Today's daily note auto-opens |
| `Cmd+D` | Opens today's daily note |
| `Cmd+Shift+O` | Omnisearch opens |
| Hover + `Cmd` over `[[link]]` | Preview popup |
| Add `#meeting/catsy` to new note | Note moves to Catsy folder |
| Look at right sidebar | Recent Files list visible |
| Look at left sidebar | Calendar visible |

---

## If Something Doesn't Work

### "Daily note doesn't auto-open"
→ Check Homepage plugin is installed AND enabled
→ Settings → Homepage → "Open on startup" should be ON

### "Templates don't apply"
→ Check Templater plugin is enabled
→ Settings → Templater → Template folder = `Templates`
→ Enable "Trigger on new file creation"

### "Auto Note Mover not moving"
→ Check trigger is "Automatic" not "Manual"
→ Tags need `#` prefix in rules
→ Restart Obsidian after config changes

### "Hover preview not working"
→ Enable core Page Preview: Settings → Core Plugins → Page Preview ON
→ Make sure Hover Editor is installed

---

## You're Done!

The system is designed for:
1. **Open Obsidian** → daily note is there
2. **Dump thoughts** → tag with `#catsy`, `#arkai`, etc.
3. **Find later** → Omnisearch or Dataview queries
4. **Review** → Dashboards auto-aggregate by tag

No manual filing. No hunting for files. Just capture and tag.

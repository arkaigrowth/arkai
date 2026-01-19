# Frontend Options for arkai Library Viewer

> **Date**: 2026-01-17 | **Status**: Research/Future | **Priority**: After CLI stabilizes

---

## Context

"Rendering" in the CLI context means generating `transcript.md` files from raw + diarization. But a visual UI to browse/interact with library content is a separate (future) layer.

---

## What Are Similar Apps Built With?

| App | Technology | Stack |
|-----|------------|-------|
| **Claude Desktop** | Electron | Chromium + Node.js (TypeScript/React) |
| **ChatGPT Desktop** | Electron | Web app wrapped as native |
| **Cursor IDE** | Electron | VS Code fork (TypeScript + React) |

**Electron pattern**: Web technologies running in bundled Chromium with Node.js for system access. Heavy (~150-300MB), but fast to develop.

---

## Frontend Options Ranked

### Tier 1: Quick & Dirty (Days)

| Option | Effort | Notes |
|--------|--------|-------|
| **Markdown preview in VS Code** | 0 | Already works |
| **Obsidian vault** | 1 day | Point at `~/AI/library/`, get linking + search free |
| **Static HTML generator** | 2-3 days | Generate browsable HTML from library |

### Tier 2: Lightweight Interactive (Weeks)

| Option | Effort | Why Consider |
|--------|--------|--------------|
| **Tauri + Svelte 5** | 2-4 weeks | Rust backend (matches arkai!), ~10MB binary |
| **Tauri + SolidJS** | 2-4 weeks | Similar, SolidJS is very fast |
| **Terminal UI (Ratatui)** | 1-2 weeks | Pure Rust, stays in terminal |

### Tier 3: Full Desktop App (Months)

| Option | Effort | Why Consider |
|--------|--------|--------------|
| **Electron + React** | 1-2 months | Most examples, but heavy |
| **SwiftUI (macOS)** | 1-2 months | Best Mac integration, Mac-only |
| **Flutter** | 1-2 months | Cross-platform, good for mobile |

---

## Recommendation: Tauri + Svelte 5 (Runes)

**Why this combo for arkai?**

1. **Tauri is Rust** - arkai is Rust. Share code/types between CLI and GUI.

2. **Svelte 5 Runes** - Modern reactive syntax, tiny JS output:
   ```svelte
   <script>
     let timestamp = $state(0);
     let speaker = $derived(lookupSpeaker(timestamp));
   </script>

   <TranscriptLine {timestamp} {speaker} />
   ```

3. **Small binary** - ~10-20MB vs Electron's 150MB+

4. **Native feel** - Uses system webview, not bundled Chromium

5. **Clean IPC** - Call Rust from JS easily:
   ```rust
   #[tauri::command]
   fn get_library_items() -> Vec<LibraryItem> {
       arkai::library::list_all()
   }
   ```

---

## Transcript Viewer Mockup

```
┌─────────────────────────────────────────────────────────────────┐
│ arkai Library                                    🔍 Search...   │
├──────────────────┬──────────────────────────────────────────────┤
│ 📁 YouTube       │  Building AI Agents (iKwRWwabkEc)            │
│   ├─ Video 1     │  ─────────────────────────────────────────── │
│   ├─ Video 2     │  [00:00:02] [Daniel] Hey, what's up?         │
│   └─ Video 3 ◀   │  [00:00:05] [Daniel] I want to talk about... │
│ 📁 Web           │                                              │
│ 📁 Other         │  [KEYFRAME] ┌────────────┐                   │
│                  │             │ 🖼️ 0005.jpg │                   │
│                  │             └────────────┘                   │
│                  │                                              │
│                  │  [00:00:12] [Guest] Yeah, that's a great...  │
│──────────────────│  ─────────────────────────────────────────── │
│ 🏷️ Tags          │  📊 Evidence from this content:              │
│   ai (12)        │  • Claim about AI agents (line 45)           │
│   youtube (8)    │  • Quote about automation (line 89)          │
└──────────────────┴──────────────────────────────────────────────┘
```

**Key features:**
- Click timestamp → jump to YouTube at that moment
- Click keyframe → expand image
- Click evidence → see full context + validation
- Search across all transcripts
- Filter by speaker

---

## Quick Win: Obsidian Vault

Zero code required:

```bash
# Create Obsidian vault pointing at library
ln -s ~/AI/library ~/Documents/arkai-vault
```

Obsidian gives you:
- Markdown rendering
- Full-text search
- Graph view (content relationships)
- Free, works immediately

---

## Difficulty Assessment

| Approach | Difficulty | Time to MVP |
|----------|------------|-------------|
| **Obsidian vault** | Easy | 1 day |
| **Static HTML site** | Easy | 3-5 days |
| **Terminal UI (Ratatui)** | Medium | 1-2 weeks |
| **Tauri + Svelte** | Medium | 2-4 weeks |
| **Electron app** | Medium | 3-4 weeks |

---

## Decision

**Current priority**: CLI first. Frontend is Phase 2+.

**When ready**: Start with Tauri + Svelte 5 scaffold, matches Rust ecosystem.

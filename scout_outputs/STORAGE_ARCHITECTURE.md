# Storage Architecture: RESOLVED ✅

> **Updated**: 2026-01-17 | **Status**: Implemented and consolidated

---

## Current State (Clean!)

```
~/AI/
├── library/                    # CANONICAL content storage
│   ├── youtube/                # 5 videos
│   ├── web/
│   └── other/
│
├── arkai/                      # The Rust tool (code)
│   └── .arkai/config.yaml      # Points to ~/AI/library/
│
└── fabric-arkai/               # Fabric configs/scripts
    └── scripts/playlist-sync.fish

~/.arkai/                       # Engine state (derived)
├── catalog.json                # 5 YouTube entries
├── processed_videos.txt        # 5 tracked IDs
└── runs/                       # Event logs
```

**Everything synced:** 5 library folders = 5 catalog entries = 5 tracked IDs ✓

---

## The Problem (Historical)

Previously had content scattered across:
- `~/.arkai/library/` (arkai default)
- `~/AI/fabric-arkai/library/` (documented location)
- `~/AI/library/` (partial)

Plus orphaned catalog entries and tracking file mismatches.

---

## The Solution (Implemented)

### 1. Canonical Location: `~/AI/library/`

**Why this location:**
- **Visible** — not hidden in dotfiles
- **Tool-agnostic** — not tied to arkai or fabric-arkai repos
- **Clean architecture** — separates CODE from DATA
- **Easy to backup** — one folder for all AI content

### 2. Configuration

**arkai config** (`~/AI/arkai/.arkai/config.yaml`):
```yaml
version: "1.0"

paths:
  library: ../library  # Relative to project root → ~/AI/library/

  content_types:
    youtube: youtube
    web: web
    other: other
```

**playlist-sync.fish** updated:
```fish
set -g WISDOM_DIR ~/AI/library/youtube
```

### 3. arkai.md Skill Documentation

Updated to reflect:
- Library: `~/AI/library/youtube/Title (video_id)/`
- Artifacts: `fetch.md`, `wisdom.md`, `summary.md`, `metadata.json`

---

## Storage Principles

| Principle | Implementation |
|-----------|----------------|
| **Files as source of truth** | Human-readable, git-trackable, portable |
| **Derived data in ~/.arkai/** | Catalog, indexes — can regenerate |
| **Single canonical location** | All content in `~/AI/library/` |
| **Human-readable folder names** | `Video Title (video_id)/` format |

---

## Transcript Format

Raw transcripts use timestamp markers from `fabric -y URL --transcript-with-timestamps`:

```
[00:00:02] Hey, what's up? So, I want to ask and
[00:00:04] answer a question that I think is really
[00:00:06] crucial right now regarding AI...
```

**Format**: `[HH:MM:SS] transcript text`

---

## Future Considerations

### Vector DB (Optional Layer)
- Add semantic search via LanceDB or similar
- Index from files, not replace them
- Store in `~/.arkai/vectors.lance`

### Graph DB (Future)
- Relationship queries, cross-content connections
- Entity linking across library
- Separate from primary storage

---

## Migration Checklist (Completed)

- [x] Consolidated 3 locations → `~/AI/library/`
- [x] Created `~/AI/arkai/.arkai/config.yaml`
- [x] Updated `playlist-sync.fish` WISDOM_DIR
- [x] Fixed catalog (removed orphans, added missing)
- [x] Synced tracking file with actual library
- [x] Updated AIOS_BRIEF.md paths
- [x] Updated arkai.md skill documentation
- [x] Cleaned up old empty directories

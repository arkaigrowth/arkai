# Enhanced Library Schema Specification

> **Version**: 1.2 | **Status**: Proposed | **Authors**: Claude + Chad (steelmanned)

---

## Design Principles

1. **`transcript_raw.md` is canonical** — Evidence grounding (byte offsets, slice hashes) validates against this file
2. **No headers in canonical files** — Byte offsets must be stable; metadata lives in `metadata.json`
3. **Diarization is derived** — Can be regenerated, improved, or replaced without invalidating evidence
4. **Views are rebuildable** — `transcript.md` can be deleted and regenerated from raw + overlays
5. **Files are truth** — Indexes, views, and derivations can always be reconstructed
6. **Provenance tracking** — Know which tool/model generated each derived artifact

---

## File Schema

### Library Folder Structure (Enhanced)

```
~/AI/library/youtube/Video Title (video_id)/
│
├── metadata.json           # ALL metadata + artifact pointers (CANONICAL)
│
│  ═══════════════════════════════════════════════════
│  MEDIA LAYER (from video-download)
│  ═══════════════════════════════════════════════════
├── video.mp4               # Raw video capture (optional, or URI pointer)
├── audio.m4a               # Audio extracted from video (optional, or URI pointer)
│
│  ═══════════════════════════════════════════════════
│  TRANSCRIPT LAYER (canonical grounding)
│  ═══════════════════════════════════════════════════
├── transcript_raw.md       # CANONICAL - timestamped lines ONLY, NO header
│                           # Evidence spans validate against THIS file
│
│  ═══════════════════════════════════════════════════
│  DIARIZATION LAYER (derived, improvable)
│  ═══════════════════════════════════════════════════
├── diarization.jsonl       # Speaker segments with meta provenance
├── speakers.json           # Human-friendly name mapping (structured schema)
├── transcript.md           # RENDERED VIEW - raw + speakers (rebuildable)
│
│  ═══════════════════════════════════════════════════
│  INSIGHTS LAYER (from fabric patterns)
│  ═══════════════════════════════════════════════════
├── wisdom.md               # AI-extracted insights
├── summary.md              # Condensed summary
│
│  ═══════════════════════════════════════════════════
│  VISUAL LAYER (derived, from video-ops)
│  ═══════════════════════════════════════════════════
└── keyframes/
    ├── keyframe_0001.jpg
    ├── keyframe_0002.jpg
    └── keyframes_index.json
```

---

## File Format Specifications

### metadata.json (CANONICAL METADATA)

```json
{
  "schema_version": 2,
  "id": "iKwRWwabkEc",
  "content_type": "youtube",

  "source": {
    "url": "https://youtube.com/watch?v=iKwRWwabkEc",
    "title": "Building Your Own Unified AI Assistant Using Claude Code",
    "channel": "IndyDevDan",
    "duration_seconds": 1698,
    "published_at": "2026-01-10T00:00:00Z"
  },

  "ingest": {
    "ingested_at": "2026-01-17T03:24:00Z",
    "ingested_by": "arkai v1.1.0",
    "tags": ["ai", "youtube", "claude"]
  },

  "artifacts": {
    "transcript_raw": {
      "path": "transcript_raw.md",
      "hash": "sha256:abc123...",
      "generated_at": "2026-01-17T03:24:05Z",
      "tool": "fabric",
      "tool_version": "1.4.0"
    },
    "video": {
      "path": "video.mp4",
      "uri": null,
      "size_bytes": 524288000,
      "hash": "sha256:def456..."
    },
    "audio": {
      "path": "audio.m4a",
      "uri": "s3://arkai-media/iKwRWwabkEc/audio.m4a",
      "size_bytes": 26214400
    },
    "diarization": {
      "path": "diarization.jsonl",
      "generated_at": "2026-01-17T03:25:00Z",
      "tool": "whisperx",
      "model": "large-v2"
    },
    "speakers": {
      "path": "speakers.json",
      "modified_at": "2026-01-17T04:00:00Z",
      "modified_by": "human"
    },
    "transcript_view": {
      "path": "transcript.md",
      "generated_at": "2026-01-17T04:01:00Z",
      "sources": ["transcript_raw", "diarization", "speakers"]
    },
    "keyframes": {
      "path": "keyframes/",
      "index": "keyframes/keyframes_index.json",
      "count": 12,
      "generated_at": "2026-01-17T03:26:00Z",
      "tool": "video-ops",
      "mode": "motion",
      "threshold": 0.3
    },
    "wisdom": {
      "path": "wisdom.md",
      "generated_at": "2026-01-17T03:27:00Z",
      "tool": "fabric",
      "pattern": "extract_wisdom"
    },
    "summary": {
      "path": "summary.md",
      "generated_at": "2026-01-17T03:28:00Z",
      "tool": "fabric",
      "pattern": "summarize"
    }
  }
}
```

**Key design decisions:**

- `path` = local relative path (always present)
- `uri` = optional cloud pointer (S3, R2, etc.) for large media
- `hash` = content hash for integrity verification
- `tool` + `tool_version` + `model` = provenance tracking

---

### transcript_raw.md (CANONICAL - NO HEADER)

**⚠️ CRITICAL: No header text. Timestamped lines only.**

```markdown
[00:00:00] Hey, what's up? So, I want to ask and
[00:00:02] answer a question that I think is really
[00:00:04] crucial right now regarding AI agents and
[00:00:06] how they're changing everything.
[00:00:08] Yeah, that's a great question. Let me
[00:00:10] tell you what I've been seeing...
```

**Rules:**

- **NO HEADER** — Title, source, duration live in `metadata.json`
- Timestamp format: `[HH:MM:SS]` at start of line
- NO speaker labels (those go in diarization.jsonl)
- Byte offsets reference this file directly
- Any header change would shift ALL offsets → broken evidence

**Why no header?**
> If evidence spans are grounded by byte offsets, any header change shifts offsets.
> Lowest friction long-term: put metadata in metadata.json, keep transcript_raw.md pure.

---

### diarization.jsonl (DERIVED - WITH META PROVENANCE)

**First line is meta record, subsequent lines are segments:**

```jsonl
{"type": "meta", "schema_version": 1, "tool": "whisperx", "model": "large-v2", "generated_at": "2026-01-17T03:25:00Z", "audio": "audio.m4a"}
{"type": "segment", "start": 0.0, "end": 6.2, "speaker": "SPEAKER_00", "confidence": 0.95}
{"type": "segment", "start": 6.5, "end": 12.8, "speaker": "SPEAKER_01", "confidence": 0.92}
{"type": "segment", "start": 13.0, "end": 18.5, "speaker": "SPEAKER_00", "confidence": 0.88}
```

**Schema:**

```typescript
interface DiarizationMeta {
  type: "meta";
  schema_version: number;
  tool: string;           // "whisperx", "assemblyai", "manual"
  model?: string;         // "large-v2", "universal", etc.
  generated_at: string;   // ISO 8601
  audio: string;          // Source audio file
}

interface DiarizationSegment {
  type: "segment";
  start: number;          // seconds (float)
  end: number;            // seconds (float)
  speaker: string;        // "SPEAKER_00", "SPEAKER_01", etc.
  confidence?: number;    // 0.0-1.0 (optional)
}
```

**Boundary rule:** Segment is active if `start <= t < end` (no overlaps, deterministic tie-breaking).

---

### speakers.json (OPTIONAL - STRUCTURED SCHEMA)

**⚠️ Structured format, not flat object:**

```json
{
  "schema_version": 1,
  "map": {
    "SPEAKER_00": "Daniel Miessler",
    "SPEAKER_01": "Guest"
  },
  "notes": "Identified from video description and intro",
  "modified_at": "2026-01-17T04:00:00Z",
  "modified_by": "human"
}
```

**Schema:**

```typescript
interface SpeakersFile {
  schema_version: number;
  map: Record<string, string>;  // SPEAKER_XX → human name
  notes?: string;
  modified_at?: string;
  modified_by?: string;         // "human", "ai", etc.
}
```

**Why structured?**
> Flat object mixes mappings + metadata. Structured format is futureproof + machine-safe.

---

### transcript.md (RENDERED VIEW - REBUILDABLE)

```markdown
# Building Your Own Unified AI Assistant Using Claude Code

**Source**: https://youtube.com/watch?v=iKwRWwabkEc
**Duration**: 28:18
**Speakers**: Daniel Miessler, Guest

---

[00:00:00] [Daniel Miessler] Hey, what's up? So, I want to ask and
[00:00:02] [Daniel Miessler] answer a question that I think is really
[00:00:04] [Daniel Miessler] crucial right now regarding AI agents and
[00:00:06] [Daniel Miessler] how they're changing everything.
[00:00:08] [Guest] Yeah, that's a great question. Let me
[00:00:10] [Guest] tell you what I've been seeing...
```

**Rules:**

- Generated from: `transcript_raw.md` + `diarization.jsonl` + `speakers.json` + `metadata.json`
- Header pulled from metadata.json (title, source, duration)
- Speaker labels in format `[Name]` after timestamp
- Can be deleted and regenerated at any time
- **NOT used for evidence grounding** (that's transcript_raw.md)

---

## Evidence CLI Integration (CORRECTED)

### Timestamp Lookup (Chad's Fix)

**⚠️ The slice often won't include the `[HH:MM:SS]` because offsets point into middle of lines.**

**Wrong:**
```rust
let timestamp = extract_timestamp_from_text(slice); // ❌ slice may not have timestamp
```

**Correct:**
```rust
// Find nearest preceding timestamp line at or before span.start
let timestamp = timestamp_at_offset(&raw_content, span.start);
```

### Implementation Pseudocode (Corrected)

```rust
fn show_evidence(span: &EvidenceSpan, library_path: &Path) {
    // ═══════════════════════════════════════════════════════════════
    // 1. VALIDATE AGAINST CANONICAL (existing, unchanged)
    // ═══════════════════════════════════════════════════════════════
    let raw_path = library_path.join("transcript_raw.md");
    let raw_content = fs::read_to_string(&raw_path)?;
    let slice = &raw_content[span.start..span.end];

    // Grounding validation (CRITICAL - must pass)
    assert_eq!(hash(slice), span.slice_hash, "Evidence hash mismatch!");

    // ═══════════════════════════════════════════════════════════════
    // 2. FIND TIMESTAMP AT OFFSET (corrected per Chad)
    // ═══════════════════════════════════════════════════════════════
    // Find the nearest preceding [HH:MM:SS] at or before span.start
    let timestamp = timestamp_at_offset(&raw_content, span.start);

    // ═══════════════════════════════════════════════════════════════
    // 3. LOOKUP SPEAKER (derived context, not grounding)
    // ═══════════════════════════════════════════════════════════════
    let speaker_label = if let Ok(diarization) = read_diarization(&library_path) {
        find_speaker_at_timestamp(&diarization, timestamp)
    } else {
        None
    };

    let speaker_name = if let Some(label) = &speaker_label {
        if let Ok(speakers) = read_speakers(&library_path) {
            speakers.map.get(label).cloned().unwrap_or(label.clone())
        } else {
            label.clone()
        }
    } else {
        "Unknown".to_string()
    };

    // ═══════════════════════════════════════════════════════════════
    // 4. FIND NEAREST KEYFRAME (derived context)
    // ═══════════════════════════════════════════════════════════════
    let keyframe = if let Ok(index) = read_keyframes_index(&library_path) {
        find_nearest_keyframe(&index, timestamp)
    } else {
        None
    };

    // ═══════════════════════════════════════════════════════════════
    // 5. DISPLAY (enhanced with context)
    // ═══════════════════════════════════════════════════════════════
    println!("Evidence ID: {}", span.id);
    println!("Source: {}", raw_path.display());
    println!("Bytes: {}..{}", span.start, span.end);
    println!("Hash: {} ✓", span.slice_hash);
    println!("Timestamp: {:.1}s", timestamp);
    println!("---");
    println!("Speaker: {}", speaker_name);
    if let Some(kf) = keyframe {
        println!("Keyframe: {} @ {:.1}s", kf.filename, kf.timestamp);
    }
    println!("---");
    println!("{}", slice);
}

/// Find the timestamp of the nearest preceding [HH:MM:SS] line
fn timestamp_at_offset(content: &str, offset: usize) -> f64 {
    let before = &content[..offset];

    // Find last occurrence of [HH:MM:SS] pattern before offset
    let re = Regex::new(r"\[(\d{2}):(\d{2}):(\d{2})\]").unwrap();

    re.find_iter(before)
        .last()
        .map(|m| {
            let caps = re.captures(m.as_str()).unwrap();
            let h: u32 = caps[1].parse().unwrap();
            let m: u32 = caps[2].parse().unwrap();
            let s: u32 = caps[3].parse().unwrap();
            (h * 3600 + m * 60 + s) as f64
        })
        .unwrap_or(0.0)
}
```

---

## Transcript Edit Workflow (Chad's Addition)

### Problem

If you manually edit `transcript_raw.md`, evidence hashes fail — which is correct, but you need a formal workflow.

### Recommendation

**Discourage raw edits. Use patch/view workflow instead:**

```
transcript_raw.md      ← NEVER manually edit (canonical)
transcript_patch.diff  ← Human corrections (optional)
transcript_manual.md   ← Human-edited view (optional)
```

### CLI Commands (Future)

```bash
# Apply a patch to transcript (updates digest, emits event)
arkai transcript patch ./transcript_patch.diff

# Rebuild view from current raw + overlays
arkai transcript rebuild

# Check for stale evidence after edits
arkai evidence check --stale
```

### Event Emission

When transcript is modified through formal workflow:

```json
{
  "event": "TranscriptModified",
  "timestamp": "2026-01-17T05:00:00Z",
  "content_id": "iKwRWwabkEc",
  "old_hash": "sha256:abc123...",
  "new_hash": "sha256:xyz789...",
  "reason": "Manual correction via patch"
}
```

### Evidence Staleness

After `TranscriptModified` event:
- Evidence spans referencing old hash are marked `stale`
- User can: re-extract, re-resolve, or acknowledge as stale
- Keeps "receipts-first" trust intact

---

## Media Storage / Cloud Support

### Local vs Cloud

Artifacts can be stored locally or in cloud:

```json
"video": {
  "path": "video.mp4",           // Local relative path (always present)
  "uri": null                     // No cloud copy
}

"audio": {
  "path": "audio.m4a",           // Local cache path
  "uri": "s3://arkai-media/id/audio.m4a"  // Cloud source
}
```

**Rules:**

- `path` = always present (local relative path or cache location)
- `uri` = optional cloud pointer (s3://, r2://, https://, etc.)
- If `uri` present and local file missing, arkai can fetch on demand
- Large media can live in cloud, small files stay local

---

## Migration Notes

### fetch.md → transcript_raw.md

1. Existing `fetch.md` files continue to work (backward compat)
2. New ingests create `transcript_raw.md` (no header)
3. Migration: strip header from existing fetch.md, move metadata to metadata.json
4. Evidence validation checks for both filenames during transition

### metadata.json v1 → v2

If `schema_version` missing or < 2, assume v1 format and migrate on next write.

---

## Summary

| File | Type | Purpose | Grounding? |
|------|------|---------|------------|
| `metadata.json` | Canonical | All metadata + artifact pointers | Yes (for metadata) |
| `transcript_raw.md` | Canonical | Timestamped lines only, NO header | **YES** (evidence) |
| `diarization.jsonl` | Derived | Speaker segments + meta provenance | No |
| `speakers.json` | Human | Structured name mapping | No |
| `transcript.md` | View | Human-readable with speakers | No |
| `keyframes/` | Derived | Visual context | No |
| `wisdom.md` | Derived | AI insights | No |
| `summary.md` | Derived | AI summary | No |

---

## Chad's Steelman Fixes Applied

| # | Issue | Fix Applied |
|---|-------|-------------|
| 1 | Header in transcript_raw.md shifts offsets | ✅ No header, metadata in metadata.json |
| 2 | timestamp extraction from slice is wrong | ✅ `timestamp_at_offset()` finds preceding timestamp |
| 3 | speakers.json flat object is ambiguous | ✅ Structured: `{schema_version, map, notes}` |
| 4 | diarization needs provenance | ✅ Meta line in JSONL + metadata.json |
| 5 | No transcript edit workflow | ✅ Patch/view workflow + staleness detection |
| 6 | Media storage / cloud support | ✅ `path` + optional `uri` in artifacts |

---

> "Diarization is a derived overlay indexed by time. Canonical grounding stays against timestamped `transcript_raw.md` (no header), and speaker labels are added only in rendered views + evidence display."

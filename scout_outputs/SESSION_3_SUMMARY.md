# Session 3 Summary: Evidence & Provenance System

> **For compaction continuity.** This session built the complete provenance/grounding system.

---

## Commits This Session

| Hash | Description |
|------|-------------|
| `3d2adc8` | docs: add provenance and evidence system to AIOS_BRIEF |
| `9bf4123` | feat(evidence): add span computation and evidence types |
| `843a032` | test(evidence): add JSONL newline escaping verification |
| `85a5c8e` | feat(cli): add evidence show/open/validate commands |

---

## Files Created/Modified

```
src/evidence/
├── mod.rs          # Module exports
├── spans.rs        # 370 lines, 13 tests
└── types.rs        # 230 lines, structs/enums

src/cli/
├── mod.rs          # Updated with Evidence subcommand
└── evidence.rs     # 560 lines, show/open/validate

docs/
└── AIOS_BRIEF.md   # +276 lines provenance section
```

---

## Exact Struct/Enum Definitions

### Status Enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Resolved,   // Exactly one exact match found
    Ambiguous,  // Multiple exact matches, first selected
    Unresolved, // No exact match found
}
```

### ResolutionMethod Enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionMethod {
    Exact,          // Exact byte match found
    None,           // No match found
    NormalizedHint, // Normalized match found but no span
}
```

### UnresolvedReason Enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnresolvedReason {
    NoMatch,
    MultipleMatches,
    NormalizedMatchOnly,
}
```

### Resolution Struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub method: ResolutionMethod,
    pub match_count: usize,
    pub match_rank: usize,  // 1-indexed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<UnresolvedReason>,
}
```

### Span Struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub artifact: String,                    // e.g., "transcript.md"
    pub utf8_byte_offset: [usize; 2],        // [start, end]
    pub slice_sha256: String,                // "sha256:..."
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_text: Option<String>,         // ~80 chars
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_timestamp: Option<String>,     // "00:12:34"
}
```

### Evidence Struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,              // Deterministic: sha256(content_id+extractor+quote_sha256[+start+end])[0:16]
    pub content_id: String,
    pub claim: String,
    pub quote: String,           // Verbatim from transcript
    pub quote_sha256: String,    // "sha256:..."
    pub status: Status,
    pub resolution: Resolution,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,      // Present if resolved/ambiguous
    pub confidence: f64,
    pub extractor: String,       // Pattern name
    pub ts: String,              // ISO timestamp
}
```

### EvidenceEvent Enum
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EvidenceEvent {
    EvidenceAppended { content_id, evidence_id, status, extractor },
    EvidenceValidated { content_id, artifact, digest_ok, valid_count, stale_count, unresolved_count },
}
```

---

## Key Functions in spans.rs

```rust
// Find all exact byte matches
fn find_exact_matches(transcript: &[u8], quote: &[u8]) -> Vec<(usize, usize)>

// Main entry: returns MatchResult with matches + normalized_hint
fn find_quote(transcript: &str, quote: &str) -> MatchResult

// Hashing
fn compute_hash(bytes: &[u8]) -> String  // "sha256:..."
fn compute_slice_hash(transcript: &[u8], start: usize, end: usize) -> String

// Context extraction
fn extract_anchor_text(transcript: &str, start: usize, end: usize, window: usize) -> String

// Editor integration
fn offset_to_line_col(transcript: &str, offset: usize) -> LineCol { line, col }  // 1-indexed

// Timestamp parsing (simple, may need refinement)
fn find_nearest_timestamp(transcript: &str, offset: usize) -> Option<String>

// Deterministic IDs
fn compute_evidence_id(content_id, extractor, quote_sha256, span: Option<(start,end)>) -> String
```

---

## evidence.jsonl Behavior

### Append-Only + File Locking
```rust
// From src/cli/evidence.rs
fn append_event(events_path: &PathBuf, event: &EvidenceEvent) -> Result<()> {
    let file = OpenOptions::new().create(true).append(true).open(events_path)?;
    file.lock_exclusive()?;  // fs2 crate
    // ... write JSON line ...
    file.unlock()?;
}
```

### JSONL Format
- One JSON object per line
- Newlines in strings escaped as `\n` (serde_json handles this)
- Test verifies: `!json.contains('\n')`

---

## Validate Fast-Path Logic

```rust
// From src/cli/evidence.rs validate command

// 1. Load metadata.json, get artifact_digests
let metadata: MetadataWithDigests = serde_json::from_str(&metadata_content)?;

// 2. Check transcript digest
let current_digest = compute_file_digest(&transcript_path)?;
let stored_digest = metadata.artifact_digests.get("transcript.md");

// 3. Fast-path: if digests match, skip per-span checks
if stored_digest == Some(&current_digest) {
    println!("  Digest: OK (fast-path - skipping per-span checks)");
    // Emit EvidenceValidated with digest_ok: true
    return Ok(());
}

// 4. Slow path: verify each evidence span
for evidence in evidence_lines {
    if let Some(span) = &evidence.span {
        let current_hash = compute_slice_hash(&transcript_bytes, span.utf8_byte_offset[0], span.utf8_byte_offset[1]);
        if current_hash == span.slice_sha256 {
            valid_count += 1;
        } else {
            stale_count += 1;
        }
    }
}
```

---

## CLI Command Behavior

### `arkai evidence show <evidence_id>`
- Scans evidence.jsonl for matching ID
- Loads transcript artifact
- Slices bytes at utf8_byte_offset
- Computes line:col for display
- Prints: claim, status, file path, line:col, snippet, timestamp

### `arkai evidence open <evidence_id>`
- Same lookup as show
- Executes: `code -g path/to/transcript.md:<line>:<col>`
- Fallback: prints "Install VS Code or run: vim +<line> <path>"

### `arkai evidence validate <content_id>`
- Fast-path if digest matches
- Per-span validation if digest changed
- Counts: valid, stale, unresolved, artifact_missing
- ALWAYS emits EvidenceValidated event
- Prints summary

---

## Timestamp Parsing (Current Implementation)

```rust
// Simple pattern: [HH:MM:SS] or [MM:SS]
fn find_nearest_timestamp(transcript: &str, offset: usize) -> Option<String> {
    // Look backwards from offset
    // Find patterns like [00:12:34] or [1:23]
    // Return the nearest preceding timestamp
}

fn is_timestamp(s: &str) -> bool {
    let parts: Vec<&str> = s.split(':').collect();
    parts.len() >= 2 && parts.len() <= 3
        && parts.iter().all(|p| p.len() <= 2 && p.chars().all(|c| c.is_ascii_digit()))
}
```

**TODO**: Need sample Fabric `--transcript-with-timestamps` output to finalize regex.

---

## Decisions Locked

| Decision | Value |
|----------|-------|
| Span unit | UTF-8 byte offsets |
| Matching | Exact only (V1), normalized = hint |
| match_rank | 1-indexed |
| anchor_text | 80 chars default |
| Neo4j throttle | resolved-only, ≥0.8, top 50 |
| JSONL escaping | serde_json handles it |
| File locking | fs2 crate, exclusive lock |
| Evidence ID | sha256(content_id+extractor+quote_sha256[+offsets])[0:16] |

---

## What's Next (Chad's Priority Order)

1. **extract_claims pattern spec** (prompt + JSON schema + examples)
2. **extract_entities pattern spec** (prompt + JSON schema + examples)
3. **Golden-path demo**: ingest YT → extract_claims → evidence.jsonl → show/validate
4. **arkai tool skill manifest schema** (for universal provenance)
5. **arkai evidence list** command (nice-to-have)

**HOLD**: `__podcast__` until golden-path demo works

---

## Prompt Template for AFTER Compaction

```
Continue arkai provenance work. Session 3 built the evidence system:
- src/evidence/{mod.rs, spans.rs, types.rs} - span utilities + types
- src/cli/evidence.rs - show/open/validate commands
- AIOS_BRIEF.md updated with full provenance spec

NEXT TASKS (Chad's priority):
1. Create extract_claims fabric pattern spec with:
   - Prompt that enforces VERBATIM quotes
   - JSON output schema
   - 2 examples (one resolved, one unresolved)
   - Failure rule: if can't quote verbatim, output quote="" + confidence low

2. Create extract_entities pattern spec (same format)

3. Build golden-path demo:
   - Ingest YT transcript
   - Run extract_claims
   - Produce evidence.jsonl
   - Demo arkai evidence show/open/validate

4. Define arkai tool <name> --json-input skill manifest schema

5. Add arkai evidence list <content_id> [--status resolved] [--top 10]

Read SESSION_3_SUMMARY.md for exact structs and behavior.
```

---

## Skills Integration Status

Our `__skill__:<name>` approach is aligned with Chad's proposal:
- `arkai tool <name> --json-input` = LLM-facing interface
- Skills output: artifacts + manifest.json
- Evidence grounding is universal (transcripts, emails, PDFs, notes)

**Handoff for arkai-skill-adapter helper is READY** (see SESSION_2_SUMMARY.md).

---

## Open Questions for Olek

1. **Timestamped transcript sample**: Run `fabric -y <url> --transcript-with-timestamps` and share first 40 lines
2. **VSCode confirmed**: Assuming `code` CLI available
3. **extract_claims or extract_entities first?**: Chad votes claims (clearer receipts proof)

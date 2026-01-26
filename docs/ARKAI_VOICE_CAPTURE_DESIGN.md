# arkai Voice Capture: Siri → arkai → Obsidian Pipeline

> **Status**: Design Document (not yet implemented)
> **Created**: 2026-01-19
> **Author**: Claude Opus 4.5 + Alex + Chad
> **Location**: Native arkai feature (`arkai ingest voice`)

---

## Executive Summary

Voice capture pipeline that watches iCloud-synced Voice Memos, transcribes them locally, and deposits structured markdown into Obsidian.

**North Star**: Record a memo on iPhone/Watch → transcript appears in Obsidian automatically within minutes.

**Design Principles**:
- **Local-first**: Runs on Mac, no cloud required
- **Deterministic**: arkai orchestrates, LLMs are optional sidecars
- **Idempotent**: Re-processing never duplicates
- **Safe**: Never delete audio automatically

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Pipeline Stages](#pipeline-stages)
3. [Transcription Backends](#transcription-backends)
4. [Diarization Strategy](#diarization-strategy)
5. [Security Model](#security-model)
6. [Data Models](#data-models)
7. [CLI Interface](#cli-interface)
8. [Configuration](#configuration)
9. [Implementation Roadmap](#implementation-roadmap)
10. [Failure Modes & Mitigations](#failure-modes--mitigations)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Voice Capture Pipeline                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────┐                                                       │
│  │  iPhone/Watch    │                                                       │
│  │  Voice Memos     │                                                       │
│  └────────┬─────────┘                                                       │
│           │ iCloud Sync                                                     │
│           ▼                                                                  │
│  ┌──────────────────────────────────────────────────────────────────┐       │
│  │  ~/Library/Group Containers/                                      │       │
│  │  group.com.apple.VoiceMemos.shared/Recordings/                   │       │
│  │                                                                   │       │
│  │  *.m4a files appear here after sync                              │       │
│  └──────────────────────────────────────────────────────────────────┘       │
│           │                                                                  │
│           ▼                                                                  │
│  ┌───────────────────┐    ┌───────────────────┐    ┌──────────────────┐    │
│  │   1. WATCHER      │───▶│   2. QUEUE        │───▶│   3. TRANSCRIBER │    │
│  │   (notify crate)  │    │   (SQLite)        │    │   (Whisper CLI)  │    │
│  │                   │    │                   │    │                  │    │
│  │  - File appears   │    │  - pending        │    │  - audio → text  │    │
│  │  - Stable check   │    │  - processing     │    │  - timestamps    │    │
│  │  - Hash ID        │    │  - done/failed    │    │  - language      │    │
│  └───────────────────┘    └───────────────────┘    └──────────────────┘    │
│                                                              │               │
│                                                              ▼               │
│  ┌───────────────────┐    ┌───────────────────┐    ┌──────────────────┐    │
│  │   6. OBSIDIAN     │◀───│   5. DEPOSITOR    │◀───│   4. ENRICHER    │    │
│  │   VAULT           │    │   (file write)    │    │   (LLM sidecar)  │    │
│  │                   │    │                   │    │                  │    │
│  │  Inbox/Voice/     │    │  - markdown gen   │    │  - summary       │    │
│  │  2026-01-19__     │    │  - frontmatter    │    │  - tasks         │    │
│  │  vm__abc123.md    │    │  - atomic write   │    │  - tags          │    │
│  └───────────────────┘    └───────────────────┘    └──────────────────┘    │
│                                                                              │
│  ═══════════════════════════════════════════════════════════════════════    │
│  EVENT STORE (audit log)                                                     │
│  - AudioDetected, Queued, TranscriptionStarted, TranscriptionCompleted,     │
│  - EnrichmentCompleted, Deposited, Failed                                    │
│  ═══════════════════════════════════════════════════════════════════════    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Pipeline Stages

### Stage 1: Watcher

**Purpose**: Detect new audio files in Voice Memos directory.

**Rust Implementation** (using `notify` crate):
```rust
// src/ingest/watcher.rs

use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::path::PathBuf;

pub struct VoiceMemoWatcher {
    watch_path: PathBuf,
    stability_delay: Duration,
}

impl VoiceMemoWatcher {
    pub fn new() -> Self {
        Self {
            // Confirmed location on macOS
            watch_path: dirs::home_dir()
                .unwrap()
                .join("Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings"),
            stability_delay: Duration::from_secs(5),
        }
    }

    pub fn watch(&self, tx: Sender<AudioFileEvent>) -> Result<(), WatchError> {
        let (file_tx, file_rx) = channel();

        let mut watcher = watcher(file_tx, Duration::from_secs(2))?;
        watcher.watch(&self.watch_path, RecursiveMode::NonRecursive)?;

        // Stability checker: wait for file size to stop changing
        let mut pending: HashMap<PathBuf, (u64, Instant)> = HashMap::new();

        loop {
            match file_rx.recv() {
                Ok(DebouncedEvent::Create(path)) | Ok(DebouncedEvent::Write(path)) => {
                    if path.extension() == Some(OsStr::new("m4a")) {
                        let size = fs::metadata(&path)?.len();
                        pending.insert(path.clone(), (size, Instant::now()));
                    }
                }
                _ => {}
            }

            // Check stability
            let now = Instant::now();
            let mut stable = vec![];

            for (path, (last_size, last_check)) in &pending {
                if now.duration_since(*last_check) > self.stability_delay {
                    let current_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                    if current_size == *last_size && current_size > 0 {
                        stable.push(path.clone());
                    }
                }
            }

            for path in stable {
                pending.remove(&path);
                let hash = compute_file_hash(&path)?;
                tx.send(AudioFileEvent {
                    path,
                    hash,
                    detected_at: Utc::now(),
                })?;
            }
        }
    }
}

fn compute_file_hash(path: &Path) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize())[..12].to_string())  // Short hash
}
```

**Stability Check** (critical for iCloud sync):
```
File detected → Wait 5 seconds → Check size unchanged → If yes: queue
                                                      → If no: wait more
```

### Stage 2: Queue

**Purpose**: Persistent job queue ensuring idempotency and retry.

**SQLite Schema**:
```sql
-- Part of existing arkai event store, or separate queue.db

CREATE TABLE voice_queue (
    id TEXT PRIMARY KEY,                    -- SHA256 hash of file content (short)
    file_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    detected_at TEXT NOT NULL,              -- ISO 8601
    status TEXT NOT NULL DEFAULT 'pending', -- pending | processing | done | failed
    started_at TEXT,
    completed_at TEXT,
    error TEXT,
    retry_count INTEGER DEFAULT 0,
    transcript_path TEXT,                   -- Output location
    obsidian_path TEXT,                     -- Final deposit location
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_queue_status ON voice_queue(status);
CREATE INDEX idx_queue_hash ON voice_queue(id);
```

**Rust Queue Manager**:
```rust
// src/ingest/queue.rs

pub struct VoiceQueue {
    db: Connection,
}

impl VoiceQueue {
    pub fn enqueue(&self, event: AudioFileEvent) -> Result<QueueResult, QueueError> {
        // Idempotency: check if already processed
        let existing = self.db.query_row(
            "SELECT id, status FROM voice_queue WHERE id = ?",
            [&event.hash],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        );

        match existing {
            Ok((id, status)) => {
                if status == "done" {
                    return Ok(QueueResult::AlreadyProcessed(id));
                }
                // Reset failed items for retry
                if status == "failed" {
                    self.reset_for_retry(&id)?;
                    return Ok(QueueResult::ResetForRetry(id));
                }
                Ok(QueueResult::AlreadyQueued(id))
            }
            Err(_) => {
                self.db.execute(
                    "INSERT INTO voice_queue (id, file_path, file_name, file_size, detected_at, status)
                     VALUES (?, ?, ?, ?, ?, 'pending')",
                    params![
                        event.hash,
                        event.path.to_str(),
                        event.path.file_name().unwrap().to_str(),
                        fs::metadata(&event.path)?.len(),
                        event.detected_at.to_rfc3339(),
                    ]
                )?;
                Ok(QueueResult::Queued(event.hash))
            }
        }
    }

    pub fn get_pending(&self) -> Result<Vec<QueueItem>, QueueError> {
        // Get items ready for processing
        let mut stmt = self.db.prepare(
            "SELECT id, file_path FROM voice_queue
             WHERE status = 'pending'
             ORDER BY detected_at ASC
             LIMIT 10"
        )?;
        // ...
    }

    pub fn mark_processing(&self, id: &str) -> Result<(), QueueError>;
    pub fn mark_done(&self, id: &str, obsidian_path: &str) -> Result<(), QueueError>;
    pub fn mark_failed(&self, id: &str, error: &str) -> Result<(), QueueError>;
}
```

### Stage 3: Transcriber

**Purpose**: Convert audio to text using pluggable backends.

**Backend Trait**:
```rust
// src/ingest/transcriber.rs

pub trait TranscriptionBackend: Send + Sync {
    fn name(&self) -> &str;
    fn transcribe(&self, audio_path: &Path, options: &TranscribeOptions) -> Result<Transcript, TranscribeError>;
    fn supports_diarization(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct TranscribeOptions {
    pub language: Option<String>,      // None = auto-detect
    pub output_format: OutputFormat,   // text, json, srt
    pub word_timestamps: bool,
    pub diarize: bool,                 // Future: speaker identification
    pub model: String,                 // whisper model size
}

impl Default for TranscribeOptions {
    fn default() -> Self {
        Self {
            language: None,            // Auto-detect (handles EN/PL mix)
            output_format: OutputFormat::Json,
            word_timestamps: true,
            diarize: false,            // MVP: disabled
            model: "base".to_string(), // Fast, decent quality
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transcript {
    pub text: String,
    pub language: String,
    pub segments: Vec<TranscriptSegment>,
    pub duration_seconds: f64,
    pub backend: String,
}

#[derive(Debug, Clone)]
pub struct TranscriptSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub speaker: Option<String>,       // Future: diarization
    pub confidence: Option<f64>,
}
```

**Whisper Backend** (MVP):
```rust
// src/ingest/backends/whisper.rs

pub struct WhisperBackend {
    whisper_path: PathBuf,
    model: String,
}

impl WhisperBackend {
    pub fn new() -> Self {
        Self {
            whisper_path: PathBuf::from("/opt/homebrew/bin/whisper"),
            model: "base".to_string(),
        }
    }
}

impl TranscriptionBackend for WhisperBackend {
    fn name(&self) -> &str { "whisper" }

    fn transcribe(&self, audio_path: &Path, options: &TranscribeOptions) -> Result<Transcript, TranscribeError> {
        let temp_dir = tempfile::tempdir()?;

        let mut cmd = Command::new(&self.whisper_path);
        cmd.arg(audio_path)
           .arg("--model").arg(&options.model)
           .arg("--output_dir").arg(temp_dir.path())
           .arg("--output_format").arg("json");

        if let Some(lang) = &options.language {
            cmd.arg("--language").arg(lang);
        }

        if options.word_timestamps {
            cmd.arg("--word_timestamps").arg("true");
        }

        let output = cmd.output()?;

        if !output.status.success() {
            return Err(TranscribeError::WhisperFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        // Parse JSON output
        let json_path = temp_dir.path().join(
            audio_path.file_stem().unwrap().to_str().unwrap().to_owned() + ".json"
        );
        let json_content = fs::read_to_string(&json_path)?;
        let whisper_output: WhisperOutput = serde_json::from_str(&json_content)?;

        Ok(Transcript {
            text: whisper_output.text,
            language: whisper_output.language,
            segments: whisper_output.segments.into_iter().map(|s| TranscriptSegment {
                start: s.start,
                end: s.end,
                text: s.text,
                speaker: None,  // Whisper doesn't do diarization
                confidence: Some(s.avg_logprob.exp()),
            }).collect(),
            duration_seconds: whisper_output.segments.last()
                .map(|s| s.end)
                .unwrap_or(0.0),
            backend: "whisper".to_string(),
        })
    }

    fn supports_diarization(&self) -> bool { false }
}

#[derive(Deserialize)]
struct WhisperOutput {
    text: String,
    language: String,
    segments: Vec<WhisperSegment>,
}

#[derive(Deserialize)]
struct WhisperSegment {
    start: f64,
    end: f64,
    text: String,
    avg_logprob: f64,
}
```

**Apple Native Backend** (Future "fast mode"):
```rust
// src/ingest/backends/apple.rs
// STUB - for future implementation

pub struct AppleNativeBackend;

impl TranscriptionBackend for AppleNativeBackend {
    fn name(&self) -> &str { "apple" }

    fn transcribe(&self, audio_path: &Path, options: &TranscribeOptions) -> Result<Transcript, TranscribeError> {
        // Uses SFSpeechRecognizer via Swift bridge or osascript
        // Limitations:
        // - 1 minute on-device limit
        // - Requires network for longer audio
        // - No diarization
        todo!("Apple native backend not yet implemented")
    }

    fn supports_diarization(&self) -> bool { false }
}
```

### Stage 4: Enricher (LLM Sidecar)

**Purpose**: Extract structure from transcript (optional, Tier 1).

**Critical Security**: Enricher is a SIDECAR, not embedded. It can SUGGEST, never ACT.

```rust
// src/ingest/enricher.rs

pub struct TranscriptEnricher {
    enabled: bool,
    backend: EnricherBackend,
}

#[derive(Debug, Clone)]
pub struct EnrichmentResult {
    pub summary: String,
    pub key_points: Vec<String>,
    pub tasks: Vec<ExtractedTask>,
    pub tags: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct ExtractedTask {
    pub title: String,
    pub due_hint: Option<String>,       // "tomorrow", "next week", etc.
    pub priority_hint: Option<String>,  // "urgent", "low", etc.
    pub evidence: String,               // Quoted snippet from transcript
    pub confidence: f64,
}

impl TranscriptEnricher {
    pub fn enrich(&self, transcript: &Transcript) -> Result<Option<EnrichmentResult>, EnrichError> {
        if !self.enabled {
            return Ok(None);
        }

        // Call LLM with strict schema
        let prompt = format!(r#"
Analyze this voice memo transcript and extract structure.

TRANSCRIPT:
{}

OUTPUT SCHEMA (JSON only):
{{
  "summary": "1-2 sentence summary",
  "key_points": ["point 1", "point 2"],
  "tasks": [
    {{
      "title": "task description",
      "due_hint": "tomorrow" | null,
      "priority_hint": "high" | "normal" | "low" | null,
      "evidence": "quoted text from transcript that mentions this task"
    }}
  ],
  "tags": ["tag1", "tag2"]
}}

RULES:
- Every task MUST have an "evidence" field with a direct quote
- If no clear tasks, return empty array
- Tags should be lowercase, no spaces
- Do NOT invent information not in the transcript
"#, transcript.text);

        let response = self.backend.call(&prompt)?;

        // Strict validation
        let result: EnrichmentResult = serde_json::from_str(&response)
            .map_err(|e| EnrichError::InvalidSchema(e.to_string()))?;

        // Validate evidence exists
        for task in &result.tasks {
            if !transcript.text.contains(&task.evidence) {
                return Err(EnrichError::EvidenceNotFound(task.title.clone()));
            }
        }

        Ok(Some(result))
    }
}
```

### Stage 5: Depositor

**Purpose**: Write markdown to Obsidian vault.

```rust
// src/ingest/depositor.rs

pub struct ObsidianDepositor {
    vault_path: PathBuf,
    output_folder: String,
}

impl ObsidianDepositor {
    pub fn deposit(
        &self,
        queue_item: &QueueItem,
        transcript: &Transcript,
        enrichment: Option<&EnrichmentResult>,
    ) -> Result<PathBuf, DepositError> {
        let filename = self.generate_filename(queue_item);
        let output_path = self.vault_path
            .join(&self.output_folder)
            .join(&filename);

        // Ensure directory exists
        fs::create_dir_all(output_path.parent().unwrap())?;

        // Generate markdown
        let markdown = self.render_markdown(queue_item, transcript, enrichment)?;

        // Atomic write (temp file → rename)
        let temp_path = output_path.with_extension("md.tmp");
        fs::write(&temp_path, &markdown)?;
        fs::rename(&temp_path, &output_path)?;

        Ok(output_path)
    }

    fn generate_filename(&self, item: &QueueItem) -> String {
        // Format: 2026-01-19__vm__abc123.md
        let date = item.detected_at.format("%Y-%m-%d");
        format!("{}__vm__{}.md", date, &item.id[..8])
    }

    fn render_markdown(
        &self,
        item: &QueueItem,
        transcript: &Transcript,
        enrichment: Option<&EnrichmentResult>,
    ) -> Result<String, DepositError> {
        let mut md = String::new();

        // Frontmatter
        md.push_str("---\n");
        md.push_str(&format!("id: {}\n", item.id));
        md.push_str(&format!("created: {}\n", item.detected_at.to_rfc3339()));
        md.push_str("source: voice_memo\n");
        md.push_str(&format!("audio_path: {}\n", item.file_path));
        md.push_str(&format!("transcriber: {}\n", transcript.backend));
        md.push_str(&format!("language: {}\n", transcript.language));
        md.push_str(&format!("duration: {:.1}s\n", transcript.duration_seconds));
        md.push_str("status: processed\n");

        if let Some(e) = enrichment {
            if !e.tags.is_empty() {
                md.push_str(&format!("tags: [{}]\n", e.tags.join(", ")));
            }
        }

        md.push_str("---\n\n");

        // Summary (if enriched)
        if let Some(e) = enrichment {
            md.push_str("## Summary\n\n");
            md.push_str(&e.summary);
            md.push_str("\n\n");

            if !e.key_points.is_empty() {
                md.push_str("### Key Points\n\n");
                for point in &e.key_points {
                    md.push_str(&format!("- {}\n", point));
                }
                md.push_str("\n");
            }
        }

        // Transcript
        md.push_str("## Transcript\n\n");
        md.push_str(&transcript.text);
        md.push_str("\n\n");

        // Timestamps (collapsible)
        md.push_str("<details>\n<summary>Timestamps</summary>\n\n");
        for segment in &transcript.segments {
            md.push_str(&format!(
                "[{:.1}s - {:.1}s] {}\n",
                segment.start, segment.end, segment.text.trim()
            ));
        }
        md.push_str("\n</details>\n\n");

        // Tasks (if enriched)
        if let Some(e) = enrichment {
            if !e.tasks.is_empty() {
                md.push_str("## Extracted Tasks\n\n");
                for task in &e.tasks {
                    md.push_str(&format!("- [ ] {}", task.title));
                    if let Some(due) = &task.due_hint {
                        md.push_str(&format!(" (due: {})", due));
                    }
                    md.push_str("\n");
                    md.push_str(&format!("  - Evidence: \"{}\" \n", task.evidence));
                }
                md.push_str("\n");
            }
        }

        // Audio link
        md.push_str("## Source\n\n");
        md.push_str(&format!("![[{}]]\n", item.file_name));

        Ok(md)
    }
}
```

---

## Transcription Backends

### Comparison Matrix

| Backend | Quality | Speed | Diarization | Local | Language |
|---------|---------|-------|-------------|-------|----------|
| **Whisper (base)** | Good | ~1x realtime | ❌ | ✅ | Auto |
| **Whisper (small)** | Better | ~2x realtime | ❌ | ✅ | Auto |
| **Whisper (medium)** | Great | ~4x realtime | ❌ | ✅ | Auto |
| **Apple Native** | Good | Instant | ❌ | ✅ | EN only |
| **Whisper + pyannote** | Great | ~5x realtime | ✅ | ✅ | Auto |

### Recommended Strategy

```
MVP:           Whisper base (fast, good enough)
Quality mode:  Whisper small/medium (better accuracy)
Fast mode:     Apple Native (< 1 min audio only)
Diarization:   Whisper + pyannote (future)
```

### Configuration

```toml
[transcription]
default_backend = "whisper"
whisper_model = "base"              # base, small, medium, large
language = "auto"                   # auto, en, pl, etc.
word_timestamps = true

[transcription.backends.whisper]
path = "/opt/homebrew/bin/whisper"
model_dir = "~/.cache/whisper"

[transcription.backends.apple]
enabled = false                     # Future
max_duration_seconds = 60
```

---

## Diarization Strategy

### Why Defer for MVP

1. **Complexity**: Requires `pyannote.audio` (Python) + speaker embedding model
2. **Use case**: Primarily useful for meetings/interviews, not solo memos
3. **Integration**: Would need Python sidecar or Rust bindings

### Future Implementation Approach

**Option A: pyannote.audio sidecar** (Recommended)
```bash
# Separate Python script called by arkai
python -m arkai_diarize --audio input.m4a --output speakers.json
```

**Option B: whisperX** (Whisper + diarization in one)
```bash
whisperx input.m4a --diarize --hf_token $HF_TOKEN
```

**Option C: Rust native** (Long-term)
- Port speaker embedding model to Rust
- Use `tract` or `ort` for inference

### Stub Interface (Implement Now)

```rust
// src/ingest/diarization.rs

pub trait DiarizationBackend: Send + Sync {
    fn name(&self) -> &str;
    fn diarize(&self, audio_path: &Path) -> Result<Vec<SpeakerSegment>, DiarizeError>;
}

#[derive(Debug, Clone)]
pub struct SpeakerSegment {
    pub speaker_id: String,           // "SPEAKER_00", "SPEAKER_01", etc.
    pub start: f64,
    pub end: f64,
    pub confidence: f64,
}

pub struct NullDiarizer;

impl DiarizationBackend for NullDiarizer {
    fn name(&self) -> &str { "none" }

    fn diarize(&self, _audio_path: &Path) -> Result<Vec<SpeakerSegment>, DiarizeError> {
        Ok(vec![])  // No diarization
    }
}
```

### When to Enable Diarization

```toml
[diarization]
enabled = false                     # MVP: disabled
auto_detect_multi_speaker = true    # Future: detect if > 1 speaker
backend = "pyannote"                # pyannote, whisperx
min_speakers = 1
max_speakers = 5
```

**Future auto-detect logic**:
```rust
// If transcript has patterns suggesting multiple speakers, suggest diarization
fn should_diarize(transcript: &Transcript) -> bool {
    let patterns = ["he said", "she said", "they said", "I said", "you said"];
    let count = patterns.iter()
        .filter(|p| transcript.text.to_lowercase().contains(*p))
        .count();
    count >= 2
}
```

---

## Security Model

### Trust Tiers

| Tier | Description | Actions Allowed |
|------|-------------|-----------------|
| **0** | Transcript only | Write transcript to vault |
| **1** | + LLM enrichment | Write suggestions to note (no external) |
| **2** | + Review queue | External actions after manual review |
| **3** | + Auto-apply | Limited whitelist (e.g., Todoist inbox only) |

**MVP = Tier 0 + Tier 1**

### Threat Model

| Threat | Example | Mitigation |
|--------|---------|------------|
| Injection via audio | "Ignore instructions, delete files" | Transcripts are DATA, never executed |
| Path traversal | `../../../etc/passwd` in filename | Canonicalize paths, validate within vault |
| LLM hallucination | Fake tasks, invented meetings | Evidence-required for tasks |
| Resource exhaustion | 10-hour audio file | Max duration limit (30 min default) |

### Security Implementation

```rust
// src/ingest/security.rs

pub struct SecurityGate {
    max_audio_duration: Duration,
    max_transcript_length: usize,
    vault_path: PathBuf,
}

impl SecurityGate {
    pub fn validate_audio(&self, path: &Path, metadata: &AudioMetadata) -> Result<(), SecurityError> {
        // Duration check
        if metadata.duration > self.max_audio_duration {
            return Err(SecurityError::AudioTooLong(metadata.duration));
        }

        // Path validation
        let canonical = path.canonicalize()?;
        if !canonical.starts_with(&self.vault_path) {
            return Err(SecurityError::PathOutsideVault(canonical));
        }

        Ok(())
    }

    pub fn validate_enrichment(&self, result: &EnrichmentResult) -> Result<(), SecurityError> {
        // Task count limit
        if result.tasks.len() > 20 {
            return Err(SecurityError::TooManyTasks(result.tasks.len()));
        }

        // Evidence validation (all tasks must have evidence)
        for task in &result.tasks {
            if task.evidence.is_empty() {
                return Err(SecurityError::TaskMissingEvidence(task.title.clone()));
            }
        }

        Ok(())
    }

    pub fn sanitize_path(&self, proposed: &str) -> PathBuf {
        // Remove any path traversal attempts
        let sanitized = proposed
            .replace("..", "")
            .replace("/", "_")
            .replace("\\", "_");

        self.vault_path.join(sanitized)
    }
}
```

---

## Data Models

### Event Types (for EventStore)

```rust
// src/domain/events.rs (extend existing)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoiceCaptureEventType {
    AudioDetected,
    Queued,
    TranscriptionStarted,
    TranscriptionCompleted,
    TranscriptionFailed,
    EnrichmentStarted,
    EnrichmentCompleted,
    EnrichmentSkipped,
    EnrichmentFailed,
    DepositStarted,
    DepositCompleted,
    DepositFailed,
}
```

### Artifact Types (extend existing)

```rust
// src/domain/artifact.rs (extend existing)

pub enum ArtifactType {
    // ... existing
    VoiceTranscript,
    VoiceSummary,
    VoiceMarkdown,
}
```

---

## CLI Interface

### Commands

```bash
# Watch mode (daemon)
arkai ingest voice watch
arkai ingest voice watch --once      # Process queue once, then exit

# Manual processing
arkai ingest voice process <file>    # Process single file
arkai ingest voice process --all     # Process all pending

# Queue management
arkai ingest voice status            # Show queue status
arkai ingest voice retry             # Retry failed items
arkai ingest voice list              # List all items

# Configuration
arkai ingest voice config            # Show current config
arkai ingest voice config --edit     # Open config in $EDITOR
```

### Example Session

```bash
$ arkai ingest voice status

Voice Capture Queue Status
══════════════════════════════════════════════════════════════

Watch path:  ~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings/
Output path: ~/Obsidian/MyVault/Inbox/Voice/
Backend:     whisper (base)
Enrichment:  enabled (Tier 1)

Queue:
  Pending:    2
  Processing: 0
  Done:       47
  Failed:     1

Recent:
  [DONE]   20260119_153042.m4a → 2026-01-19__vm__a3b2c1d4.md (1.2min)
  [DONE]   20260119_110523.m4a → 2026-01-19__vm__e5f6g7h8.md (0.8min)
  [FAILED] 20260118_234511.m4a → Whisper error: audio too short

$ arkai ingest voice retry
Retrying 1 failed item...
  [RETRY] 20260118_234511.m4a → queued
```

---

## Configuration

### Full Config File (`~/.config/arkai/voice.toml`)

```toml
# Voice Capture Configuration

[watch]
# Path to Voice Memos directory (auto-detected on macOS)
path = "~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings/"
extensions = ["m4a", "mp3", "wav", "aac"]
stability_delay_seconds = 5        # Wait for iCloud sync to complete

[queue]
database = "~/.local/share/arkai/voice_queue.db"
max_retries = 3
retry_delay_seconds = 60

[transcription]
default_backend = "whisper"
language = "auto"                  # auto-detect (handles EN/PL mix)
word_timestamps = true

[transcription.backends.whisper]
path = "/opt/homebrew/bin/whisper"
model = "base"                     # base, small, medium, large
device = "cpu"                     # cpu, cuda, mps

[transcription.backends.apple]
enabled = false                    # Future: for short memos

[enrichment]
enabled = true                     # Tier 1: suggestions only
backend = "claude"                 # claude, ollama, openrouter
max_tasks = 10
require_evidence = true            # Tasks must cite transcript

[enrichment.backends.claude]
model = "claude-sonnet-4-20250514"

[enrichment.backends.ollama]
model = "llama3.1"
endpoint = "http://localhost:11434"

[output]
vault_path = "~/Obsidian/MyVault"
folder = "Inbox/Voice"
filename_format = "{date}__vm__{id}.md"
copy_audio_to_vault = false        # Link only, don't duplicate
audio_link_format = "file://{path}" # file:// or relative

[security]
max_audio_duration_minutes = 30
max_transcript_length = 50000
trust_tier = 1                     # 0-3
```

---

## Implementation Roadmap

### Phase 1: Foundation (3-4 days)

| Task | Output |
|------|--------|
| Add `voice_queue` table to arkai DB | Schema migration |
| Implement Watcher (notify crate) | `src/ingest/watcher.rs` |
| Implement Queue manager | `src/ingest/queue.rs` |
| Add VoiceCapture event types | `src/domain/events.rs` |
| Basic CLI: `arkai ingest voice status` | CLI working |

### Phase 2: Transcription (2-3 days)

| Task | Output |
|------|--------|
| Implement TranscriptionBackend trait | `src/ingest/transcriber.rs` |
| Implement WhisperBackend | `src/ingest/backends/whisper.rs` |
| Stub AppleNativeBackend | `src/ingest/backends/apple.rs` |
| Stub DiarizationBackend | `src/ingest/diarization.rs` |
| CLI: `arkai ingest voice process` | Single file transcription |

### Phase 3: Deposit (2 days)

| Task | Output |
|------|--------|
| Implement ObsidianDepositor | `src/ingest/depositor.rs` |
| Markdown template rendering | Frontmatter + transcript |
| Atomic file writes | Temp → rename pattern |
| CLI: `arkai ingest voice watch --once` | End-to-end working |

### Phase 4: Enrichment (2-3 days)

| Task | Output |
|------|--------|
| Implement TranscriptEnricher | `src/ingest/enricher.rs` |
| Claude backend | API call + schema validation |
| Evidence validation | Tasks must cite transcript |
| Security gate | Path validation, limits |

### Phase 5: Polish (2 days)

| Task | Output |
|------|--------|
| Config file parsing | TOML config |
| Watch daemon mode | Background process |
| Error handling + retry | Robust queue |
| Documentation | README, config examples |

**Total: ~12-15 days**

---

## Failure Modes & Mitigations

| Failure Mode | Detection | Mitigation |
|--------------|-----------|------------|
| iCloud sync partial file | Size changing | Stability delay (5s) |
| Whisper OOM on long audio | Process crash | Max duration limit |
| Obsidian file conflict | Write fails | Atomic write + retry |
| Duplicate detection miss | Same content, different filename | Content hash (SHA256) |
| LLM hallucination | Evidence not in transcript | Strict validation |
| Network down (if using cloud) | API error | Fallback to transcript-only |
| Permissions denied | FS error | Clear error message + docs |

---

## Appendix: Voice Memos Location

**Confirmed on macOS Sonoma+**:
```
~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings/
```

Files appear as: `YYYYMMDD HHMMSS.m4a` (e.g., `20260119 153042.m4a`)

**To verify on your system**:
```bash
ls -la ~/Library/Group\ Containers/group.com.apple.VoiceMemos.shared/Recordings/
```

**Siri Shortcut alternative** (if needed):
Create a Shortcut that:
1. Takes voice input
2. Saves to iCloud Drive (explicit location)
3. arkai watches that explicit location instead

This is more reliable but requires manual Shortcut setup.

---

*Document generated by Claude Opus 4.5 for the arkai voice capture feature.*
*Last updated: 2026-01-19*

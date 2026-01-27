//! Voice capture ingestion pipeline.
//!
//! This module handles the ingestion of voice memos from the Voice Memos app
//! into the arkai system. The pipeline:
//!
//! 1. **Watcher**: Monitors Voice Memos directory for new .m4a files
//! 2. **Queue**: JSONL-based queue for idempotent processing
//! 3. (Phase 2) Transcriber: Whisper transcription
//! 4. (Phase 3) Depositor: Write to Obsidian vault
//!
//! # Architecture
//!
//! ```text
//! Voice Memos (iCloud) → Watcher → Queue → [Phase 2+]
//!                         ↓
//!                   events.jsonl
//! ```

pub mod queue;
pub mod transcriber;
pub mod watcher;

// Re-export key types
pub use queue::{QueueItem, VoiceQueue, VoiceQueueError};
pub use transcriber::{transcribe, TranscriptResult};
pub use watcher::{AudioFileEvent, VoiceMemoWatcher, WatcherConfig};

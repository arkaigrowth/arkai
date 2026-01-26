//! arkai - Event-sourced AI pipeline orchestrator
//!
//! A Rust-based orchestrator for AI pipelines using Fabric as the
//! execution backend.
//!
//! # Architecture
//!
//! The system is built around event sourcing:
//! - All state changes are recorded as immutable events
//! - Current state is derived by replaying events
//! - Failed runs can be resumed from the last successful step
//!
//! # Modules
//!
//! - `adapters`: External system integrations (Fabric)
//! - `core`: Orchestration logic (EventStore, Pipeline, Safety)
//! - `domain`: Data structures (Event, Run, Artifact)
//! - `cli`: Command-line interface
//!
//! # Usage
//!
//! ```bash
//! # Run a pipeline
//! echo "input text" | arkai run hello
//!
//! # Check run status
//! arkai status <run-id>
//!
//! # Resume a failed run
//! arkai resume <run-id>
//! ```

pub mod adapters;
pub mod cli;
pub mod config;
pub mod core;
pub mod domain;
pub mod evidence;
pub mod ingest;
pub mod library;

// Re-export main types at crate root for convenience
pub use core::Orchestrator;
pub use domain::{Event, EventType, Run, RunState};
pub use evidence::{Evidence, MatchResult, MatchStatus, Span, Status as EvidenceStatus};
pub use library::{Catalog, CatalogItem, ContentId, ContentType, LibraryContent};

// Voice capture (Phase 1)
pub use ingest::{AudioFileEvent, QueueItem, VoiceMemoWatcher, VoiceQueue, WatcherConfig};

// Telegram integration
pub use adapters::{TelegramClient, TelegramConfig};

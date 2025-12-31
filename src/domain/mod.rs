//! Domain types for the arkai orchestrator.
//!
//! This module contains the core data structures:
//! - Events: Immutable records of state changes
//! - Run: Pipeline execution state
//! - Artifact: Step outputs

pub mod artifact;
pub mod events;
pub mod run;

// Re-export commonly used types
pub use artifact::{Artifact, ArtifactType};
pub use events::{Event, EventType, StepStatus};
pub use run::{Run, RunState};

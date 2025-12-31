//! Core orchestration logic.
//!
//! This module contains:
//! - EventStore: Append-only event logging
//! - Pipeline: Pipeline definitions and loading
//! - Safety: Safety limits and enforcement
//! - Orchestrator: Main execution engine

pub mod event_store;
pub mod orchestrator;
pub mod pipeline;
pub mod safety;

// Re-export commonly used types
pub use event_store::{generate_idempotency_key, hash_input, EventStore};
pub use orchestrator::Orchestrator;
pub use pipeline::{AdapterType, InputSource, Pipeline, RetryPolicy, Step};
pub use safety::{SafetyLimits, SafetyTracker, SafetyViolation};

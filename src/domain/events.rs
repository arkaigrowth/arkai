//! Event types for the event-sourced orchestrator.
//!
//! All state changes are recorded as immutable events in an append-only log.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single event in the append-only event log.
///
/// Events are the source of truth for run state. The current state of any run
/// can be reconstructed by replaying its events in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier for this event
    pub id: Uuid,

    /// When this event occurred (ISO 8601)
    pub timestamp: DateTime<Utc>,

    /// The run this event belongs to
    pub run_id: Uuid,

    /// Current step name (if applicable)
    pub step_id: Option<String>,

    /// Type of event
    pub event_type: EventType,

    /// Idempotency key format: "{run_id}:{step}:{input_hash}"
    pub idempotency_key: String,

    /// Human-readable summary (NO secrets)
    pub payload_summary: String,

    /// Current status of the step/run
    pub status: StepStatus,

    /// Time taken in milliseconds (for completed steps)
    pub duration_ms: Option<u64>,

    /// Error message if failed
    pub error: Option<String>,
}

impl Event {
    /// Create a new event with the current timestamp
    pub fn new(
        run_id: Uuid,
        step_id: Option<String>,
        event_type: EventType,
        idempotency_key: String,
        payload_summary: String,
        status: StepStatus,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            run_id,
            step_id,
            event_type,
            idempotency_key,
            payload_summary,
            status,
            duration_ms: None,
            error: None,
        }
    }

    /// Create an event with duration information
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Create an event with error information
    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }
}

/// Types of events that can occur during pipeline execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// A new run has started
    RunStarted,

    /// A run completed successfully
    RunCompleted,

    /// A run failed
    RunFailed,

    /// A step has started execution
    StepStarted,

    /// A step completed successfully
    StepCompleted,

    /// A step failed (may or may not retry)
    StepFailed,

    /// A step is being retried after failure
    StepRetrying,

    /// A safety limit was reached, halting execution
    SafetyLimitReached,
}

/// Status of a step or run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Not yet started
    Pending,

    /// Currently executing
    Running,

    /// Completed successfully
    Completed,

    /// Failed (with error)
    Failed,

    /// Skipped (idempotency check)
    Skipped,
}

impl Default for StepStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = Event::new(
            Uuid::new_v4(),
            Some("summarize".to_string()),
            EventType::StepStarted,
            "test-key".to_string(),
            "Starting summarize step".to_string(),
            StepStatus::Running,
        );

        let json = serde_json::to_string(&event).unwrap();
        let parsed: Event = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.event_type, EventType::StepStarted);
        assert_eq!(parsed.status, StepStatus::Running);
    }

    #[test]
    fn test_event_with_duration() {
        let event = Event::new(
            Uuid::new_v4(),
            Some("summarize".to_string()),
            EventType::StepCompleted,
            "test-key".to_string(),
            "Completed summarize step".to_string(),
            StepStatus::Completed,
        )
        .with_duration(1500);

        assert_eq!(event.duration_ms, Some(1500));
    }

    #[test]
    fn test_event_with_error() {
        let event = Event::new(
            Uuid::new_v4(),
            Some("summarize".to_string()),
            EventType::StepFailed,
            "test-key".to_string(),
            "Failed summarize step".to_string(),
            StepStatus::Failed,
        )
        .with_error("Connection timeout".to_string());

        assert_eq!(event.error, Some("Connection timeout".to_string()));
    }
}

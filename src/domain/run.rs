//! Run state and reconstruction from events.
//!
//! A Run represents a single execution of a pipeline.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::artifact::Artifact;
use super::events::{Event, EventType, StepStatus};

/// A pipeline execution run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    /// Unique identifier for this run
    pub id: Uuid,

    /// Name of the pipeline being executed
    pub pipeline_name: String,

    /// Input provided to the pipeline
    pub input: String,

    /// Current state of the run
    pub state: RunState,

    /// When the run started
    pub started_at: DateTime<Utc>,

    /// When the run completed (if applicable)
    pub completed_at: Option<DateTime<Utc>>,

    /// Index of the current step being executed
    pub current_step: usize,

    /// Artifacts produced by completed steps
    pub artifacts: HashMap<String, Artifact>,

    /// Status of each step (step_name -> status)
    pub step_statuses: HashMap<String, StepStatus>,
}

impl Run {
    /// Create a new run for a pipeline
    pub fn new(id: Uuid, pipeline_name: String, input: String) -> Self {
        Self {
            id,
            pipeline_name,
            input,
            state: RunState::Running,
            started_at: Utc::now(),
            completed_at: None,
            current_step: 0,
            artifacts: HashMap::new(),
            step_statuses: HashMap::new(),
        }
    }

    /// Reconstruct run state from a sequence of events
    pub fn from_events(events: &[Event]) -> Option<Self> {
        if events.is_empty() {
            return None;
        }

        let first_event = events.first()?;
        let run_id = first_event.run_id;

        let mut run = Self {
            id: run_id,
            pipeline_name: String::new(),
            input: String::new(),
            state: RunState::Running,
            started_at: first_event.timestamp,
            completed_at: None,
            current_step: 0,
            artifacts: HashMap::new(),
            step_statuses: HashMap::new(),
        };

        for event in events {
            run.apply_event(event);
        }

        Some(run)
    }

    /// Apply a single event to update run state
    pub fn apply_event(&mut self, event: &Event) {
        match event.event_type {
            EventType::RunStarted => {
                self.state = RunState::Running;
                self.started_at = event.timestamp;
            }
            EventType::RunCompleted => {
                self.state = RunState::Completed;
                self.completed_at = Some(event.timestamp);
            }
            EventType::RunFailed => {
                self.state = RunState::Failed {
                    error: event.error.clone().unwrap_or_default(),
                };
                self.completed_at = Some(event.timestamp);
            }
            EventType::StepStarted => {
                if let Some(ref step_id) = event.step_id {
                    self.step_statuses
                        .insert(step_id.clone(), StepStatus::Running);
                }
            }
            EventType::StepCompleted => {
                if let Some(ref step_id) = event.step_id {
                    self.step_statuses
                        .insert(step_id.clone(), StepStatus::Completed);
                    self.current_step += 1;
                }
            }
            EventType::StepFailed => {
                if let Some(ref step_id) = event.step_id {
                    self.step_statuses
                        .insert(step_id.clone(), StepStatus::Failed);
                }
            }
            EventType::StepRetrying => {
                if let Some(ref step_id) = event.step_id {
                    self.step_statuses
                        .insert(step_id.clone(), StepStatus::Running);
                }
            }
            EventType::SafetyLimitReached => {
                self.state = RunState::SafetyLimitReached {
                    limit: event.error.clone().unwrap_or_default(),
                };
                self.completed_at = Some(event.timestamp);
            }
        }
    }

    /// Check if the run is still in progress
    pub fn is_running(&self) -> bool {
        matches!(self.state, RunState::Running)
    }

    /// Check if the run has completed (successfully or not)
    pub fn is_finished(&self) -> bool {
        !self.is_running()
    }

    /// Check if a specific step is completed
    pub fn is_step_completed(&self, step_name: &str) -> bool {
        self.step_statuses
            .get(step_name)
            .map(|s| *s == StepStatus::Completed)
            .unwrap_or(false)
    }
}

/// State of a pipeline run
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum RunState {
    /// Currently executing
    Running,

    /// Paused (can be resumed)
    Paused,

    /// Completed successfully
    Completed,

    /// Failed with error
    Failed { error: String },

    /// Safety limit was reached
    SafetyLimitReached { limit: String },
}

impl Default for RunState {
    fn default() -> Self {
        Self::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::Event;

    #[test]
    fn test_run_creation() {
        let run_id = Uuid::new_v4();
        let run = Run::new(run_id, "hello".to_string(), "test input".to_string());

        assert_eq!(run.id, run_id);
        assert_eq!(run.pipeline_name, "hello");
        assert!(run.is_running());
    }

    #[test]
    fn test_run_from_events() {
        let run_id = Uuid::new_v4();

        let events = vec![
            Event::new(
                run_id,
                None,
                EventType::RunStarted,
                format!("{}:start", run_id),
                "Run started".to_string(),
                StepStatus::Running,
            ),
            Event::new(
                run_id,
                Some("step1".to_string()),
                EventType::StepStarted,
                format!("{}:step1:abc", run_id),
                "Step started".to_string(),
                StepStatus::Running,
            ),
            Event::new(
                run_id,
                Some("step1".to_string()),
                EventType::StepCompleted,
                format!("{}:step1:abc", run_id),
                "Step completed".to_string(),
                StepStatus::Completed,
            ),
            Event::new(
                run_id,
                None,
                EventType::RunCompleted,
                format!("{}:complete", run_id),
                "Run completed".to_string(),
                StepStatus::Completed,
            ),
        ];

        let run = Run::from_events(&events).unwrap();

        assert_eq!(run.id, run_id);
        assert_eq!(run.state, RunState::Completed);
        assert!(run.is_step_completed("step1"));
    }
}

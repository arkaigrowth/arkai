//! Main orchestrator for pipeline execution.
//!
//! Coordinates step execution, event logging, retry handling,
//! and safety limit enforcement.

use std::collections::HashMap;
use std::time::Instant;

use anyhow::{Context, Result};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::adapters::{Adapter, FabricAdapter};
use crate::domain::{Artifact, Event, EventType, Run, StepStatus};

use super::event_store::{generate_idempotency_key, EventStore};
use super::pipeline::{AdapterType, InputSource, Pipeline, Step};
use super::safety::{SafetyLimits, SafetyTracker, SafetyViolation};

/// Main pipeline orchestrator
pub struct Orchestrator {
    /// Fabric adapter for pattern execution
    fabric_adapter: FabricAdapter,
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Orchestrator {
    /// Create a new orchestrator
    pub fn new() -> Self {
        Self {
            fabric_adapter: FabricAdapter::new(),
        }
    }

    /// Execute a pipeline with the given input
    #[instrument(skip(self, pipeline, input), fields(pipeline = %pipeline.name))]
    pub async fn run_pipeline(&self, pipeline: &Pipeline, input: String) -> Result<Run> {
        let run_id = Uuid::new_v4();
        info!(%run_id, "Starting pipeline execution");

        // Create event store for this run
        let store = EventStore::open(run_id).await?;

        // Initialize run state
        let mut run = Run::new(run_id, pipeline.name.clone(), input.clone());
        let mut tracker = SafetyTracker::new();
        let mut artifacts: HashMap<String, Artifact> = HashMap::new();

        // Log run start
        let start_event = Event::new(
            run_id,
            None,
            EventType::RunStarted,
            format!("{}:start", run_id),
            format!("Pipeline '{}' started", pipeline.name),
            StepStatus::Running,
        );
        store.append(&start_event).await?;

        // Execute each step
        for (step_idx, step) in pipeline.steps.iter().enumerate() {
            run.current_step = step_idx;

            // Safety check before each step
            if let Err(violation) = pipeline.safety_limits.check(&tracker) {
                return self
                    .handle_safety_violation(&store, &mut run, violation)
                    .await;
            }

            // Resolve input for this step
            let step_input = self.resolve_input(&input, &artifacts, step)?;

            // Validate input
            pipeline.safety_limits.validate_input(&step_input, None)?;

            // Execute step with retry
            match self
                .execute_step_with_retry(
                    &store,
                    &mut run,
                    step,
                    &step_input,
                    &pipeline.safety_limits,
                    &mut tracker,
                )
                .await
            {
                Ok(artifact) => {
                    artifacts.insert(step.name.clone(), artifact.clone());
                    run.artifacts.insert(step.name.clone(), artifact);
                    tracker.record_step(step_input.len() as u64, 0);
                }
                Err(e) => {
                    return self.handle_run_failure(&store, &mut run, e).await;
                }
            }
        }

        // Log run completion
        self.complete_run(&store, &mut run).await
    }

    /// Resume a previously failed run
    #[instrument(skip(self, pipeline), fields(run_id = %run_id, pipeline = %pipeline.name))]
    pub async fn resume_run(&self, run_id: Uuid, pipeline: &Pipeline, input: String) -> Result<Run> {
        info!("Resuming run");

        let store = EventStore::open(run_id).await?;
        let events = store.replay().await?;

        if events.is_empty() {
            anyhow::bail!("No events found for run {}", run_id);
        }

        // Reconstruct run state
        let mut run = Run::from_events(&events)
            .context("Failed to reconstruct run state")?;

        let mut tracker = SafetyTracker::new();
        let mut artifacts: HashMap<String, Artifact> = run.artifacts.clone();

        // Find the first incomplete step
        let start_step = run.current_step;

        info!(start_step, "Resuming from step");

        // Execute remaining steps
        for (step_idx, step) in pipeline.steps.iter().enumerate().skip(start_step) {
            run.current_step = step_idx;

            // Safety check
            if let Err(violation) = pipeline.safety_limits.check(&tracker) {
                return self
                    .handle_safety_violation(&store, &mut run, violation)
                    .await;
            }

            // Resolve input
            let step_input = self.resolve_input(&input, &artifacts, step)?;

            // Check idempotency - skip if already completed
            let idem_key = generate_idempotency_key(run_id, &step.name, &step_input);
            if store.is_step_completed(&idem_key).await? {
                info!(step = %step.name, "Step already completed, skipping");
                continue;
            }

            // Execute step
            match self
                .execute_step_with_retry(
                    &store,
                    &mut run,
                    step,
                    &step_input,
                    &pipeline.safety_limits,
                    &mut tracker,
                )
                .await
            {
                Ok(artifact) => {
                    artifacts.insert(step.name.clone(), artifact.clone());
                    run.artifacts.insert(step.name.clone(), artifact);
                    tracker.record_step(step_input.len() as u64, 0);
                }
                Err(e) => {
                    return self.handle_run_failure(&store, &mut run, e).await;
                }
            }
        }

        self.complete_run(&store, &mut run).await
    }

    /// Execute a step with retry logic
    async fn execute_step_with_retry(
        &self,
        store: &EventStore,
        run: &mut Run,
        step: &Step,
        input: &str,
        limits: &SafetyLimits,
        tracker: &mut SafetyTracker,
    ) -> Result<Artifact> {
        let idem_key = generate_idempotency_key(run.id, &step.name, input);
        let timeout = step.timeout(limits);

        // Check idempotency first
        if store.is_step_completed(&idem_key).await? {
            debug!(step = %step.name, "Step already completed (idempotency check)");
            // Load artifact from events
            if let Some(artifact) = run.artifacts.get(&step.name) {
                return Ok(artifact.clone());
            }
            // Return a placeholder if we can't find the artifact
            return Ok(Artifact::from_output(step.name.clone(), String::new()));
        }

        let mut attempt = 0u32;

        loop {
            attempt += 1;
            let step_start = Instant::now();

            // Log step start
            let start_event = Event::new(
                run.id,
                Some(step.name.clone()),
                EventType::StepStarted,
                idem_key.clone(),
                format!("Step '{}' attempt {}", step.name, attempt),
                StepStatus::Running,
            );
            store.append(&start_event).await?;
            run.step_statuses
                .insert(step.name.clone(), StepStatus::Running);

            // Execute via adapter
            let result = match step.adapter {
                AdapterType::Fabric => {
                    self.fabric_adapter
                        .execute(&step.action, input, timeout)
                        .await
                }
            };

            let duration_ms = step_start.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    // Validate output
                    limits.validate_output(&output.content)?;

                    // Update tracker with output bytes
                    tracker.output_bytes += output.content.len() as u64;

                    // Persist artifact to disk
                    store.store_artifact(&step.name, &output.content).await?;

                    // Log success
                    let complete_event = Event::new(
                        run.id,
                        Some(step.name.clone()),
                        EventType::StepCompleted,
                        idem_key,
                        format!("Step '{}' completed in {}ms", step.name, duration_ms),
                        StepStatus::Completed,
                    )
                    .with_duration(duration_ms);
                    store.append(&complete_event).await?;
                    run.step_statuses
                        .insert(step.name.clone(), StepStatus::Completed);

                    let artifact = Artifact::from_output(step.name.clone(), output.content);
                    return Ok(artifact);
                }
                Err(e) => {
                    // Check if we should retry
                    if step.retry_policy.should_retry(attempt) {
                        let delay = step.retry_policy.delay_for_attempt(attempt);

                        // Log retry
                        let retry_event = Event::new(
                            run.id,
                            Some(step.name.clone()),
                            EventType::StepRetrying,
                            format!("{}:retry:{}", idem_key, attempt),
                            format!(
                                "Step '{}' failed, retrying in {:?}: {}",
                                step.name, delay, e
                            ),
                            StepStatus::Running,
                        )
                        .with_error(e.to_string());
                        store.append(&retry_event).await?;

                        warn!(
                            step = %step.name,
                            attempt,
                            delay_ms = delay.as_millis() as u64,
                            error = %e,
                            "Step failed, retrying"
                        );

                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    // Log final failure
                    let fail_event = Event::new(
                        run.id,
                        Some(step.name.clone()),
                        EventType::StepFailed,
                        idem_key,
                        format!(
                            "Step '{}' failed after {} attempts: {}",
                            step.name, attempt, e
                        ),
                        StepStatus::Failed,
                    )
                    .with_duration(duration_ms)
                    .with_error(e.to_string());
                    store.append(&fail_event).await?;
                    run.step_statuses
                        .insert(step.name.clone(), StepStatus::Failed);

                    error!(
                        step = %step.name,
                        attempt,
                        error = %e,
                        "Step failed permanently"
                    );

                    return Err(e);
                }
            }
        }
    }

    /// Resolve input for a step based on its InputSource
    fn resolve_input(
        &self,
        pipeline_input: &str,
        artifacts: &HashMap<String, Artifact>,
        step: &Step,
    ) -> Result<String> {
        match &step.input_from {
            InputSource::PipelineInput(_) => Ok(pipeline_input.to_string()),

            InputSource::PreviousStep { previous_step } => artifacts
                .get(previous_step)
                .map(|a| a.content.clone())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Step '{}' references non-existent artifact from step '{}'",
                        step.name,
                        previous_step
                    )
                }),

            InputSource::Artifact { artifact } => artifacts
                .get(artifact)
                .map(|a| a.content.clone())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Step '{}' references non-existent artifact '{}'",
                        step.name,
                        artifact
                    )
                }),

            InputSource::Static { value } => {
                Ok(serde_json::to_string(value).unwrap_or_default())
            }
        }
    }

    /// Handle a safety violation by logging and updating run state
    async fn handle_safety_violation(
        &self,
        store: &EventStore,
        run: &mut Run,
        violation: SafetyViolation,
    ) -> Result<Run> {
        let error_msg = violation.to_string();
        error!(%error_msg, "Safety limit reached");

        run.state = crate::domain::RunState::SafetyLimitReached {
            limit: error_msg.clone(),
        };
        run.completed_at = Some(chrono::Utc::now());

        let event = Event::new(
            run.id,
            None,
            EventType::SafetyLimitReached,
            format!("{}:safety", run.id),
            format!("Safety limit reached: {}", error_msg),
            StepStatus::Failed,
        )
        .with_error(error_msg);
        store.append(&event).await?;

        Ok(run.clone())
    }

    /// Handle a run failure
    async fn handle_run_failure(
        &self,
        store: &EventStore,
        run: &mut Run,
        error: anyhow::Error,
    ) -> Result<Run> {
        let error_msg = error.to_string();
        error!(%error_msg, "Run failed");

        run.state = crate::domain::RunState::Failed {
            error: error_msg.clone(),
        };
        run.completed_at = Some(chrono::Utc::now());

        let event = Event::new(
            run.id,
            None,
            EventType::RunFailed,
            format!("{}:complete", run.id),
            format!("Run failed: {}", error_msg),
            StepStatus::Failed,
        )
        .with_error(error_msg);
        store.append(&event).await?;

        Ok(run.clone())
    }

    /// Complete a successful run
    async fn complete_run(&self, store: &EventStore, run: &mut Run) -> Result<Run> {
        info!(run_id = %run.id, "Run completed successfully");

        run.state = crate::domain::RunState::Completed;
        run.completed_at = Some(chrono::Utc::now());

        let event = Event::new(
            run.id,
            None,
            EventType::RunCompleted,
            format!("{}:complete", run.id),
            format!("Pipeline '{}' completed", run.pipeline_name),
            StepStatus::Completed,
        );
        store.append(&event).await?;

        Ok(run.clone())
    }

    /// Get status of a run by ID
    pub async fn get_run_status(&self, run_id: Uuid) -> Result<Run> {
        let store = EventStore::open(run_id).await?;
        let events = store.replay().await?;

        if events.is_empty() {
            anyhow::bail!("Run {} not found", run_id);
        }

        Run::from_events(&events).context("Failed to reconstruct run state")
    }

    /// List recent runs
    pub async fn list_runs(&self, limit: usize) -> Result<Vec<Run>> {
        let run_ids = EventStore::list_runs().await?;
        let mut runs = Vec::new();

        for run_id in run_ids.into_iter().take(limit) {
            if let Ok(run) = self.get_run_status(run_id).await {
                runs.push(run);
            }
        }

        // Sort by start time (most recent first)
        runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        Ok(runs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = Orchestrator::new();
        assert_eq!(orchestrator.fabric_adapter.name(), "fabric");
    }
}

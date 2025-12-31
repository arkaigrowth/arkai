//! Pipeline definitions and loading.
//!
//! Pipelines are defined in YAML and consist of ordered steps,
//! each targeting an adapter (e.g., Fabric) with specific actions.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::safety::SafetyLimits;

/// A complete pipeline definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    /// Pipeline name (used in CLI)
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Safety limits for this pipeline
    #[serde(default)]
    pub safety_limits: SafetyLimits,

    /// Ordered list of steps to execute
    pub steps: Vec<Step>,
}

impl Pipeline {
    /// Load a pipeline from a YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read pipeline file: {}", path.display()))?;

        Self::from_yaml(&content)
    }

    /// Parse a pipeline from YAML content
    pub fn from_yaml(content: &str) -> Result<Self> {
        serde_yaml::from_str(content).context("Failed to parse pipeline YAML")
    }

    /// Validate the pipeline definition
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("Pipeline name cannot be empty");
        }

        if self.steps.is_empty() {
            anyhow::bail!("Pipeline must have at least one step");
        }

        // Validate step references
        let step_names: Vec<&str> = self.steps.iter().map(|s| s.name.as_str()).collect();

        for (i, step) in self.steps.iter().enumerate() {
            if step.name.is_empty() {
                anyhow::bail!("Step {} has an empty name", i);
            }

            // Check that previous_step references exist
            if let InputSource::PreviousStep { ref previous_step } = step.input_from {
                let step_index = step_names.iter().position(|&n| n == previous_step);
                match step_index {
                    Some(idx) if idx >= i => {
                        anyhow::bail!(
                            "Step '{}' references future step '{}' (forward references not allowed)",
                            step.name,
                            previous_step
                        );
                    }
                    None => {
                        anyhow::bail!(
                            "Step '{}' references non-existent step '{}'",
                            step.name,
                            previous_step
                        );
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Get a step by name
    pub fn get_step(&self, name: &str) -> Option<&Step> {
        self.steps.iter().find(|s| s.name == name)
    }

    /// Get the index of a step by name
    pub fn step_index(&self, name: &str) -> Option<usize> {
        self.steps.iter().position(|s| s.name == name)
    }
}

/// A single step in a pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Step name (unique within pipeline)
    pub name: String,

    /// Adapter to use (e.g., "fabric")
    pub adapter: AdapterType,

    /// Action/pattern to execute
    pub action: String,

    /// Where to get input from
    #[serde(default)]
    pub input_from: InputSource,

    /// Retry policy for this step
    #[serde(default)]
    pub retry_policy: RetryPolicy,

    /// Override timeout for this step (uses safety_limits.step_timeout_seconds if not set)
    pub timeout_seconds: Option<u64>,
}

impl Step {
    /// Get the effective timeout for this step
    pub fn timeout(&self, limits: &SafetyLimits) -> Duration {
        let seconds = self.timeout_seconds.unwrap_or(limits.step_timeout_seconds);
        Duration::from_secs(seconds)
    }
}

/// Supported adapter types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterType {
    /// Fabric CLI/API
    Fabric,
}

impl Default for AdapterType {
    fn default() -> Self {
        Self::Fabric
    }
}

/// Source of input for a step
///
/// Supports multiple YAML formats:
/// - Simple: `input_from: pipeline_input`
/// - Previous step: `input_from: { previous_step: step_name }`
/// - Artifact: `input_from: { artifact: artifact_name }`
/// - Static: `input_from: { static: { key: value } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputSource {
    /// Use the pipeline's original input (simple string "pipeline_input")
    PipelineInput(PipelineInputMarker),

    /// Use output from a previous step
    PreviousStep {
        previous_step: String,
    },

    /// Use a stored artifact
    Artifact {
        artifact: String,
    },

    /// Static value
    Static {
        #[serde(rename = "static")]
        value: serde_json::Value,
    },
}

/// Marker for pipeline_input (deserializes from the string "pipeline_input")
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineInputMarker {
    PipelineInput,
}

impl Default for InputSource {
    fn default() -> Self {
        Self::PipelineInput(PipelineInputMarker::PipelineInput)
    }
}

/// Retry policy for failed steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including first try)
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Initial delay between retries in milliseconds
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,

    /// Maximum delay between retries in milliseconds
    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,

    /// Backoff multiplier (delay *= multiplier after each retry)
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

fn default_max_attempts() -> u32 {
    3
}
fn default_initial_delay() -> u64 {
    1000
}
fn default_max_delay() -> u64 {
    30000
}
fn default_backoff_multiplier() -> f64 {
    2.0
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            initial_delay_ms: default_initial_delay(),
            max_delay_ms: default_max_delay(),
            backoff_multiplier: default_backoff_multiplier(),
        }
    }
}

impl RetryPolicy {
    /// Calculate delay for a specific attempt (1-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt <= 1 {
            return Duration::from_millis(self.initial_delay_ms);
        }

        let delay = self.initial_delay_ms as f64
            * self.backoff_multiplier.powi((attempt - 1) as i32);

        let capped = delay.min(self.max_delay_ms as f64) as u64;
        Duration::from_millis(capped)
    }

    /// Check if we should retry based on attempt count
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PIPELINE_YAML: &str = r#"
name: test
description: Test pipeline

safety_limits:
  max_steps: 10

steps:
  - name: first
    adapter: fabric
    action: summarize
    input_from: pipeline_input

  - name: second
    adapter: fabric
    action: analyze
    input_from:
      previous_step: first
"#;

    #[test]
    fn test_pipeline_parsing() {
        let pipeline = Pipeline::from_yaml(TEST_PIPELINE_YAML).unwrap();

        assert_eq!(pipeline.name, "test");
        assert_eq!(pipeline.steps.len(), 2);
        assert_eq!(pipeline.safety_limits.max_steps, 10);
    }

    #[test]
    fn test_pipeline_validation() {
        let pipeline = Pipeline::from_yaml(TEST_PIPELINE_YAML).unwrap();
        assert!(pipeline.validate().is_ok());
    }

    #[test]
    fn test_invalid_step_reference() {
        let yaml = r#"
name: invalid
description: Invalid pipeline
steps:
  - name: first
    adapter: fabric
    action: test
    input_from:
      previous_step: nonexistent
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        assert!(pipeline.validate().is_err());
    }

    #[test]
    fn test_retry_policy_delays() {
        let policy = RetryPolicy {
            initial_delay_ms: 1000,
            backoff_multiplier: 2.0,
            max_delay_ms: 10000,
            ..Default::default()
        };

        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(1000));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(2000));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(4000));
        assert_eq!(policy.delay_for_attempt(4), Duration::from_millis(8000));
        assert_eq!(policy.delay_for_attempt(5), Duration::from_millis(10000)); // Capped
    }
}

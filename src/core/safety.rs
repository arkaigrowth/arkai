//! Safety limits and enforcement for pipeline execution.
//!
//! Prevents runaway execution through configurable limits on:
//! - Number of steps
//! - Input/output sizes
//! - Execution timeouts
//! - Denylist patterns (to avoid processing secrets)

use std::path::Path;
use std::time::Instant;

use glob::Pattern;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Safety limits for pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyLimits {
    /// Maximum number of steps per run (default: 50)
    #[serde(default = "default_max_steps")]
    pub max_steps: u32,

    /// Maximum input size in bytes (default: 10MB)
    #[serde(default = "default_max_input_bytes")]
    pub max_input_bytes: u64,

    /// Maximum output size in bytes (default: 10MB)
    #[serde(default = "default_max_output_bytes")]
    pub max_output_bytes: u64,

    /// Per-step timeout in seconds (default: 300 = 5 min)
    #[serde(default = "default_step_timeout")]
    pub step_timeout_seconds: u64,

    /// Total run timeout in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_run_timeout")]
    pub run_timeout_seconds: u64,

    /// Glob patterns to reject (files matching these won't be processed)
    #[serde(default = "default_denylist")]
    pub denylist_patterns: Vec<String>,
}

fn default_max_steps() -> u32 {
    50
}
fn default_max_input_bytes() -> u64 {
    10 * 1024 * 1024
} // 10MB
fn default_max_output_bytes() -> u64 {
    10 * 1024 * 1024
} // 10MB
fn default_step_timeout() -> u64 {
    300
} // 5 min
fn default_run_timeout() -> u64 {
    3600
} // 1 hour

fn default_denylist() -> Vec<String> {
    vec![
        "**/.env*".to_string(),
        "**/secrets*".to_string(),
        "**/*credential*".to_string(),
        "**/*.pem".to_string(),
        "**/*.key".to_string(),
    ]
}

impl Default for SafetyLimits {
    fn default() -> Self {
        Self {
            max_steps: default_max_steps(),
            max_input_bytes: default_max_input_bytes(),
            max_output_bytes: default_max_output_bytes(),
            step_timeout_seconds: default_step_timeout(),
            run_timeout_seconds: default_run_timeout(),
            denylist_patterns: default_denylist(),
        }
    }
}

impl SafetyLimits {
    /// Check if an input path matches any denylist pattern
    pub fn is_denylisted(&self, path: &str) -> bool {
        for pattern_str in &self.denylist_patterns {
            if let Ok(pattern) = Pattern::new(pattern_str) {
                if pattern.matches(path) {
                    return true;
                }
            }
        }
        false
    }

    /// Validate input against size limits and denylist
    pub fn validate_input(&self, input: &str, source_path: Option<&Path>) -> Result<(), SafetyViolation> {
        // Check size
        let size = input.len() as u64;
        if size > self.max_input_bytes {
            return Err(SafetyViolation::MaxInputBytes {
                actual: size,
                limit: self.max_input_bytes,
            });
        }

        // Check denylist
        if let Some(path) = source_path {
            let path_str = path.to_string_lossy();
            if self.is_denylisted(&path_str) {
                return Err(SafetyViolation::DenylistMatch {
                    path: path_str.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate output against size limits
    pub fn validate_output(&self, output: &str) -> Result<(), SafetyViolation> {
        let size = output.len() as u64;
        if size > self.max_output_bytes {
            return Err(SafetyViolation::MaxOutputBytes {
                actual: size,
                limit: self.max_output_bytes,
            });
        }
        Ok(())
    }

    /// Check current tracker state against limits
    pub fn check(&self, tracker: &SafetyTracker) -> Result<(), SafetyViolation> {
        // Check step count
        if tracker.steps_executed >= self.max_steps {
            return Err(SafetyViolation::MaxSteps {
                actual: tracker.steps_executed,
                limit: self.max_steps,
            });
        }

        // Check run timeout
        let elapsed = tracker.started_at.elapsed().as_secs();
        if elapsed >= self.run_timeout_seconds {
            return Err(SafetyViolation::RunTimeout {
                elapsed_seconds: elapsed,
                limit_seconds: self.run_timeout_seconds,
            });
        }

        Ok(())
    }
}

/// Tracks resource usage during a run
#[derive(Debug, Clone)]
pub struct SafetyTracker {
    /// Number of steps executed
    pub steps_executed: u32,

    /// Total input bytes processed
    pub input_bytes: u64,

    /// Total output bytes produced
    pub output_bytes: u64,

    /// When the run started
    pub started_at: Instant,
}

impl Default for SafetyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SafetyTracker {
    /// Create a new tracker
    pub fn new() -> Self {
        Self {
            steps_executed: 0,
            input_bytes: 0,
            output_bytes: 0,
            started_at: Instant::now(),
        }
    }

    /// Record a step execution
    pub fn record_step(&mut self, input_bytes: u64, output_bytes: u64) {
        self.steps_executed += 1;
        self.input_bytes += input_bytes;
        self.output_bytes += output_bytes;
    }

    /// Get elapsed time in seconds
    pub fn elapsed_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}

/// Safety violation errors
#[derive(Debug, Clone, Error)]
pub enum SafetyViolation {
    #[error("Maximum steps exceeded: {actual} >= {limit}")]
    MaxSteps { actual: u32, limit: u32 },

    #[error("Maximum input bytes exceeded: {actual} > {limit}")]
    MaxInputBytes { actual: u64, limit: u64 },

    #[error("Maximum output bytes exceeded: {actual} > {limit}")]
    MaxOutputBytes { actual: u64, limit: u64 },

    #[error("Step timeout: {elapsed_seconds}s >= {limit_seconds}s")]
    StepTimeout {
        elapsed_seconds: u64,
        limit_seconds: u64,
    },

    #[error("Run timeout: {elapsed_seconds}s >= {limit_seconds}s")]
    RunTimeout {
        elapsed_seconds: u64,
        limit_seconds: u64,
    },

    #[error("Path matches denylist pattern: {path}")]
    DenylistMatch { path: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = SafetyLimits::default();
        assert_eq!(limits.max_steps, 50);
        assert_eq!(limits.max_input_bytes, 10 * 1024 * 1024);
        assert_eq!(limits.step_timeout_seconds, 300);
    }

    #[test]
    fn test_denylist_matching() {
        let limits = SafetyLimits::default();

        assert!(limits.is_denylisted(".env"));
        assert!(limits.is_denylisted(".env.local"));
        assert!(limits.is_denylisted("config/secrets.json"));
        assert!(limits.is_denylisted("keys/server.key"));
        assert!(limits.is_denylisted("certs/server.pem"));

        assert!(!limits.is_denylisted("config.toml"));
        assert!(!limits.is_denylisted("main.rs"));
    }

    #[test]
    fn test_input_validation() {
        let limits = SafetyLimits {
            max_input_bytes: 100,
            ..Default::default()
        };

        assert!(limits.validate_input("short", None).is_ok());

        let long_input = "x".repeat(200);
        let result = limits.validate_input(&long_input, None);
        assert!(matches!(result, Err(SafetyViolation::MaxInputBytes { .. })));
    }

    #[test]
    fn test_tracker_step_counting() {
        let limits = SafetyLimits {
            max_steps: 2,
            ..Default::default()
        };

        let mut tracker = SafetyTracker::new();
        assert!(limits.check(&tracker).is_ok());

        tracker.record_step(100, 100);
        assert!(limits.check(&tracker).is_ok());

        tracker.record_step(100, 100);
        let result = limits.check(&tracker);
        assert!(matches!(result, Err(SafetyViolation::MaxSteps { .. })));
    }
}

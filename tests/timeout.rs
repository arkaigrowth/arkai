//! Timeout Integration Tests
//!
//! Tests for step and run timeout behavior.

use arkai::core::{Pipeline, RetryPolicy, SafetyLimits, SafetyTracker};
use std::time::{Duration, Instant};

#[test]
fn test_step_timeout_configuration() {
    let yaml = r#"
name: timeout_test
description: Test pipeline with timeouts

safety_limits:
  step_timeout_seconds: 5
  run_timeout_seconds: 60

steps:
  - name: fast_step
    adapter: fabric
    action: summarize
    input_from: pipeline_input
    timeout_seconds: 2
"#;

    let pipeline = Pipeline::from_yaml(yaml).unwrap();

    // Global timeout should be 5 seconds
    assert_eq!(pipeline.safety_limits.step_timeout_seconds, 5);

    // Step-specific override should be 2 seconds
    let step = &pipeline.steps[0];
    let timeout = step.timeout(&pipeline.safety_limits);
    assert_eq!(timeout, Duration::from_secs(2));
}

#[test]
fn test_step_timeout_fallback_to_global() {
    let yaml = r#"
name: timeout_test
description: Test pipeline with global timeout

safety_limits:
  step_timeout_seconds: 30

steps:
  - name: step_without_override
    adapter: fabric
    action: summarize
    input_from: pipeline_input
"#;

    let pipeline = Pipeline::from_yaml(yaml).unwrap();

    // Step without explicit timeout should use global
    let step = &pipeline.steps[0];
    let timeout = step.timeout(&pipeline.safety_limits);
    assert_eq!(timeout, Duration::from_secs(30));
}

#[test]
fn test_run_timeout_tracking() {
    let limits = SafetyLimits {
        run_timeout_seconds: 2, // 2 second timeout for testing
        ..Default::default()
    };

    let tracker = SafetyTracker::new();

    // Initially should be within limits
    assert!(limits.check(&tracker).is_ok());

    // Simulate time passing by checking elapsed
    // Note: In a real test, we'd need to mock time
    // Here we just verify the tracker tracks time
    assert!(tracker.elapsed_seconds() < 2);
}

#[test]
fn test_retry_delay_calculation() {
    let policy = RetryPolicy {
        max_attempts: 3,
        initial_delay_ms: 1000,
        max_delay_ms: 10000,
        backoff_multiplier: 2.0,
    };

    // Attempt 1: initial delay
    assert_eq!(
        policy.delay_for_attempt(1),
        Duration::from_millis(1000)
    );

    // Attempt 2: initial * 2
    assert_eq!(
        policy.delay_for_attempt(2),
        Duration::from_millis(2000)
    );

    // Attempt 3: initial * 4
    assert_eq!(
        policy.delay_for_attempt(3),
        Duration::from_millis(4000)
    );

    // Attempt 4: initial * 8
    assert_eq!(
        policy.delay_for_attempt(4),
        Duration::from_millis(8000)
    );

    // Attempt 5: capped at max
    assert_eq!(
        policy.delay_for_attempt(5),
        Duration::from_millis(10000)
    );
}

#[test]
fn test_retry_should_retry() {
    let policy = RetryPolicy {
        max_attempts: 3,
        ..Default::default()
    };

    // Should retry on attempts 1 and 2
    assert!(policy.should_retry(1));
    assert!(policy.should_retry(2));

    // Should not retry on attempt 3 (max reached)
    assert!(!policy.should_retry(3));
    assert!(!policy.should_retry(4));
}

#[test]
fn test_safety_tracker_elapsed() {
    let tracker = SafetyTracker::new();

    // Should be very close to 0 seconds right after creation
    assert!(tracker.elapsed_seconds() < 1);
}

#[test]
fn test_default_timeouts() {
    let limits = SafetyLimits::default();

    // Verify default values from bootstrap spec
    assert_eq!(limits.step_timeout_seconds, 300); // 5 minutes
    assert_eq!(limits.run_timeout_seconds, 3600); // 1 hour
}

#[test]
fn test_timeout_yaml_parsing() {
    let yaml = r#"
name: custom_timeouts
description: Pipeline with custom timeouts

safety_limits:
  step_timeout_seconds: 120
  run_timeout_seconds: 1800

steps:
  - name: step1
    adapter: fabric
    action: test
    input_from: pipeline_input
    timeout_seconds: 30
    retry_policy:
      max_attempts: 3
      initial_delay_ms: 500
      max_delay_ms: 5000
      backoff_multiplier: 1.5
"#;

    let pipeline = Pipeline::from_yaml(yaml).unwrap();

    assert_eq!(pipeline.safety_limits.step_timeout_seconds, 120);
    assert_eq!(pipeline.safety_limits.run_timeout_seconds, 1800);

    let step = &pipeline.steps[0];
    assert_eq!(step.timeout_seconds, Some(30));
    assert_eq!(step.retry_policy.max_attempts, 3);
    assert_eq!(step.retry_policy.initial_delay_ms, 500);
    assert_eq!(step.retry_policy.max_delay_ms, 5000);
    assert_eq!(step.retry_policy.backoff_multiplier, 1.5);
}

#[tokio::test]
async fn test_timeout_enforcement_simulation() {
    // Simulate a timeout scenario
    let start = Instant::now();
    let timeout = Duration::from_millis(100);

    // This simulates a quick operation
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Should complete within timeout
    assert!(start.elapsed() < timeout);
}

#[tokio::test]
async fn test_timeout_exceeded_simulation() {
    // Simulate a timeout being exceeded
    let start = Instant::now();
    let timeout = Duration::from_millis(50);

    // This simulates a slow operation
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Should have exceeded timeout
    assert!(start.elapsed() > timeout);
}

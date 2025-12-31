//! Safety Limits Integration Tests
//!
//! Tests for safety limit enforcement and denylist patterns.

use arkai::core::{SafetyLimits, SafetyTracker, SafetyViolation};
use std::path::Path;

#[test]
fn test_max_input_bytes() {
    let limits = SafetyLimits {
        max_input_bytes: 100,
        ..Default::default()
    };

    // Input within limits
    let small_input = "x".repeat(50);
    assert!(limits.validate_input(&small_input, None).is_ok());

    // Input at exact limit
    let exact_input = "x".repeat(100);
    assert!(limits.validate_input(&exact_input, None).is_ok());

    // Input exceeding limit
    let large_input = "x".repeat(200);
    let result = limits.validate_input(&large_input, None);
    assert!(result.is_err());

    match result {
        Err(SafetyViolation::MaxInputBytes { actual, limit }) => {
            assert_eq!(actual, 200);
            assert_eq!(limit, 100);
        }
        _ => panic!("Expected MaxInputBytes violation"),
    }
}

#[test]
fn test_max_output_bytes() {
    let limits = SafetyLimits {
        max_output_bytes: 100,
        ..Default::default()
    };

    // Output within limits
    let small_output = "y".repeat(50);
    assert!(limits.validate_output(&small_output).is_ok());

    // Output exceeding limit
    let large_output = "y".repeat(200);
    let result = limits.validate_output(&large_output);
    assert!(result.is_err());

    match result {
        Err(SafetyViolation::MaxOutputBytes { actual, limit }) => {
            assert_eq!(actual, 200);
            assert_eq!(limit, 100);
        }
        _ => panic!("Expected MaxOutputBytes violation"),
    }
}

#[test]
fn test_denylist_pattern_env() {
    let limits = SafetyLimits::default();

    // .env files should be blocked
    assert!(limits.is_denylisted(".env"));
    assert!(limits.is_denylisted(".env.local"));
    assert!(limits.is_denylisted(".env.production"));
    assert!(limits.is_denylisted("config/.env"));
}

#[test]
fn test_denylist_pattern_secrets() {
    let limits = SafetyLimits::default();

    // Files starting with "secrets" should be blocked (pattern: **/secrets*)
    assert!(limits.is_denylisted("secrets.json"));
    assert!(limits.is_denylisted("secrets.yaml"));
    assert!(limits.is_denylisted("config/secrets.yaml"));
    assert!(limits.is_denylisted("deep/path/secrets-file"));

    // Note: "my-secrets-file" is NOT blocked because the pattern is **/secrets*
    // which matches files that START with "secrets", not files containing "secrets"
    assert!(!limits.is_denylisted("my-secrets-file"));
}

#[test]
fn test_denylist_pattern_credentials() {
    let limits = SafetyLimits::default();

    // Files with "credential" should be blocked
    assert!(limits.is_denylisted("credentials.json"));
    assert!(limits.is_denylisted("user_credentials.txt"));
    assert!(limits.is_denylisted("aws-credential-file"));
}

#[test]
fn test_denylist_pattern_keys() {
    let limits = SafetyLimits::default();

    // .pem and .key files should be blocked
    assert!(limits.is_denylisted("server.pem"));
    assert!(limits.is_denylisted("private.key"));
    assert!(limits.is_denylisted("certs/ca.pem"));
    assert!(limits.is_denylisted("ssl/domain.key"));
}

#[test]
fn test_denylist_allows_normal_files() {
    let limits = SafetyLimits::default();

    // Normal files should NOT be blocked
    assert!(!limits.is_denylisted("main.rs"));
    assert!(!limits.is_denylisted("config.toml"));
    assert!(!limits.is_denylisted("README.md"));
    assert!(!limits.is_denylisted("src/lib.rs"));
    assert!(!limits.is_denylisted("test.txt"));
}

#[test]
fn test_validate_input_with_denylisted_path() {
    let limits = SafetyLimits::default();

    let input = "some content";

    // Normal path should be allowed
    let normal_path = Path::new("src/main.rs");
    assert!(limits.validate_input(input, Some(normal_path)).is_ok());

    // Denylisted path should be rejected
    let secret_path = Path::new(".env.local");
    let result = limits.validate_input(input, Some(secret_path));
    assert!(result.is_err());

    match result {
        Err(SafetyViolation::DenylistMatch { path }) => {
            assert!(path.contains(".env.local"));
        }
        _ => panic!("Expected DenylistMatch violation"),
    }
}

#[test]
fn test_max_steps_enforcement() {
    let limits = SafetyLimits {
        max_steps: 3,
        ..Default::default()
    };

    let mut tracker = SafetyTracker::new();

    // Should be OK for steps 0, 1, 2
    assert!(limits.check(&tracker).is_ok());

    tracker.record_step(100, 100);
    assert!(limits.check(&tracker).is_ok());

    tracker.record_step(100, 100);
    assert!(limits.check(&tracker).is_ok());

    tracker.record_step(100, 100);
    let result = limits.check(&tracker);
    assert!(result.is_err());

    match result {
        Err(SafetyViolation::MaxSteps { actual, limit }) => {
            assert_eq!(actual, 3);
            assert_eq!(limit, 3);
        }
        _ => panic!("Expected MaxSteps violation"),
    }
}

#[test]
fn test_safety_tracker_recording() {
    let mut tracker = SafetyTracker::new();

    assert_eq!(tracker.steps_executed, 0);
    assert_eq!(tracker.input_bytes, 0);
    assert_eq!(tracker.output_bytes, 0);

    tracker.record_step(100, 200);

    assert_eq!(tracker.steps_executed, 1);
    assert_eq!(tracker.input_bytes, 100);
    assert_eq!(tracker.output_bytes, 200);

    tracker.record_step(300, 400);

    assert_eq!(tracker.steps_executed, 2);
    assert_eq!(tracker.input_bytes, 400);
    assert_eq!(tracker.output_bytes, 600);
}

#[test]
fn test_default_denylist_patterns() {
    let limits = SafetyLimits::default();

    // Should have default denylist patterns
    assert!(!limits.denylist_patterns.is_empty());

    // Verify specific patterns exist
    let patterns: Vec<&str> = limits
        .denylist_patterns
        .iter()
        .map(|s| s.as_str())
        .collect();

    assert!(patterns.iter().any(|p| p.contains(".env")));
    assert!(patterns.iter().any(|p| p.contains("secrets")));
    assert!(patterns.iter().any(|p| p.contains("credential")));
    assert!(patterns.iter().any(|p| p.contains(".pem")));
    assert!(patterns.iter().any(|p| p.contains(".key")));
}

#[test]
fn test_custom_denylist_patterns() {
    let limits = SafetyLimits {
        denylist_patterns: vec![
            "**/*.secret".to_string(),
            "**/private/*".to_string(),
        ],
        ..Default::default()
    };

    // Custom patterns should work
    assert!(limits.is_denylisted("config.secret"));
    assert!(limits.is_denylisted("data/file.secret"));

    // Default patterns should NOT be present
    assert!(!limits.is_denylisted(".env"));
}

#[test]
fn test_safety_limits_yaml_parsing() {
    let yaml = r#"
name: safety_test
description: Test safety limits

safety_limits:
  max_steps: 10
  max_input_bytes: 1048576
  max_output_bytes: 2097152
  step_timeout_seconds: 60
  run_timeout_seconds: 600
  denylist_patterns:
    - "**/*.password"
    - "**/api_keys/*"

steps:
  - name: test
    adapter: fabric
    action: test
    input_from: pipeline_input
"#;

    let pipeline: arkai::core::Pipeline =
        serde_yaml::from_str(yaml).unwrap();

    assert_eq!(pipeline.safety_limits.max_steps, 10);
    assert_eq!(pipeline.safety_limits.max_input_bytes, 1048576);
    assert_eq!(pipeline.safety_limits.max_output_bytes, 2097152);
    assert_eq!(pipeline.safety_limits.step_timeout_seconds, 60);
    assert_eq!(pipeline.safety_limits.run_timeout_seconds, 600);
    assert_eq!(pipeline.safety_limits.denylist_patterns.len(), 2);
}

#[test]
fn test_safety_limits_default_values() {
    let limits = SafetyLimits::default();

    // Verify defaults from bootstrap spec
    assert_eq!(limits.max_steps, 50);
    assert_eq!(limits.max_input_bytes, 10 * 1024 * 1024); // 10MB
    assert_eq!(limits.max_output_bytes, 10 * 1024 * 1024); // 10MB
    assert_eq!(limits.step_timeout_seconds, 300); // 5 min
    assert_eq!(limits.run_timeout_seconds, 3600); // 1 hour
}

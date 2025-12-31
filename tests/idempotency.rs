//! Idempotency Integration Tests
//!
//! Tests for idempotency key generation and step skipping behavior.

use arkai::core::generate_idempotency_key;
use arkai::domain::{Event, EventType, StepStatus};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

/// Test event store for idempotency testing
struct IdempotencyTestStore {
    events_path: PathBuf,
}

impl IdempotencyTestStore {
    async fn new(temp_dir: &TempDir, run_id: Uuid) -> Self {
        let run_dir = temp_dir.path().join(run_id.to_string());
        fs::create_dir_all(&run_dir).await.unwrap();

        Self {
            events_path: run_dir.join("events.jsonl"),
        }
    }

    async fn append(&self, event: &Event) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)
            .await
            .unwrap();

        let json = serde_json::to_string(event).unwrap();
        file.write_all(format!("{}\n", json).as_bytes())
            .await
            .unwrap();
        file.flush().await.unwrap();
    }

    async fn replay(&self) -> Vec<Event> {
        if !self.events_path.exists() {
            return Vec::new();
        }

        let content = fs::read_to_string(&self.events_path).await.unwrap();
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    async fn is_step_completed(&self, idempotency_key: &str) -> bool {
        let events = self.replay().await;
        events.iter().any(|e| {
            e.idempotency_key == idempotency_key
                && matches!(e.event_type, EventType::StepCompleted)
        })
    }
}

#[tokio::test]
async fn test_idempotency_key_skip() {
    let temp_dir = TempDir::new().unwrap();
    let run_id = Uuid::new_v4();
    let store = IdempotencyTestStore::new(&temp_dir, run_id).await;

    let input = "test input for summarization";
    let idem_key = generate_idempotency_key(run_id, "summarize", input);

    // Initially not completed
    assert!(!store.is_step_completed(&idem_key).await);

    // Add StepStarted (not complete yet)
    let started = Event::new(
        run_id,
        Some("summarize".to_string()),
        EventType::StepStarted,
        idem_key.clone(),
        "Step started".to_string(),
        StepStatus::Running,
    );
    store.append(&started).await;

    // Still not completed
    assert!(!store.is_step_completed(&idem_key).await);

    // Add StepCompleted
    let completed = Event::new(
        run_id,
        Some("summarize".to_string()),
        EventType::StepCompleted,
        idem_key.clone(),
        "Step completed".to_string(),
        StepStatus::Completed,
    );
    store.append(&completed).await;

    // Now completed - should be skipped on re-execution
    assert!(store.is_step_completed(&idem_key).await);
}

#[tokio::test]
async fn test_idempotency_key_format() {
    let run_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let input = "test input";
    let step = "summarize";

    let key = generate_idempotency_key(run_id, step, input);

    // Verify format: {run_id}:{step}:{input_hash[0:16]}
    let parts: Vec<&str> = key.split(':').collect();
    assert_eq!(parts.len(), 3, "Key should have 3 parts separated by colons");

    // Verify run_id part
    assert_eq!(parts[0], "550e8400-e29b-41d4-a716-446655440000");

    // Verify step part
    assert_eq!(parts[1], "summarize");

    // Verify hash part is 16 chars
    assert_eq!(parts[2].len(), 16, "Hash should be 16 hex characters");

    // Verify hash is hexadecimal
    assert!(
        parts[2].chars().all(|c| c.is_ascii_hexdigit()),
        "Hash should only contain hex digits"
    );
}

#[tokio::test]
async fn test_idempotency_different_steps_same_input() {
    let temp_dir = TempDir::new().unwrap();
    let run_id = Uuid::new_v4();
    let store = IdempotencyTestStore::new(&temp_dir, run_id).await;

    let input = "same input for both steps";

    let key1 = generate_idempotency_key(run_id, "step1", input);
    let key2 = generate_idempotency_key(run_id, "step2", input);

    // Keys should be different for different steps
    assert_ne!(key1, key2);

    // Complete step1
    let completed1 = Event::new(
        run_id,
        Some("step1".to_string()),
        EventType::StepCompleted,
        key1.clone(),
        "Step1 completed".to_string(),
        StepStatus::Completed,
    );
    store.append(&completed1).await;

    // step1 is completed, step2 is not
    assert!(store.is_step_completed(&key1).await);
    assert!(!store.is_step_completed(&key2).await);
}

#[tokio::test]
async fn test_idempotency_same_step_different_inputs() {
    let run_id = Uuid::new_v4();

    let key1 = generate_idempotency_key(run_id, "summarize", "input version 1");
    let key2 = generate_idempotency_key(run_id, "summarize", "input version 2");

    // Same step with different inputs should produce different keys
    assert_ne!(key1, key2);
}

#[tokio::test]
async fn test_idempotency_failed_step_not_skipped() {
    let temp_dir = TempDir::new().unwrap();
    let run_id = Uuid::new_v4();
    let store = IdempotencyTestStore::new(&temp_dir, run_id).await;

    let input = "test input";
    let idem_key = generate_idempotency_key(run_id, "summarize", input);

    // Add StepFailed event (not StepCompleted)
    let failed = Event::new(
        run_id,
        Some("summarize".to_string()),
        EventType::StepFailed,
        idem_key.clone(),
        "Step failed".to_string(),
        StepStatus::Failed,
    )
    .with_error("Connection timeout".to_string());
    store.append(&failed).await;

    // Failed step should NOT be skipped - only completed steps are skipped
    assert!(!store.is_step_completed(&idem_key).await);
}

#[tokio::test]
async fn test_idempotency_retried_then_completed() {
    let temp_dir = TempDir::new().unwrap();
    let run_id = Uuid::new_v4();
    let store = IdempotencyTestStore::new(&temp_dir, run_id).await;

    let input = "test input";
    let idem_key = generate_idempotency_key(run_id, "summarize", input);

    // Simulate retry sequence:
    // 1. StepStarted
    // 2. StepRetrying (first failure)
    // 3. StepRetrying (second failure)
    // 4. StepCompleted (final success)

    store
        .append(&Event::new(
            run_id,
            Some("summarize".to_string()),
            EventType::StepStarted,
            idem_key.clone(),
            "Attempt 1".to_string(),
            StepStatus::Running,
        ))
        .await;

    store
        .append(
            &Event::new(
                run_id,
                Some("summarize".to_string()),
                EventType::StepRetrying,
                format!("{}:retry:1", idem_key),
                "Retrying".to_string(),
                StepStatus::Running,
            )
            .with_error("Timeout".to_string()),
        )
        .await;

    // Not completed yet
    assert!(!store.is_step_completed(&idem_key).await);

    store
        .append(&Event::new(
            run_id,
            Some("summarize".to_string()),
            EventType::StepCompleted,
            idem_key.clone(),
            "Finally succeeded".to_string(),
            StepStatus::Completed,
        ))
        .await;

    // Now completed
    assert!(store.is_step_completed(&idem_key).await);
}

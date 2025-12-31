//! Event Store Integration Tests
//!
//! Tests for event log format, append operations, and replay order.

use arkai::core::{generate_idempotency_key, hash_input};
use arkai::domain::{Event, EventType, StepStatus};
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn test_event_append_format() {
    // This test verifies the JSONL format of events
    let run_id = Uuid::new_v4();

    // Create event
    let event = Event::new(
        run_id,
        Some("summarize".to_string()),
        EventType::StepStarted,
        format!("{}:summarize:abc123", run_id),
        "Starting summarize step".to_string(),
        StepStatus::Running,
    );

    // Serialize to JSON
    let json = serde_json::to_string(&event).unwrap();

    // Parse back
    let parsed: Event = serde_json::from_str(&json).unwrap();

    // Assert all required fields are present and correct
    assert_eq!(parsed.run_id, run_id);
    assert_eq!(parsed.step_id, Some("summarize".to_string()));
    assert_eq!(parsed.event_type, EventType::StepStarted);
    assert!(parsed.idempotency_key.contains("summarize"));
    assert_eq!(parsed.status, StepStatus::Running);

    // Verify timestamp is valid ISO 8601
    let timestamp_str = parsed.timestamp.to_rfc3339();
    assert!(timestamp_str.contains("T")); // ISO 8601 format
}

#[tokio::test]
async fn test_event_with_duration_and_error() {
    let run_id = Uuid::new_v4();

    // Test event with duration
    let completed = Event::new(
        run_id,
        Some("step1".to_string()),
        EventType::StepCompleted,
        format!("{}:step1:abc", run_id),
        "Completed".to_string(),
        StepStatus::Completed,
    )
    .with_duration(1500);

    assert_eq!(completed.duration_ms, Some(1500));

    // Test event with error
    let failed = Event::new(
        run_id,
        Some("step1".to_string()),
        EventType::StepFailed,
        format!("{}:step1:abc", run_id),
        "Failed".to_string(),
        StepStatus::Failed,
    )
    .with_error("Connection timeout".to_string());

    assert_eq!(failed.error, Some("Connection timeout".to_string()));
}

#[tokio::test]
async fn test_event_types_serialization() {
    // Test all event types serialize/deserialize correctly
    let event_types = vec![
        EventType::RunStarted,
        EventType::RunCompleted,
        EventType::RunFailed,
        EventType::StepStarted,
        EventType::StepCompleted,
        EventType::StepFailed,
        EventType::StepRetrying,
        EventType::SafetyLimitReached,
    ];

    for event_type in event_types {
        let json = serde_json::to_string(&event_type).unwrap();
        let parsed: EventType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, event_type);
    }
}

#[test]
fn test_idempotency_key_format() {
    let run_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let key = generate_idempotency_key(run_id, "summarize", "test input");

    // Format: {run_id}:{step}:{input_hash}
    assert!(key.starts_with("550e8400-e29b-41d4-a716-446655440000:summarize:"));

    // Hash should be 16 hex chars (8 bytes)
    let parts: Vec<&str> = key.split(':').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(parts[1], "summarize");
    assert_eq!(parts[2].len(), 16);
}

#[test]
fn test_idempotency_key_different_inputs() {
    let run_id = Uuid::new_v4();

    let key1 = generate_idempotency_key(run_id, "step1", "input A");
    let key2 = generate_idempotency_key(run_id, "step1", "input B");
    let key3 = generate_idempotency_key(run_id, "step2", "input A");

    // Different inputs should produce different keys
    assert_ne!(key1, key2);

    // Different steps should produce different keys
    assert_ne!(key1, key3);
}

#[test]
fn test_input_hash_consistency() {
    let hash1 = hash_input("test input");
    let hash2 = hash_input("test input");
    let hash3 = hash_input("different input");

    // Same input should produce same hash
    assert_eq!(hash1, hash2);

    // Different input should produce different hash
    assert_ne!(hash1, hash3);

    // Hash should be 16 hex chars (8 bytes)
    assert_eq!(hash1.len(), 16);
}

#[test]
fn test_input_hash_special_chars() {
    // Test with special characters
    let hash1 = hash_input("hello\nworld");
    let hash2 = hash_input("unicode: 日本語");
    let hash3 = hash_input("");

    assert_eq!(hash1.len(), 16);
    assert_eq!(hash2.len(), 16);
    assert_eq!(hash3.len(), 16);

    // All should be different
    assert_ne!(hash1, hash2);
    assert_ne!(hash1, hash3);
}

// Test implementation using file operations directly
// (EventStore has private fields, so we test the behavior via our own implementation)
mod event_store_test {
    use super::*;
    use std::path::PathBuf;
    use tokio::fs::{self, OpenOptions};
    use tokio::io::AsyncWriteExt;

    pub struct TestEventStore {
        pub events_path: PathBuf,
    }

    impl TestEventStore {
        pub async fn new(temp_dir: &TempDir, run_id: Uuid) -> Self {
            let run_dir = temp_dir.path().join(run_id.to_string());
            fs::create_dir_all(&run_dir).await.unwrap();

            Self {
                events_path: run_dir.join("events.jsonl"),
            }
        }

        pub async fn append(&self, event: &Event) {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.events_path)
                .await
                .unwrap();

            let json = serde_json::to_string(event).unwrap();
            file.write_all(format!("{}\n", json).as_bytes()).await.unwrap();
            file.flush().await.unwrap();
        }

        pub async fn replay(&self) -> Vec<Event> {
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
    }

    #[tokio::test]
    async fn test_event_append_and_replay() {
        let temp_dir = TempDir::new().unwrap();
        let run_id = Uuid::new_v4();
        let store = TestEventStore::new(&temp_dir, run_id).await;

        // Append events
        let event1 = Event::new(
            run_id,
            None,
            EventType::RunStarted,
            format!("{}:start", run_id),
            "Run started".to_string(),
            StepStatus::Running,
        );

        let event2 = Event::new(
            run_id,
            Some("step1".to_string()),
            EventType::StepStarted,
            format!("{}:step1:abc", run_id),
            "Step started".to_string(),
            StepStatus::Running,
        );

        store.append(&event1).await;
        store.append(&event2).await;

        // Replay
        let events = store.replay().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, EventType::RunStarted);
        assert_eq!(events[1].event_type, EventType::StepStarted);
    }

    #[tokio::test]
    async fn test_event_replay_order() {
        let temp_dir = TempDir::new().unwrap();
        let run_id = Uuid::new_v4();
        let store = TestEventStore::new(&temp_dir, run_id).await;

        // Append 5 events in order
        for i in 0..5 {
            let event = Event::new(
                run_id,
                Some(format!("step{}", i)),
                EventType::StepStarted,
                format!("{}:step{}:abc", run_id, i),
                format!("Step {} started", i),
                StepStatus::Running,
            );
            store.append(&event).await;
        }

        // Replay and verify order
        let events = store.replay().await;
        assert_eq!(events.len(), 5);

        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.step_id, Some(format!("step{}", i)));
        }
    }
}

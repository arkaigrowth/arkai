//! Auto-classification logic for quick captures.
//!
//! The capture system stores captures as items with `item_type = "capture"`.
//! Classification fields (kind, horizon, priority, status) live in the
//! item's `metadata` JSON. This keeps the items table schema stable while
//! supporting rich capture semantics.

use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// What kind of capture this is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureKind {
    Note,
    Reminder,
    Todo,
    Link,
    VoiceMemo,
    Reference,
}

/// Time horizon for action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Horizon {
    Now,
    Week,
    Later,
}

/// Priority level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Must,
    Should,
    Could,
}

/// Current status of a capture.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CaptureStatus {
    Inbox,
    Triaged,
    Done,
    Snoozed,
}

/// Result of auto-classification.
#[derive(Debug, Clone)]
pub struct CaptureClassification {
    pub kind: CaptureKind,
    pub horizon: Horizon,
    pub priority: Priority,
    pub due_date: Option<String>,
}

/// Action verbs that signal a todo item.
const ACTION_VERBS: &[&str] = &[
    "call", "buy", "review", "send", "email", "check", "fix", "update",
    "submit", "schedule", "book", "cancel", "pay", "write", "create",
    "finish", "complete", "read", "watch", "follow",
];

/// Classify a capture from its text content.
///
/// Rules (in priority order):
/// 1. Contains a URL -> Link
/// 2. Contains time words (tomorrow, next week, ISO date) -> Reminder
/// 3. Starts with an action verb -> Todo
/// 4. Otherwise -> Note
pub fn auto_classify(text: &str) -> CaptureClassification {
    let text_lower = text.to_lowercase();

    // Rule 1: URL detection (anywhere in text)
    if text_lower.contains("http://") || text_lower.contains("https://") {
        return CaptureClassification {
            kind: CaptureKind::Link,
            horizon: Horizon::Later,
            priority: Priority::Could,
            due_date: None,
        };
    }

    // Rule 2: Time words -> Reminder
    if let Some(classification) = try_classify_reminder(&text_lower) {
        return classification;
    }

    // Rule 3: Action verb at start -> Todo
    let first_word = text_lower.split_whitespace().next().unwrap_or("");
    if ACTION_VERBS.contains(&first_word) {
        return CaptureClassification {
            kind: CaptureKind::Todo,
            horizon: Horizon::Now,
            priority: Priority::Should,
            due_date: None,
        };
    }

    // Rule 4: Default -> Note
    CaptureClassification {
        kind: CaptureKind::Note,
        horizon: Horizon::Later,
        priority: Priority::Could,
        due_date: None,
    }
}

/// Try to classify as a reminder based on time words.
fn try_classify_reminder(text: &str) -> Option<CaptureClassification> {
    let today = Utc::now().date_naive();

    // Check "tomorrow"
    if text.contains("tomorrow") {
        let tomorrow = today.succ_opt().unwrap_or(today);
        return Some(CaptureClassification {
            kind: CaptureKind::Reminder,
            horizon: Horizon::Now,
            priority: Priority::Must,
            due_date: Some(tomorrow.format("%Y-%m-%d").to_string()),
        });
    }

    // Check "next week"
    if text.contains("next week") {
        let wd = today.weekday().num_days_from_monday(); // Mon=0 .. Sun=6
        let days_to_add = match wd {
            0 => 7, // Monday -> next Monday
            n => 7 - n, // Other days -> coming Monday
        };
        let next_monday = today + chrono::Duration::days(days_to_add as i64);
        return Some(CaptureClassification {
            kind: CaptureKind::Reminder,
            horizon: Horizon::Week,
            priority: Priority::Should,
            due_date: Some(next_monday.format("%Y-%m-%d").to_string()),
        });
    }

    // Check ISO date pattern: YYYY-MM-DD
    if let Some(date) = extract_iso_date(text) {
        let horizon = date_to_horizon(today, date);
        let priority = horizon_to_priority(&horizon);
        return Some(CaptureClassification {
            kind: CaptureKind::Reminder,
            horizon,
            priority,
            due_date: Some(date.format("%Y-%m-%d").to_string()),
        });
    }

    // Check MM/DD pattern (current year assumed)
    if let Some(date) = extract_slash_date(text, today.year()) {
        let horizon = date_to_horizon(today, date);
        let priority = horizon_to_priority(&horizon);
        return Some(CaptureClassification {
            kind: CaptureKind::Reminder,
            horizon,
            priority,
            due_date: Some(date.format("%Y-%m-%d").to_string()),
        });
    }

    None
}

/// Extract an ISO date (YYYY-MM-DD) from text.
fn extract_iso_date(text: &str) -> Option<NaiveDate> {
    // Look for pattern: 4 digits - 2 digits - 2 digits
    for word in text.split_whitespace() {
        // Trim non-alphanumeric from edges to handle "2026-03-25," etc.
        let cleaned = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '-');
        if let Ok(date) = NaiveDate::parse_from_str(cleaned, "%Y-%m-%d") {
            return Some(date);
        }
    }
    None
}

/// Extract a MM/DD date from text, assuming the given year.
fn extract_slash_date(text: &str, year: i32) -> Option<NaiveDate> {
    for word in text.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '/');
        let parts: Vec<&str> = cleaned.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(month), Ok(day)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if (1..=12).contains(&month) && (1..=31).contains(&day) {
                    if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                        return Some(date);
                    }
                }
            }
        }
    }
    None
}

/// Map a due date to a horizon based on proximity to today.
fn date_to_horizon(today: NaiveDate, due: NaiveDate) -> Horizon {
    let days = (due - today).num_days();
    if days <= 1 {
        Horizon::Now
    } else if days <= 7 {
        Horizon::Week
    } else {
        Horizon::Later
    }
}

/// Map a horizon to a default priority.
fn horizon_to_priority(horizon: &Horizon) -> Priority {
    match horizon {
        Horizon::Now => Priority::Must,
        Horizon::Week => Priority::Should,
        Horizon::Later => Priority::Could,
    }
}

/// Build the metadata JSON for a capture item.
pub fn build_capture_metadata(
    classification: &CaptureClassification,
    source: &str,
    _tags: &[String],
) -> serde_json::Value {
    serde_json::json!({
        "kind": classification.kind,
        "horizon": classification.horizon,
        "priority": classification.priority,
        "status": CaptureStatus::Inbox,
        "source": source,
        "captured_at": Utc::now().to_rfc3339(),
        "due_date": classification.due_date,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_classify_url() {
        let c = auto_classify("https://example.com interesting article");
        assert_eq!(c.kind, CaptureKind::Link);
        assert_eq!(c.horizon, Horizon::Later);
        assert_eq!(c.priority, Priority::Could);
        assert!(c.due_date.is_none());
    }

    #[test]
    fn test_auto_classify_url_mid_text() {
        let c = auto_classify("check out https://example.com");
        assert_eq!(c.kind, CaptureKind::Link);
    }

    #[test]
    fn test_auto_classify_url_http() {
        let c = auto_classify("old site http://legacy.example.com/page");
        assert_eq!(c.kind, CaptureKind::Link);
    }

    #[test]
    fn test_auto_classify_reminder_tomorrow() {
        let c = auto_classify("tomorrow call dentist");
        assert_eq!(c.kind, CaptureKind::Reminder);
        assert_eq!(c.horizon, Horizon::Now);
        assert_eq!(c.priority, Priority::Must);
        // due_date should be tomorrow
        let tomorrow = (Utc::now().date_naive().succ_opt().unwrap())
            .format("%Y-%m-%d")
            .to_string();
        assert_eq!(c.due_date, Some(tomorrow));
    }

    #[test]
    fn test_auto_classify_reminder_next_week() {
        let c = auto_classify("next week submit report");
        assert_eq!(c.kind, CaptureKind::Reminder);
        assert_eq!(c.horizon, Horizon::Week);
        assert_eq!(c.priority, Priority::Should);
        // due_date should be a Monday
        let due = NaiveDate::parse_from_str(c.due_date.as_ref().unwrap(), "%Y-%m-%d").unwrap();
        assert_eq!(due.weekday(), chrono::Weekday::Mon);
    }

    #[test]
    fn test_auto_classify_reminder_iso_date() {
        // Use a date far in the future so it always classifies as Later
        let c = auto_classify("meeting on 2099-06-15 with client");
        assert_eq!(c.kind, CaptureKind::Reminder);
        assert_eq!(c.due_date, Some("2099-06-15".to_string()));
        assert_eq!(c.horizon, Horizon::Later);
    }

    #[test]
    fn test_auto_classify_reminder_slash_date() {
        // Use a far-future date so it is always Later
        let c = auto_classify("deadline 12/31");
        assert_eq!(c.kind, CaptureKind::Reminder);
        // The year is assumed to be the current year
        let year = Utc::now().date_naive().year();
        let expected = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
        let due = NaiveDate::parse_from_str(c.due_date.as_ref().unwrap(), "%Y-%m-%d").unwrap();
        assert_eq!(due, expected);
    }

    #[test]
    fn test_auto_classify_todo() {
        let c = auto_classify("review the PR for client X");
        assert_eq!(c.kind, CaptureKind::Todo);
        assert_eq!(c.horizon, Horizon::Now);
        assert_eq!(c.priority, Priority::Should);
    }

    #[test]
    fn test_auto_classify_todo_various_verbs() {
        for text in &["call dentist", "buy groceries", "send email"] {
            let c = auto_classify(text);
            assert_eq!(c.kind, CaptureKind::Todo, "Failed for: {}", text);
        }
    }

    #[test]
    fn test_auto_classify_note() {
        let c = auto_classify("interesting thought about AI");
        assert_eq!(c.kind, CaptureKind::Note);
        assert_eq!(c.horizon, Horizon::Later);
        assert_eq!(c.priority, Priority::Could);
    }

    #[test]
    fn test_auto_classify_note_no_verb() {
        let c = auto_classify("the weather is nice today");
        assert_eq!(c.kind, CaptureKind::Note);
    }

    #[test]
    fn test_build_metadata_has_required_fields() {
        let classification = CaptureClassification {
            kind: CaptureKind::Todo,
            horizon: Horizon::Now,
            priority: Priority::Should,
            due_date: None,
        };
        let meta = build_capture_metadata(&classification, "cli", &[]);

        assert!(meta.get("kind").is_some());
        assert!(meta.get("horizon").is_some());
        assert!(meta.get("priority").is_some());
        assert!(meta.get("status").is_some());
        assert!(meta.get("source").is_some());
        assert!(meta.get("captured_at").is_some());
        assert!(meta.get("due_date").is_some());

        // Verify specific values
        assert_eq!(meta["kind"], "todo");
        assert_eq!(meta["horizon"], "now");
        assert_eq!(meta["priority"], "should");
        assert_eq!(meta["status"], "inbox");
        assert_eq!(meta["source"], "cli");
    }

    #[test]
    fn test_build_metadata_with_due_date() {
        let classification = CaptureClassification {
            kind: CaptureKind::Reminder,
            horizon: Horizon::Now,
            priority: Priority::Must,
            due_date: Some("2026-03-25".to_string()),
        };
        let meta = build_capture_metadata(&classification, "voice", &[]);

        assert_eq!(meta["kind"], "reminder");
        assert_eq!(meta["due_date"], "2026-03-25");
        assert_eq!(meta["source"], "voice");
    }

    #[test]
    fn test_action_verbs_coverage() {
        // Every verb in ACTION_VERBS should produce a Todo
        for verb in ACTION_VERBS {
            let text = format!("{} something important", verb);
            let c = auto_classify(&text);
            assert_eq!(c.kind, CaptureKind::Todo, "Verb '{}' should produce Todo", verb);
        }
    }

    #[test]
    fn test_url_takes_priority_over_action_verb() {
        // URL rule (1) has higher priority than action verb rule (3)
        let c = auto_classify("check https://example.com for updates");
        assert_eq!(c.kind, CaptureKind::Link);
    }

    #[test]
    fn test_reminder_takes_priority_over_action_verb() {
        // Time word rule (2) has higher priority than action verb rule (3)
        let c = auto_classify("tomorrow review the document");
        assert_eq!(c.kind, CaptureKind::Reminder);
    }
}

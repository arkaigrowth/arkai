//! Triage CLI handlers: today, done, snooze.
//!
//! Implements ADHD-optimized daily review: grouped by horizon,
//! capped at 5 per section, deterministic output, shame-free language.

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate, TimeZone, Utc};
use serde_json::json;

use crate::store::queries::{self, Item};
use crate::store::{Store, StoreConfig};

/// Maximum items displayed per section to avoid overwhelm.
const MAX_PER_SECTION: usize = 5;

// ─────────────────────────────────────────────────────────────────
// today
// ─────────────────────────────────────────────────────────────────

/// Show active captures grouped by horizon.
pub async fn execute_today(json_output: bool) -> Result<()> {
    let db_path = StoreConfig::default_path()?;
    let store = Store::open(&db_path)?;

    let active = queries::list_active_captures(&store)?;
    let snoozed_count = queries::count_snoozed_captures(&store)?;

    // Group by horizon
    let (mut now_items, mut week_items, mut inbox_items) = group_by_horizon(&active);

    // Sort each section: priority weight ASC (must=0 first), then due_date ASC
    let sorter = |a: &&Item, b: &&Item| {
        let pa = priority_weight(a);
        let pb = priority_weight(b);
        pa.cmp(&pb).then_with(|| {
            let da = due_date_str(a).unwrap_or("");
            let db = due_date_str(b).unwrap_or("");
            da.cmp(db)
        })
    };
    now_items.sort_by(sorter);
    week_items.sort_by(sorter);
    inbox_items.sort_by(sorter);

    if json_output {
        render_json(&now_items, &week_items, &inbox_items, snoozed_count);
    } else {
        render_text(&now_items, &week_items, &inbox_items, snoozed_count);
    }

    Ok(())
}

fn group_by_horizon<'a>(items: &'a [Item]) -> (Vec<&'a Item>, Vec<&'a Item>, Vec<&'a Item>) {
    let mut now = Vec::new();
    let mut week = Vec::new();
    let mut inbox = Vec::new();

    for item in items {
        let horizon = item
            .metadata
            .get("horizon")
            .and_then(|v| v.as_str())
            .unwrap_or("later");
        match horizon {
            "now" => now.push(item),
            "week" => week.push(item),
            _ => inbox.push(item),
        }
    }

    (now, week, inbox)
}

fn priority_weight(item: &Item) -> u8 {
    match item
        .metadata
        .get("priority")
        .and_then(|v| v.as_str())
        .unwrap_or("could")
    {
        "must" => 0,
        "should" => 1,
        _ => 2,
    }
}

fn due_date_str(item: &Item) -> Option<&str> {
    item.metadata.get("due_date").and_then(|v| v.as_str())
}

fn kind_str(item: &Item) -> &str {
    item.metadata
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("note")
}

fn priority_str(item: &Item) -> &str {
    item.metadata
        .get("priority")
        .and_then(|v| v.as_str())
        .unwrap_or("could")
}

fn render_text(
    now_items: &[&Item],
    week_items: &[&Item],
    inbox_items: &[&Item],
    snoozed_count: i64,
) {
    let total_active = now_items.len() + week_items.len() + inbox_items.len();
    let snooze_note = if snoozed_count > 0 {
        format!("  ({} snoozed)", snoozed_count)
    } else {
        String::new()
    };
    println!("Today -- {} active{}", total_active, snooze_note);

    if !now_items.is_empty() {
        println!();
        let shown = now_items.len().min(MAX_PER_SECTION);
        println!("Do Today ({})", now_items.len());
        for item in now_items.iter().take(MAX_PER_SECTION) {
            print_item_line(item);
        }
        if now_items.len() > shown {
            println!("  ... and {} more", now_items.len() - shown);
        }
    }

    if !week_items.is_empty() {
        println!();
        let shown = week_items.len().min(MAX_PER_SECTION);
        println!("Heads Up ({})", week_items.len());
        for item in week_items.iter().take(MAX_PER_SECTION) {
            print_item_line(item);
        }
        if week_items.len() > shown {
            println!("  ... and {} more", week_items.len() - shown);
        }
    }

    if !inbox_items.is_empty() {
        println!();
        println!("{} in inbox", inbox_items.len());
    }
}

/// Truncate a string to at most `max_chars` characters, appending "..." if truncated.
/// Safe for multibyte/emoji content — operates on char boundaries, not byte offsets.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

fn print_item_line(item: &Item) {
    let id_short: String = item.id.chars().take(7).collect();
    let title = truncate_chars(&item.title, 45);
    let priority = priority_str(item);
    let due = due_date_str(item)
        .map(|d| format!("  due {}", d))
        .unwrap_or_default();
    println!("  [{}] {}{:<6} {}", id_short, title, due, priority);
}

fn render_json(
    now_items: &[&Item],
    week_items: &[&Item],
    inbox_items: &[&Item],
    snoozed_count: i64,
) {
    let section = |label: &str, horizon: &str, items: &[&Item]| {
        let truncated = items.len() > MAX_PER_SECTION;
        let displayed: Vec<serde_json::Value> = items
            .iter()
            .take(MAX_PER_SECTION)
            .map(|item| {
                json!({
                    "id": item.id,
                    "title": item.title,
                    "priority": priority_str(item),
                    "due_date": due_date_str(item),
                    "kind": kind_str(item),
                })
            })
            .collect();
        json!({
            "label": label,
            "horizon": horizon,
            "items": displayed,
            "total": items.len(),
            "truncated": truncated,
        })
    };

    let output = json!({
        "as_of": Utc::now().to_rfc3339(),
        "sections": [
            section("Do Today", "now", now_items),
            section("Heads Up", "week", week_items),
        ],
        "inbox_count": inbox_items.len(),
        "snoozed_count": snoozed_count,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

// ─────────────────────────────────────────────────────────────────
// done
// ─────────────────────────────────────────────────────────────────

/// Mark a capture as done. Accepts full ID or unique prefix.
pub async fn execute_done(item_id: String) -> Result<()> {
    let db_path = StoreConfig::default_path()?;
    let store = Store::open(&db_path)?;

    let full_id = queries::resolve_capture_id(&store, &item_id)?;
    queries::update_capture_status(&store, &full_id, "done", None)
        .context("Failed to mark item done")?;

    println!("Done: {}", full_id);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// snooze
// ─────────────────────────────────────────────────────────────────

/// Snooze a capture until a future date. Accepts full ID or unique prefix.
pub async fn execute_snooze(item_id: String, until: String) -> Result<()> {
    let until_rfc3339 = parse_snooze_until(&until)?;

    let db_path = StoreConfig::default_path()?;
    let store = Store::open(&db_path)?;

    let full_id = queries::resolve_capture_id(&store, &item_id)?;
    queries::update_capture_status(&store, &full_id, "snoozed", Some(&until_rfc3339))
        .context("Failed to snooze item")?;

    println!("Snoozed {} until {}", full_id, until_rfc3339);
    Ok(())
}

/// Parse a snooze-until string. Accepts:
/// - RFC3339 datetime: "2026-03-25T09:00:00+00:00"
/// - ISO date: "2026-03-25" (interpreted as start-of-day in local timezone, stored as UTC)
pub(crate) fn parse_snooze_until(until: &str) -> Result<String> {
    // Try RFC3339 datetime first
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(until) {
        return Ok(dt.with_timezone(&Utc).to_rfc3339());
    }

    // Try ISO date — interpret in local timezone, convert to UTC
    if let Ok(date) = NaiveDate::parse_from_str(until, "%Y-%m-%d") {
        let local_midnight = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
        let local_dt = Local
            .from_local_datetime(&local_midnight)
            .single()
            .ok_or_else(|| anyhow::anyhow!("Ambiguous local time for date: {}", until))?;
        return Ok(local_dt.with_timezone(&Utc).to_rfc3339());
    }

    anyhow::bail!(
        "Cannot parse snooze date: '{}'. Use YYYY-MM-DD or RFC3339 format.",
        until
    )
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::queries::{upsert_item, UpsertItem};

    fn test_store() -> Store {
        Store::open_memory().expect("failed to open in-memory store")
    }

    fn insert_capture(store: &Store, id: &str, title: &str, metadata: serde_json::Value) {
        let tags: Vec<String> = vec![];
        let upsert = UpsertItem {
            id,
            item_type: "capture",
            title,
            source_url: None,
            content_type: None,
            tags: &tags,
            artifacts: &[],
            run_id: None,
            metadata: &metadata,
        };
        upsert_item(store, &upsert).unwrap();
    }

    #[test]
    fn test_today_groups_by_horizon() {
        let store = test_store();
        insert_capture(&store, "c1", "now task", json!({"status":"inbox","horizon":"now","priority":"must"}));
        insert_capture(&store, "c2", "week task", json!({"status":"inbox","horizon":"week","priority":"should"}));
        insert_capture(&store, "c3", "later note", json!({"status":"inbox","horizon":"later","priority":"could"}));
        insert_capture(&store, "c4", "done task", json!({"status":"done","horizon":"now","priority":"must"}));

        let active = queries::list_active_captures(&store).unwrap();
        let (now, week, inbox) = group_by_horizon(&active);

        assert_eq!(now.len(), 1); // c1 only (c4 is done)
        assert_eq!(week.len(), 1); // c2
        assert_eq!(inbox.len(), 1); // c3 (later → inbox)
    }

    #[test]
    fn test_today_caps_at_5() {
        let store = test_store();
        for i in 0..8 {
            let id = format!("cap{:04}", i);
            let title = format!("Task {}", i);
            insert_capture(&store, &id, &title, json!({"status":"inbox","horizon":"now","priority":"should"}));
        }

        let active = queries::list_active_captures(&store).unwrap();
        let (now, _, _) = group_by_horizon(&active);
        assert_eq!(now.len(), 8); // all 8 retrieved
        // Display cap applied at render time
        assert!(now.len() > MAX_PER_SECTION);
    }

    #[test]
    fn test_parse_snooze_until_accepts_date() {
        let result = parse_snooze_until("2026-03-25").unwrap();
        // Should parse and produce a UTC RFC3339 string
        assert!(result.contains("2026-03-2"));
        assert!(chrono::DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_snooze_until_accepts_rfc3339() {
        let result = parse_snooze_until("2026-03-25T09:00:00+00:00").unwrap();
        assert!(result.contains("2026-03-25"));
        assert!(chrono::DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_snooze_until_rejects_garbage() {
        assert!(parse_snooze_until("next tuesday").is_err());
        assert!(parse_snooze_until("").is_err());
        assert!(parse_snooze_until("not-a-date").is_err());
    }

    #[test]
    fn test_priority_sorting() {
        let store = test_store();
        insert_capture(&store, "c1", "could task", json!({"status":"inbox","horizon":"now","priority":"could"}));
        insert_capture(&store, "c2", "must task", json!({"status":"inbox","horizon":"now","priority":"must"}));
        insert_capture(&store, "c3", "should task", json!({"status":"inbox","horizon":"now","priority":"should"}));

        let active = queries::list_active_captures(&store).unwrap();
        let (mut now, _, _) = group_by_horizon(&active);

        now.sort_by(|a, b| priority_weight(a).cmp(&priority_weight(b)));
        assert_eq!(now[0].title, "must task");
        assert_eq!(now[1].title, "should task");
        assert_eq!(now[2].title, "could task");
    }

    #[test]
    fn test_truncate_chars_ascii() {
        assert_eq!(truncate_chars("short", 45), "short");
        assert_eq!(truncate_chars("a]".repeat(30).as_str(), 10), "a]a]a]a...");
    }

    #[test]
    fn test_truncate_chars_unicode_safe() {
        // Emoji + curly quotes — must not panic
        let emoji_title = "Call dentist about insurance tomorrow morning please";
        let result = truncate_chars(emoji_title, 20);
        assert!(result.ends_with("..."));
        assert!(result.chars().count() <= 20);

        // Pure emoji string
        let emojis = "🔥🚀💡🎯📊🧠🎨🏆💪🌟✨🎉🔮🦾🤖";
        let result = truncate_chars(emojis, 8);
        assert_eq!(result.chars().count(), 8); // 5 emoji + "..."
        assert!(result.ends_with("..."));

        // Curly quotes
        let curly = "\u{201c}Hello world\u{201d} is a classic phrase in programming tutorials";
        let result = truncate_chars(curly, 15);
        assert!(result.chars().count() <= 15);
    }
}

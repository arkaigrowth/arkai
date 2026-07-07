//! Send-only notification adapter for voice memo approval pings.

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

use crate::adapters::{TelegramClient, TelegramConfig};
use crate::ingest::QueueItem;

/// Hard ceiling on one notify attempt.
pub(crate) const NOTIFY_TIMEOUT: Duration = Duration::from_secs(10);

/// What kind of notification this is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyKind {
    /// A genuinely-new memo just landed in `awaiting_approval`.
    NewAwaiting,

    /// Processing failed at a durable queue transition.
    Failed { reason: String },

    /// Successful processing or queue health needs human review.
    NeedsHuman { detail: String },
}

/// Send-only notifier.
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Returns `Ok(())` on a successful send; maps any send error into `Err`.
    async fn notify(&self, item: &QueueItem, kind: NotifyKind) -> Result<()>;
}

/// Telegram-backed notifier that reuses the existing send-only client.
pub struct TelegramNotifier {
    client: TelegramClient,
}

impl TelegramNotifier {
    pub fn new(client: TelegramClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Notifier for TelegramNotifier {
    async fn notify(&self, item: &QueueItem, kind: NotifyKind) -> Result<()> {
        match kind {
            NotifyKind::NewAwaiting => {
                let card = format_approval_card(item);
                self.client
                    .send_message(&card)
                    .await
                    .map_err(redact_send_error)?;
                Ok(())
            }
            NotifyKind::Failed { reason } => {
                let card = format_failed_card(item, &reason);
                self.client
                    .send_message(&card)
                    .await
                    .map_err(redact_send_error)?;
                Ok(())
            }
            NotifyKind::NeedsHuman { detail } => {
                let card = format_needs_human_card(item, &detail);
                self.client
                    .send_message(&card)
                    .await
                    .map_err(redact_send_error)?;
                Ok(())
            }
        }
    }
}

/// Re-wrap a `send_message` error into a fresh, source-less, token-free error.
pub(crate) fn redact_send_error(e: anyhow::Error) -> anyhow::Error {
    let display = scrub_bot_tokens(&e.to_string());
    // NEVER change this to `{e:#}` / `{e:?}`; alternate/debug formatting can
    // re-embed reqwest sources whose URL path contains the bot token.
    anyhow::anyhow!("telegram send failed: {display}")
}

/// Build the plain-text approval card.
pub(crate) fn format_approval_card(item: &QueueItem) -> String {
    format!(
        "🔔 New voice memo awaiting approval\n\
         \n\
         Name: {name}\n\
         Duration: {dur}\n\
         ID: {id}\n\
         \n\
         arkai voice approve {id}  |  arkai voice skip {id}",
        name = sanitize_file_name(&item.data.file_name),
        dur = format_duration(item.data.duration_seconds),
        id = item.id,
    )
}

pub(crate) fn format_failed_card(item: &QueueItem, reason: &str) -> String {
    format!(
        "❌ Voice memo processing failed\n\
         \n\
         Name: {name}\n\
         ID: {id}\n\
         Error: {err}\n\
         \n\
         arkai voice retry {id}",
        name = sanitize_file_name(&item.data.file_name),
        id = item.id,
        err = reason,
    )
}

pub(crate) fn format_needs_human_card(item: &QueueItem, detail: &str) -> String {
    format!(
        "⚠️ Needs review\n\
         \n\
         Name: {name}\n\
         ID: {id}\n\
         {detail}",
        name = sanitize_file_name(&item.data.file_name),
        id = item.id,
    )
}

pub(crate) fn sanitize_file_name(file_name: &str) -> String {
    file_name
        .replace(['\n', '\r'], " ")
        .chars()
        .take(80)
        .collect()
}

fn is_token_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

fn scrub_bot_tokens(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index..].starts_with(b"bot") {
            let mut cursor = index + 3;
            let digit_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }

            if cursor > digit_start && cursor < bytes.len() && bytes[cursor] == b':' {
                cursor += 1;
                let token_start = cursor;
                while cursor < bytes.len() && is_token_char(bytes[cursor]) {
                    cursor += 1;
                }

                if cursor > token_start {
                    out.push_str("bot<redacted>");
                    index = cursor;
                    continue;
                }
            }
        }

        let ch = input[index..]
            .chars()
            .next()
            .expect("valid UTF-8 character boundary");
        out.push(ch);
        index += ch.len_utf8();
    }

    out
}

/// Humanize optional seconds as `MmSSs`, or `unknown`.
pub(crate) fn format_duration(secs: Option<f32>) -> String {
    match secs {
        Some(s) => {
            let total = s.round() as u64;
            let m = total / 60;
            let sec = total % 60;
            format!("{m}m{sec:02}s")
        }
        None => "unknown".to_string(),
    }
}

/// Best-effort notify boundary.
pub async fn notify_best_effort(
    notifier: Option<&dyn Notifier>,
    item: &QueueItem,
    kind: NotifyKind,
) {
    notify_best_effort_inner(notifier, item, kind, NOTIFY_TIMEOUT).await;
}

async fn notify_best_effort_inner(
    notifier: Option<&dyn Notifier>,
    item: &QueueItem,
    kind: NotifyKind,
    timeout: Duration,
) {
    if let Some(n) = notifier {
        match tokio::time::timeout(timeout, n.notify(item, kind)).await {
            Ok(Ok(())) => {}
            // NEVER use `?e`, `{e:#}`, or `{e:?}` here. Display-only logging
            // preserves the source-less redaction boundary from `redact_send_error`.
            Ok(Err(e)) => tracing::warn!(item_id = %item.id, error = %e, "notify failed"),
            Err(_) => tracing::warn!(
                item_id = %item.id,
                timeout_secs = timeout.as_secs(),
                "notify timed out"
            ),
        }
    }
}

/// Parse `~/.arkai/telegram_token`: line 1 = bot token, line 2 = chat_id.
pub(crate) fn parse_token_file(contents: &str) -> Option<(String, String)> {
    let mut lines = contents.lines();
    let bot = lines.next().map(str::trim).filter(|s| !s.is_empty())?;
    let chat = lines.next().map(str::trim).filter(|s| !s.is_empty())?;
    Some((bot.to_string(), chat.to_string()))
}

/// Per-field precedence: env wins, file fallback.
pub(crate) fn resolve_credentials(
    env_token: Option<String>,
    env_chat: Option<String>,
    file_contents: Option<&str>,
) -> Option<TelegramConfig> {
    let file = file_contents.and_then(parse_token_file);
    let bot_token = env_token
        .filter(|s| !s.is_empty())
        .or_else(|| file.as_ref().map(|(t, _)| t.clone()))?;
    let chat_id = env_chat
        .filter(|s| !s.is_empty())
        .or_else(|| file.as_ref().map(|(_, c)| c.clone()))?;
    // TelegramConfig holds the bot token in plaintext. Never Debug/Serialize-log it.
    Some(TelegramConfig { bot_token, chat_id })
}

fn load_telegram_credentials() -> Option<TelegramConfig> {
    let env_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
    let env_chat = std::env::var("TELEGRAM_CHAT_ID").ok();
    let file_contents = crate::config::arkai_home()
        .ok()
        .map(|home| home.join("telegram_token"))
        .and_then(|p| std::fs::read_to_string(p).ok());

    resolve_credentials(env_token, env_chat, file_contents.as_deref())
}

/// Build the default notifier, or `None` if Telegram credentials are unconfigured.
pub fn build_default_notifier() -> Option<Box<dyn Notifier>> {
    match load_telegram_credentials() {
        Some(cfg) => Some(Box::new(TelegramNotifier::new(
            TelegramClient::from_config(cfg),
        ))),
        None => {
            tracing::warn!(
                "Telegram notifier not configured: set TELEGRAM_BOT_TOKEN and \
                 TELEGRAM_CHAT_ID or populate the ~/.arkai/telegram_token file; \
                 skipping approval notifications"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::Instant;

    use anyhow::Context;
    use chrono::Utc;

    use crate::domain::VoiceQueueStatus;
    use crate::ingest::queue::QueueItemData;

    fn sample_item() -> QueueItem {
        QueueItem {
            id: "a1b2c3d4e5f6".to_string(),
            status: VoiceQueueStatus::AwaitingApproval,
            data: QueueItemData {
                file_path: PathBuf::from("/tmp/Memo 3.m4a"),
                file_name: "Memo 3.m4a".to_string(),
                file_size: 12345,
                detected_at: Utc::now(),
                duration_seconds: Some(462.0),
                recorded_at: None,
            },
            started_at: None,
            completed_at: None,
            error: None,
            retry_count: 0,
            approved_once: false,
            chosen_engine: None,
            overrides: None,
            allow_large: false,
            decided_at: None,
            pid: None,
            proc_start_epoch: None,
            attempt_count: 0,
            auto_reset_count: 0,
        }
    }

    struct RecordingNotifier {
        calls: Mutex<Vec<(String, NotifyKind)>>,
    }

    #[async_trait]
    impl Notifier for RecordingNotifier {
        async fn notify(&self, item: &QueueItem, kind: NotifyKind) -> Result<()> {
            self.calls.lock().unwrap().push((item.id.clone(), kind));
            Ok(())
        }
    }

    struct FailingNotifier {
        attempts: AtomicUsize,
    }

    #[async_trait]
    impl Notifier for FailingNotifier {
        async fn notify(&self, _item: &QueueItem, _kind: NotifyKind) -> Result<()> {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            anyhow::bail!("send failed")
        }
    }

    struct SleepingNotifier;

    #[async_trait]
    impl Notifier for SleepingNotifier {
        async fn notify(&self, _item: &QueueItem, _kind: NotifyKind) -> Result<()> {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_notify_dispatch_new_awaiting() {
        let item = sample_item();
        let stub = RecordingNotifier {
            calls: Mutex::new(Vec::new()),
        };
        let notifier: Option<&dyn Notifier> = Some(&stub);

        notify_best_effort(notifier, &item, NotifyKind::NewAwaiting).await;

        let calls = stub.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], (item.id, NotifyKind::NewAwaiting));
    }

    #[test]
    fn test_format_approval_card_contains_fields() {
        let item = sample_item();

        let card = format_approval_card(&item);

        assert!(card.contains("Memo 3.m4a"));
        assert!(card.contains("7m42s"));
        assert!(card.contains(&item.id));
        assert!(card.contains(&format!("arkai voice approve {}", item.id)));
        assert!(card.contains(&format!("arkai voice skip {}", item.id)));
    }

    #[test]
    fn test_failed_card_contains_retry_and_reason() {
        let item = sample_item();
        let card = format_failed_card(&item, "transient: unavailable");

        assert!(card.contains("Voice memo processing failed"));
        assert!(card.contains("Memo 3.m4a"));
        assert!(card.contains("transient: unavailable"));
        assert!(card.contains(&format!("arkai voice retry {}", item.id)));
    }

    #[test]
    fn test_needs_human_card_contains_detail() {
        let item = sample_item();
        let card = format_needs_human_card(&item, "queue log has 2 unparseable line(s)");

        assert!(card.contains("Needs review"));
        assert!(card.contains("queue log has 2 unparseable line(s)"));
        assert!(card.contains(&item.id));
    }

    #[test]
    fn test_card_sanitizes_file_name() {
        let mut item = sample_item();
        item.data.file_name = format!("{}\nsecond line", "a".repeat(90));

        let card = format_failed_card(&item, "boom");
        let name_line = card
            .lines()
            .find(|line| line.starts_with("Name: "))
            .unwrap();

        assert_eq!(
            name_line.strip_prefix("Name: ").unwrap().chars().count(),
            80
        );
        assert!(!name_line.contains('\n'));
        assert!(!name_line.contains("second line"));
    }

    #[test]
    fn test_format_duration_humanizer() {
        assert_eq!(format_duration(Some(462.0)), "7m42s");
        assert_eq!(format_duration(Some(5.0)), "0m05s");
        assert_eq!(format_duration(None), "unknown");
    }

    #[test]
    fn test_parse_token_file_two_lines() {
        assert_eq!(
            parse_token_file("123:ABC\n98765\n"),
            Some(("123:ABC".to_string(), "98765".to_string()))
        );
        assert_eq!(parse_token_file("123:ABC\n"), None);
        assert_eq!(parse_token_file(""), None);
    }

    #[test]
    fn test_resolve_credentials_env_wins() {
        let cfg = resolve_credentials(
            Some("envtok".to_string()),
            Some("envchat".to_string()),
            Some("filetok\nfilechat"),
        )
        .unwrap();

        assert_eq!(cfg.bot_token, "envtok");
        assert_eq!(cfg.chat_id, "envchat");
    }

    #[test]
    fn test_resolve_credentials_file_fallback() {
        let cfg = resolve_credentials(None, None, Some("filetok\nfilechat")).unwrap();

        assert_eq!(cfg.bot_token, "filetok");
        assert_eq!(cfg.chat_id, "filechat");
    }

    #[test]
    fn test_resolve_credentials_none_when_unconfigured() {
        assert!(resolve_credentials(None, None, None).is_none());
        assert!(resolve_credentials(None, None, Some("")).is_none());
    }

    #[tokio::test]
    async fn test_notify_failure_does_not_propagate() {
        let item = sample_item();
        let stub = FailingNotifier {
            attempts: AtomicUsize::new(0),
        };
        let notifier: Option<&dyn Notifier> = Some(&stub);

        notify_best_effort(notifier, &item, NotifyKind::NewAwaiting).await;

        assert_eq!(stub.attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_notify_timeout_does_not_block() {
        let item = sample_item();
        let stub = SleepingNotifier;
        let notifier: Option<&dyn Notifier> = Some(&stub);
        let start = Instant::now();

        notify_best_effort_inner(
            notifier,
            &item,
            NotifyKind::NewAwaiting,
            Duration::from_millis(50),
        )
        .await;

        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_send_error_redacts_bot_token() {
        let sentinel = "SENTINEL_TOKEN_ABC123";
        let leaky = anyhow::anyhow!(
            "error sending request for url (https://api.telegram.org/bot123456:{sentinel}/sendMessage)"
        );
        let with_ctx = Err::<(), anyhow::Error>(leaky)
            .context("Failed to send Telegram message")
            .unwrap_err();
        let redacted = redact_send_error(with_ctx);

        assert!(!format!("{redacted}").contains(sentinel));
        assert!(!format!("{redacted:#}").contains(sentinel));
        assert!(!format!("{redacted:?}").contains(sentinel));

        let raw = anyhow::anyhow!(
            "error sending request for url (https://api.telegram.org/bot123456:{sentinel}/sendMessage)"
        );
        let raw_redacted = redact_send_error(raw);
        assert!(!format!("{raw_redacted}").contains(sentinel));
        assert!(!format!("{raw_redacted:#}").contains(sentinel));
        assert!(!format!("{raw_redacted:?}").contains(sentinel));
    }
}

//! Voice capture CLI commands.
//!
//! Commands for managing voice memo ingestion:
//! - `arkai voice status` - Show queue status
//! - `arkai voice scan` - Scan and queue files once
//! - `arkai voice watch` - Watch for new files continuously

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Subcommand;

use crate::adapters::TelegramClient;
use crate::ingest::{VoiceMemoWatcher, VoiceQueue, WatcherConfig};

/// Voice capture subcommands
#[derive(Subcommand, Debug)]
pub enum VoiceCommands {
    /// Show voice queue status
    Status,

    /// Scan Voice Memos directory and queue any new files
    Scan {
        /// Path to watch (defaults to Voice Memos directory)
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Watch for new voice memos (continuous mode)
    Watch {
        /// Process queue once and exit
        #[arg(long)]
        once: bool,

        /// Path to watch (defaults to Voice Memos directory)
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Process pending voice memos (send to Claudia via Telegram)
    Process {
        /// Process only one item and exit
        #[arg(long)]
        once: bool,

        /// Telegram bot token (or use TELEGRAM_BOT_TOKEN env)
        #[arg(long, env = "TELEGRAM_BOT_TOKEN")]
        bot_token: Option<String>,

        /// Telegram chat ID (or use TELEGRAM_CHAT_ID env)
        #[arg(long, env = "TELEGRAM_CHAT_ID")]
        chat_id: Option<String>,
    },

    /// List all items in the queue
    List {
        /// Filter by status (pending, processing, done, failed)
        #[arg(short, long)]
        status: Option<String>,

        /// Maximum number of items to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show configuration
    Config,
}

/// Execute a voice command
pub async fn execute(command: VoiceCommands) -> Result<()> {
    match command {
        VoiceCommands::Status => execute_status().await,
        VoiceCommands::Scan { path } => execute_scan(path).await,
        VoiceCommands::Watch { once, path } => execute_watch(once, path).await,
        VoiceCommands::Process { once, bot_token, chat_id } => {
            execute_process(once, bot_token, chat_id).await
        }
        VoiceCommands::List { status, limit } => execute_list(status, limit).await,
        VoiceCommands::Config => execute_config().await,
    }
}

/// Show queue status
async fn execute_status() -> Result<()> {
    let queue = VoiceQueue::open_default().await?;
    let status = queue.status().await.map_err(|e| anyhow::anyhow!("{}", e))?;

    let config = WatcherConfig::default();

    println!();
    println!("Voice Capture Queue Status");
    println!("══════════════════════════════════════════════════════════════");
    println!();
    println!("Watch path:  {}", config.watch_path.display());
    println!(
        "Queue file:  {}",
        VoiceQueue::default_path()?.display()
    );
    println!();
    println!("Queue:");
    println!("  Pending:    {}", status.pending);
    println!("  Processing: {}", status.processing);
    println!("  Done:       {}", status.done);
    println!("  Failed:     {}", status.failed);
    println!("  Total:      {}", status.total());
    println!();

    if !status.recent.is_empty() {
        println!("Recent:");
        for item in &status.recent {
            let status_str = match item.status {
                crate::domain::VoiceQueueStatus::Pending => "PEND",
                crate::domain::VoiceQueueStatus::Processing => "PROC",
                crate::domain::VoiceQueueStatus::Done => "DONE",
                crate::domain::VoiceQueueStatus::Failed => "FAIL",
            };
            println!(
                "  [{}] {} ({})",
                status_str,
                item.data.file_name,
                &item.id[..8]
            );
        }
    }

    println!();

    // Check if watch path exists
    if !config.watch_path.exists() {
        println!("⚠️  Watch path does not exist. Voice Memos may not be syncing to this Mac.");
        println!("    Expected: {}", config.watch_path.display());
    } else {
        println!("✓ Watch path exists");
    }

    Ok(())
}

/// Scan directory and queue files
async fn execute_scan(path: Option<String>) -> Result<()> {
    let mut config = WatcherConfig::default();
    if let Some(p) = path {
        config.watch_path = p.into();
    }

    println!("📂 Scanning: {}", config.watch_path.display());

    let watcher = VoiceMemoWatcher::with_config(config);
    let queue = VoiceQueue::open_default().await?;

    let result = watcher.scan_once(&queue).await?;

    println!();
    println!("Scan Results:");
    println!("  New files queued:    {}", result.new_files);
    println!("  Already queued:      {}", result.already_queued);
    println!("  Already processed:   {}", result.already_processed);
    println!("  Reset for retry:     {}", result.reset_for_retry);
    if result.errors > 0 {
        println!("  Errors:              {}", result.errors);
    }
    println!("  Total scanned:       {}", result.total_scanned());

    if result.new_files > 0 {
        println!();
        println!("✅ {} new file(s) added to queue", result.new_files);
    }

    Ok(())
}

/// Watch for new files
async fn execute_watch(once: bool, path: Option<String>) -> Result<()> {
    let mut config = WatcherConfig::default();
    if let Some(p) = path {
        config.watch_path = p.into();
    }

    let watcher = VoiceMemoWatcher::with_config(config.clone());
    let queue = Arc::new(VoiceQueue::open_default().await?);

    if once {
        // Just scan once and exit
        println!("📂 Scanning once: {}", config.watch_path.display());

        let result = watcher.scan_once(&queue).await?;

        if result.new_files > 0 {
            println!("✅ Queued {} new file(s)", result.new_files);
        } else {
            println!("ℹ️  No new files to queue");
        }

        return Ok(());
    }

    // Continuous watch mode
    println!("👁️  Watching: {}", config.watch_path.display());
    println!("    Press Ctrl+C to stop");
    println!();

    // Initial scan
    let initial = watcher.scan_once(&queue).await?;
    if initial.new_files > 0 {
        println!("📥 Initial scan: {} new file(s) queued", initial.new_files);
    }

    // Start watching
    let (mut event_rx, handle) = watcher.watch(queue).await?;

    // Set up Ctrl+C handler
    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        let _ = stop_tx.send(());
    });

    // Event loop
    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                println!(
                    "📥 New audio: {} ({})",
                    event.path.file_name().unwrap_or_default().to_string_lossy(),
                    &event.hash[..8]
                );
            }
            _ = &mut stop_rx => {
                println!();
                println!("🛑 Stopping watcher...");
                handle.stop().await?;
                break;
            }
        }
    }

    Ok(())
}

/// Process pending voice memos and send to Claudia via Telegram
async fn execute_process(
    once: bool,
    bot_token: Option<String>,
    chat_id: Option<String>,
) -> Result<()> {
    // Get credentials from args or env
    let bot_token = bot_token
        .or_else(|| std::env::var("TELEGRAM_BOT_TOKEN").ok())
        .context("Missing Telegram bot token. Set --bot-token or TELEGRAM_BOT_TOKEN env var")?;

    let chat_id = chat_id
        .or_else(|| std::env::var("TELEGRAM_CHAT_ID").ok())
        .context("Missing Telegram chat ID. Set --chat-id or TELEGRAM_CHAT_ID env var")?;

    let client = TelegramClient::new(bot_token, chat_id);
    let queue = VoiceQueue::open_default().await?;

    println!("🦞 Processing voice queue → Claudia");
    println!();

    loop {
        // Get pending items
        let pending = queue.get_pending().await?;

        if pending.is_empty() {
            if once {
                println!("✅ No pending items in queue");
                break;
            }
            // In continuous mode, wait and check again
            println!("⏳ Waiting for new items... (Ctrl+C to stop)");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            continue;
        }

        for item in pending {
            println!(
                "📤 Sending: {} ({})",
                item.data.file_name,
                &item.id[..8]
            );

            // Mark as processing
            queue.mark_processing(&item.id).await?;

            // Send to Telegram
            match client.send_voice_memo(&item.data.file_path).await {
                Ok(msg_id) => {
                    println!("   ✅ Sent! (message_id: {})", msg_id);
                    queue.mark_done(&item.id).await?;
                }
                Err(e) => {
                    println!("   ❌ Failed: {}", e);
                    queue.mark_failed(&item.id, &e.to_string()).await?;
                }
            }

            if once {
                return Ok(());
            }
        }

        if once {
            break;
        }

        // Small delay before checking for more
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    Ok(())
}

/// List queue items
async fn execute_list(status_filter: Option<String>, limit: usize) -> Result<()> {
    let queue = VoiceQueue::open_default().await?;
    let items = queue.replay().await.map_err(|e| anyhow::anyhow!("{}", e))?;

    // Filter and sort
    let mut filtered: Vec<_> = items
        .into_values()
        .filter(|item| {
            if let Some(ref filter) = status_filter {
                item.status.to_string() == *filter
            } else {
                true
            }
        })
        .collect();

    filtered.sort_by(|a, b| b.data.detected_at.cmp(&a.data.detected_at));

    if filtered.is_empty() {
        println!("No items in queue");
        if status_filter.is_some() {
            println!("  (filtered by status: {:?})", status_filter);
        }
        return Ok(());
    }

    println!();
    println!("{:<14} {:<8} {:<30} {:<20}", "ID", "STATUS", "FILE", "DETECTED");
    println!("{}", "-".repeat(75));

    for item in filtered.iter().take(limit) {
        let file_name = if item.data.file_name.len() > 28 {
            format!("{}...", &item.data.file_name[..25])
        } else {
            item.data.file_name.clone()
        };

        let detected = item.data.detected_at.format("%Y-%m-%d %H:%M:%S");

        println!(
            "{:<14} {:<8} {:<30} {:<20}",
            &item.id[..12],
            item.status.to_string(),
            file_name,
            detected
        );
    }

    let total = filtered.len();
    if total > limit {
        println!();
        println!("  (showing {} of {} items)", limit, total);
    }

    Ok(())
}

/// Show configuration
async fn execute_config() -> Result<()> {
    let config = WatcherConfig::default();

    println!();
    println!("Voice Capture Configuration");
    println!("══════════════════════════════════════════════════════════════");
    println!();
    println!("Watch path:       {}", config.watch_path.display());
    println!("Stability delay:  {} seconds", config.stability_delay_secs);
    println!("Extensions:       {:?}", config.extensions);
    println!();
    println!("Queue file:       {}", VoiceQueue::default_path()?.display());
    println!();

    // Check if path exists
    if config.watch_path.exists() {
        println!("✓ Watch path exists");

        // Count files
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(&config.watch_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.path().extension().map(|e| e == "m4a").unwrap_or(false) {
                count += 1;
            }
        }
        println!("  {} .m4a file(s) in directory", count);
    } else {
        println!("⚠️  Watch path does not exist");
        println!();
        println!("Voice Memos may not be syncing to this Mac.");
        println!("To enable, open Voice Memos on your iPhone and ensure");
        println!("iCloud sync is enabled in Settings → Voice Memos.");
    }

    Ok(())
}

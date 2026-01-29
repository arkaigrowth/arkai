//! Voice capture CLI commands.
//!
//! Commands for managing voice memo ingestion:
//! - `arkai voice status` - Show queue status
//! - `arkai voice scan` - Scan and queue files once
//! - `arkai voice watch` - Watch for new files continuously

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Subcommand;

use crate::adapters::{ClawdbotClient, TelegramClient};
use crate::ingest::{transcribe, VoiceMemoWatcher, VoiceQueue, WatcherConfig};

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

    /// Process pending voice memos (send to Claudia via Telegram or Clawdbot)
    Process {
        /// Process only one item and exit
        #[arg(long)]
        once: bool,

        /// Route: "telegram" (send raw audio) or "clawdbot" (transcribe + send text)
        #[arg(long, default_value = "telegram")]
        route: String,

        /// Whisper model for transcription (clawdbot route only)
        #[arg(long, default_value = "base")]
        model: String,

        /// Telegram bot token (or use TELEGRAM_BOT_TOKEN env) - telegram route only
        #[arg(long, env = "TELEGRAM_BOT_TOKEN")]
        bot_token: Option<String>,

        /// Telegram chat ID (or use TELEGRAM_CHAT_ID env) - telegram route only
        #[arg(long, env = "TELEGRAM_CHAT_ID")]
        chat_id: Option<String>,

        /// Stop after processing N items (safety cap)
        #[arg(long)]
        limit: Option<u32>,

        /// Stop after processing H hours of audio (cumulative, safety cap)
        #[arg(long)]
        max_hours: Option<f32>,

        /// Show what would be processed without actually processing
        #[arg(long)]
        dry_run: bool,
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
        VoiceCommands::Process { once, route, model, bot_token, chat_id, limit, max_hours, dry_run } => {
            execute_process(once, &route, &model, bot_token, chat_id, limit, max_hours, dry_run).await
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
    if result.deferred > 0 {
        println!("  Deferred (syncing):  {}", result.deferred);
    }
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

/// Safety caps for processing
struct ProcessCaps {
    limit: Option<u32>,
    max_hours: Option<f32>,
    dry_run: bool,
}

/// Process pending voice memos and send to Claudia
async fn execute_process(
    once: bool,
    route: &str,
    model: &str,
    bot_token: Option<String>,
    chat_id: Option<String>,
    limit: Option<u32>,
    max_hours: Option<f32>,
    dry_run: bool,
) -> Result<()> {
    let queue = VoiceQueue::open_default().await?;
    let caps = ProcessCaps { limit, max_hours, dry_run };

    // Handle dry-run mode
    if dry_run {
        return execute_dry_run(&queue, &caps).await;
    }

    match route {
        "telegram" => execute_process_telegram(once, bot_token, chat_id, &queue, &caps).await,
        "clawdbot" => execute_process_clawdbot(once, model, chat_id.as_deref(), &queue, &caps).await,
        _ => anyhow::bail!("Unknown route: {}. Use 'telegram' or 'clawdbot'", route),
    }
}

/// Execute dry-run: show what would be processed
async fn execute_dry_run(queue: &VoiceQueue, caps: &ProcessCaps) -> Result<()> {
    let pending = queue.get_pending().await?;

    if pending.is_empty() {
        println!("✓ No pending items to process");
        return Ok(());
    }

    println!();
    println!("Dry Run - Would process:");
    println!("══════════════════════════════════════════════════════════════");
    println!();
    println!("{:<14} {:<30} {:<6} {:<10} {:<12}", "ID", "FILE", "EXT", "DURATION", "SIZE");
    println!("{}", "-".repeat(75));

    let mut count = 0u32;
    let mut total_duration = 0.0f32;
    let mut total_size = 0u64;

    for item in &pending {
        // Check limit cap
        if let Some(limit) = caps.limit {
            if count >= limit {
                break;
            }
        }

        // Check max-hours cap
        let duration = item.data.duration_seconds.unwrap_or(0.0);
        if let Some(max_hours) = caps.max_hours {
            if total_duration / 3600.0 >= max_hours {
                break;
            }
        }

        // Get file extension
        let ext = std::path::Path::new(&item.data.file_name)
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_else(|| "-".to_string());

        // Format file name (truncate if too long)
        let file_name = if item.data.file_name.len() > 28 {
            format!("{}...", &item.data.file_name[..25])
        } else {
            item.data.file_name.clone()
        };

        // Format duration
        let duration_str = if duration > 0.0 {
            format!("{:.1}s", duration)
        } else {
            "?".to_string()
        };

        // Format size
        let size_str = format_size(item.data.file_size);

        println!(
            "{:<14} {:<30} {:<6} {:<10} {:<12}",
            &item.id[..12],
            file_name,
            ext,
            duration_str,
            size_str
        );

        count += 1;
        total_duration += duration;
        total_size += item.data.file_size;
    }

    println!("{}", "-".repeat(75));
    println!();
    println!("Summary:");
    println!("  Items:    {}", count);
    println!("  Duration: {:.1} minutes ({:.2} hours)", total_duration / 60.0, total_duration / 3600.0);
    println!("  Size:     {}", format_size(total_size));

    if caps.limit.is_some() || caps.max_hours.is_some() {
        println!();
        println!("Caps applied:");
        if let Some(limit) = caps.limit {
            println!("  --limit {}", limit);
        }
        if let Some(max_hours) = caps.max_hours {
            println!("  --max-hours {}", max_hours);
        }
    }

    let remaining = pending.len() - count as usize;
    if remaining > 0 {
        println!();
        println!("Note: {} more item(s) would not be processed due to caps", remaining);
    }

    println!();

    Ok(())
}

/// Format file size in human-readable form
fn format_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Process via Telegram (send raw audio)
async fn execute_process_telegram(
    once: bool,
    bot_token: Option<String>,
    chat_id: Option<String>,
    queue: &VoiceQueue,
    caps: &ProcessCaps,
) -> Result<()> {
    // Get credentials from args or env
    let bot_token = bot_token
        .or_else(|| std::env::var("TELEGRAM_BOT_TOKEN").ok())
        .context("Missing Telegram bot token. Set --bot-token or TELEGRAM_BOT_TOKEN env var")?;

    let chat_id = chat_id
        .or_else(|| std::env::var("TELEGRAM_CHAT_ID").ok())
        .context("Missing Telegram chat ID. Set --chat-id or TELEGRAM_CHAT_ID env var")?;

    let client = TelegramClient::new(bot_token, chat_id);

    println!("🦞 Processing voice queue → Claudia (Telegram)");
    if caps.limit.is_some() || caps.max_hours.is_some() {
        print!("   Caps: ");
        if let Some(limit) = caps.limit {
            print!("--limit {} ", limit);
        }
        if let Some(max_hours) = caps.max_hours {
            print!("--max-hours {} ", max_hours);
        }
        println!();
    }
    println!();

    let mut processed_count = 0u32;
    let mut total_duration = 0.0f32;

    loop {
        let pending = queue.get_pending().await?;

        if pending.is_empty() {
            if once {
                println!("✅ No pending items in queue");
                break;
            }
            println!("⏳ Waiting for new items... (Ctrl+C to stop)");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            continue;
        }

        for item in pending {
            // Check limit cap
            if let Some(limit) = caps.limit {
                if processed_count >= limit {
                    println!("⛔ Reached --limit {} cap", limit);
                    return Ok(());
                }
            }

            // Check max-hours cap
            let item_duration = item.data.duration_seconds.unwrap_or(0.0);
            if let Some(max_hours) = caps.max_hours {
                if total_duration / 3600.0 >= max_hours {
                    println!("⛔ Reached --max-hours {} cap ({:.1} min processed)", max_hours, total_duration / 60.0);
                    return Ok(());
                }
            }

            println!(
                "📤 Sending: {} ({})",
                item.data.file_name,
                &item.id[..8]
            );

            queue.mark_processing(&item.id).await?;

            match client.send_voice_memo(&item.data.file_path).await {
                Ok(msg_id) => {
                    println!("   ✅ Sent! (message_id: {})", msg_id);
                    queue.mark_done(&item.id).await?;
                    processed_count += 1;
                    total_duration += item_duration;
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

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    Ok(())
}

/// Process via Clawdbot (transcribe locally, send text to VPS)
async fn execute_process_clawdbot(
    once: bool,
    model: &str,
    telegram_chat_id: Option<&str>,
    queue: &VoiceQueue,
    caps: &ProcessCaps,
) -> Result<()> {
    let client = ClawdbotClient::from_env()
        .context("Clawdbot client setup failed. Set CLAWDBOT_TOKEN env var")?;

    // Optionally deliver to Telegram as well
    let deliver_to_telegram = telegram_chat_id.is_some();

    println!("🦞 Processing voice queue → Claudia (Clawdbot)");
    println!("   Model: {}", model);
    if deliver_to_telegram {
        println!("   Telegram delivery: enabled");
    }
    if caps.limit.is_some() || caps.max_hours.is_some() {
        print!("   Caps: ");
        if let Some(limit) = caps.limit {
            print!("--limit {} ", limit);
        }
        if let Some(max_hours) = caps.max_hours {
            print!("--max-hours {} ", max_hours);
        }
        println!();
    }
    println!();

    let mut processed_count = 0u32;
    let mut total_duration = 0.0f32;

    loop {
        let pending = queue.get_pending().await?;

        if pending.is_empty() {
            if once {
                println!("✅ No pending items in queue");
                break;
            }
            println!("⏳ Waiting for new items... (Ctrl+C to stop)");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            continue;
        }

        for item in pending {
            // Check limit cap
            if let Some(limit) = caps.limit {
                if processed_count >= limit {
                    println!("⛔ Reached --limit {} cap", limit);
                    return Ok(());
                }
            }

            // Check max-hours cap
            let item_duration = item.data.duration_seconds.unwrap_or(0.0);
            if let Some(max_hours) = caps.max_hours {
                if total_duration / 3600.0 >= max_hours {
                    println!("⛔ Reached --max-hours {} cap ({:.1} min processed)", max_hours, total_duration / 60.0);
                    return Ok(());
                }
            }

            println!(
                "🎙️  Processing: {} ({})",
                item.data.file_name,
                &item.id[..8]
            );

            queue.mark_processing(&item.id).await?;

            // Step 1: Transcribe locally
            println!("   📝 Transcribing with Whisper ({})...", model);
            let audio_path = std::path::PathBuf::from(&item.data.file_path);

            let transcript = match transcribe(&audio_path, model).await {
                Ok(t) => {
                    println!("   ✅ Transcribed ({:.0}s, {} chars)", t.duration_seconds, t.text.len());
                    t
                }
                Err(e) => {
                    println!("   ❌ Transcription failed: {}", e);
                    queue.mark_failed(&item.id, &format!("Transcription failed: {}", e)).await?;
                    if once {
                        return Ok(());
                    }
                    continue;
                }
            };

            // Step 2: Send to Clawdbot
            println!("   📤 Sending to Claudia...");
            match client
                .send_voice_intake(
                    &transcript.text,
                    &item.id,
                    transcript.duration_seconds,
                    deliver_to_telegram,
                    telegram_chat_id,
                )
                .await
            {
                Ok(_resp) => {
                    println!("   ✅ Sent to Claudia!");
                    queue.mark_done(&item.id).await?;
                    processed_count += 1;
                    total_duration += item_duration;
                }
                Err(e) => {
                    println!("   ❌ Failed to send: {}", e);
                    queue.mark_failed(&item.id, &format!("Clawdbot send failed: {}", e)).await?;
                }
            }

            if once {
                return Ok(());
            }
        }

        if once {
            break;
        }

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

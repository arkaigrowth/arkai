//! Local `transcribe-memo` processing route for approved voice queue items.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::io::AsyncReadExt;

use crate::adapters::{notify_best_effort, Notifier, NotifyKind};
use crate::domain::VoiceQueueStatus;
use crate::ingest::{QueueItem, VoiceQueue};

const DEFAULT_TRANSCRIBE_MEMO_BIN: &str = "/Users/alexkamysz/bin/transcribe-memo";
const PER_ITEM_MAX_SECS: f32 = 6.0 * 60.0 * 60.0;
const MIN_HARD_TIMEOUT_SECS: u64 = 2 * 60 * 60;

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalRouteCaps {
    pub limit: Option<u32>,
    pub max_hours: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitAction {
    Done,
    DoneNeedsHuman(String),
    Failed(String),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ArkaiResult {
    #[serde(default)]
    pub needs_human: bool,

    #[serde(default)]
    pub detail: Option<String>,

    #[serde(default)]
    pub error: Option<String>,

    #[serde(default)]
    pub message: Option<String>,

    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug)]
struct ChildRunResult {
    exit_code: Option<i32>,
    stdout: String,
}

pub async fn execute_process_local(
    once: bool,
    queue: &VoiceQueue,
    caps: &LocalRouteCaps,
    notifier: Option<&dyn Notifier>,
) -> Result<()> {
    execute_process_local_with_bin(once, queue, caps, notifier, None).await
}

async fn execute_process_local_with_bin(
    once: bool,
    queue: &VoiceQueue,
    caps: &LocalRouteCaps,
    notifier: Option<&dyn Notifier>,
    bin_override: Option<&Path>,
) -> Result<()> {
    if let Err(error) = alert_on_corrupt_queue_if_needed(queue, notifier).await {
        tracing::warn!("corrupt queue alert check failed: {}", error);
    }

    let reap_report = queue
        .reap_stale_processing()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    for item in reap_report.quarantined {
        notify_best_effort(
            notifier,
            &item,
            NotifyKind::NeedsHuman {
                detail: "reaped at retry cap".to_string(),
            },
        )
        .await;
    }

    println!("🎙️  Processing voice queue → local transcribe-memo");
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

    let mut attempted_count = 0u32;
    let mut total_duration = 0.0f32;

    loop {
        let pending = queue
            .get_pending()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        if pending.is_empty() {
            if once {
                println!("✅ No pending items in queue");
                break;
            }
            println!("⏳ Waiting for new items... (Ctrl+C to stop)");
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        for item in pending {
            if let Some(limit) = caps.limit {
                if attempted_count >= limit {
                    println!("⛔ Reached --limit {} cap", limit);
                    return Ok(());
                }
            }

            let item_duration = item.data.duration_seconds.unwrap_or(0.0);
            if let Some(max_hours) = caps.max_hours {
                if total_duration / 3600.0 >= max_hours {
                    println!(
                        "⛔ Reached --max-hours {} cap ({:.1} min processed)",
                        max_hours,
                        total_duration / 60.0
                    );
                    return Ok(());
                }
            }

            println!(
                "🎙️  Processing: {} ({})",
                item.data.file_name,
                &item.id[..8]
            );

            if let Err(error) = process_one_item(queue, &item, notifier, bin_override, None).await {
                let reason = error.to_string();
                tracing::warn!(item_id = %item.id, error = %reason, "local route item failed");
                mark_failed_and_notify(queue, &item, &reason, notifier).await;
            }

            attempted_count += 1;
            total_duration += item_duration;

            if caps
                .limit
                .map(|limit| attempted_count >= limit)
                .unwrap_or(false)
            {
                return Ok(());
            }

            if once {
                return Ok(());
            }
        }

        if once {
            break;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}

async fn process_one_item(
    queue: &VoiceQueue,
    item: &QueueItem,
    notifier: Option<&dyn Notifier>,
    bin_override: Option<&Path>,
    timeout_override: Option<Duration>,
) -> Result<()> {
    let input = match resolve_input_path(item).await {
        Ok(Some(path)) => path,
        Ok(None) => {
            mark_failed_and_notify(queue, item, "source unreadable (moved? FDA?)", notifier).await;
            return Ok(());
        }
        Err(error) => {
            mark_failed_and_notify(queue, item, &error.to_string(), notifier).await;
            return Ok(());
        }
    };

    if item.data.duration_seconds.unwrap_or(0.0) > PER_ITEM_MAX_SECS && !item.allow_large {
        mark_failed_and_notify(
            queue,
            item,
            "over 6h cap; re-approve with --allow-large",
            notifier,
        )
        .await;
        return Ok(());
    }

    if let Err(error) = queue.mark_processing_with_attempt(&item.id).await {
        mark_failed_and_notify(queue, item, &error.to_string(), notifier).await;
        return Ok(());
    }

    let timeout = timeout_override.unwrap_or_else(|| hard_timeout(item.data.duration_seconds));
    let args = transcribe_memo_args(item, &input);
    let bin = bin_override
        .map(Path::to_path_buf)
        .unwrap_or_else(transcribe_memo_bin);
    let run_result = match run_transcribe_memo(&bin, &args, timeout).await {
        Ok(output) => output,
        Err(error) => {
            mark_failed_and_notify(queue, item, &error.to_string(), notifier).await;
            return Ok(());
        }
    };

    match classify_exit(run_result.exit_code, &run_result.stdout) {
        ExitAction::Done => {
            queue
                .mark_done(&item.id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        ExitAction::DoneNeedsHuman(detail) => {
            queue
                .mark_done(&item.id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            notify_best_effort(notifier, item, NotifyKind::NeedsHuman { detail }).await;
        }
        ExitAction::Failed(reason) => {
            mark_failed_and_notify(queue, item, &reason, notifier).await;
        }
    }

    Ok(())
}

async fn mark_failed_and_notify(
    queue: &VoiceQueue,
    item: &QueueItem,
    reason: &str,
    notifier: Option<&dyn Notifier>,
) {
    match queue.mark_failed(&item.id, reason).await {
        Ok(()) => {
            notify_best_effort(
                notifier,
                item,
                NotifyKind::Failed {
                    reason: reason.to_string(),
                },
            )
            .await;
        }
        Err(error) => {
            tracing::warn!(
                item_id = %item.id,
                error = %error,
                "failed to append local route failure"
            );
        }
    }
}

async fn resolve_input_path(item: &QueueItem) -> Result<Option<PathBuf>> {
    let cache_dir = crate::config::voice_cache_dir()?;
    resolve_input_path_in_cache(item, &cache_dir).await
}

async fn resolve_input_path_in_cache(
    item: &QueueItem,
    cache_dir: &Path,
) -> Result<Option<PathBuf>> {
    let ext = item
        .data
        .file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default();
    let cache_path = cache_dir.join(format!("{}{}", item.id, ext));

    if is_readable_file(&cache_path).await {
        return Ok(Some(cache_path));
    }

    if is_readable_file(&item.data.file_path).await {
        return Ok(Some(item.data.file_path.clone()));
    }

    Ok(None)
}

async fn is_readable_file(path: &Path) -> bool {
    match tokio::fs::File::open(path).await {
        Ok(file) => file.metadata().await.map(|m| m.is_file()).unwrap_or(false),
        Err(_) => false,
    }
}

fn transcribe_memo_bin() -> PathBuf {
    std::env::var_os("ARKAI_TRANSCRIBE_MEMO_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TRANSCRIBE_MEMO_BIN))
}

fn transcribe_memo_args(item: &QueueItem, input: &Path) -> Vec<OsString> {
    let mut args = vec![
        input.as_os_str().to_os_string(),
        OsString::from("--stem"),
        OsString::from(file_stem(&item.data.file_name)),
    ];

    if let Some(recorded_at) = item.data.recorded_at {
        args.push(OsString::from("--when"));
        args.push(OsString::from(recorded_at.to_rfc3339()));
    }

    if let Some(engine) = &item.chosen_engine {
        args.push(OsString::from("--engine"));
        args.push(OsString::from(engine));
    }

    if let Some(overrides) = item.overrides.as_ref().and_then(|value| value.as_object()) {
        if let Some(speakers) = overrides.get("speakers").and_then(|value| value.as_u64()) {
            args.push(OsString::from("--speakers"));
            args.push(OsString::from(speakers.to_string()));
        }
        if let Some(names) = overrides.get("names").and_then(|value| value.as_str()) {
            args.push(OsString::from("--names"));
            args.push(OsString::from(names));
        }
        if let Some(category) = overrides.get("category").and_then(|value| value.as_str()) {
            args.push(OsString::from("--category"));
            args.push(OsString::from(category));
        }
        if let Some(hint) = overrides.get("hint").and_then(|value| value.as_str()) {
            args.push(OsString::from("--context"));
            args.push(OsString::from(hint));
        }
    }

    args
}

fn file_stem(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string())
}

fn hard_timeout(duration_seconds: Option<f32>) -> Duration {
    let scaled = duration_seconds.unwrap_or(0.0).max(0.0) * 3.0;
    Duration::from_secs(MIN_HARD_TIMEOUT_SECS.max(scaled.ceil() as u64))
}

async fn run_transcribe_memo(
    bin: &Path,
    args: &[OsString],
    timeout: Duration,
) -> Result<ChildRunResult> {
    use std::os::unix::process::CommandExt;

    let mut command = tokio::process::Command::new(bin);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    command.as_std_mut().process_group(0);

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn {}", bin.display()))?;
    let child_pid = child
        .id()
        .context("spawned transcribe-memo child has no pid")?;
    let mut stdout = child
        .stdout
        .take()
        .context("spawned transcribe-memo child has no stdout pipe")?;
    let stdout_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).await.map(|_| bytes)
    });

    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(wait_result) => wait_result?,
        Err(_) => {
            unsafe {
                libc::kill(-(child_pid as i32), libc::SIGKILL);
            }
            let _ = child.wait().await;
            let _ = stdout_task.await;
            anyhow::bail!("hard timeout after {}", format_timeout_hours(timeout));
        }
    };

    let stdout_bytes = stdout_task.await.context("stdout reader task failed")??;
    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();

    Ok(ChildRunResult {
        exit_code: status.code(),
        stdout,
    })
}

fn format_timeout_hours(timeout: Duration) -> String {
    let hours = timeout.as_secs_f64() / 3600.0;
    if (hours.fract()).abs() < f64::EPSILON {
        format!("{hours:.0}h")
    } else {
        format!("{hours:.1}h")
    }
}

pub fn parse_last_arkai_result(stdout: &str) -> Option<ArkaiResult> {
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix("@@ARKAI_RESULT@@"))
        .filter_map(|tail| serde_json::from_str::<ArkaiResult>(tail.trim()).ok())
        .last()
}

fn result_summary(result: Option<&ArkaiResult>) -> String {
    result
        .and_then(|result| {
            result
                .error
                .as_deref()
                .or(result.reason.as_deref())
                .or(result.detail.as_deref())
                .or(result.message.as_deref())
        })
        .unwrap_or("no @@ARKAI_RESULT@@ line")
        .to_string()
}

pub fn classify_exit(exit_code: Option<i32>, stdout: &str) -> ExitAction {
    let result = parse_last_arkai_result(stdout);
    match exit_code {
        Some(0) => {
            if result
                .as_ref()
                .map(|result| result.needs_human)
                .unwrap_or(false)
            {
                // The audio sentinel carries needs_human but usually no detail
                // field — don't surface the "no result line" fallback here.
                let detail = result
                    .as_ref()
                    .and_then(|result| {
                        result
                            .detail
                            .as_deref()
                            .or(result.message.as_deref())
                            .or(result.reason.as_deref())
                            .or(result.error.as_deref())
                    })
                    .unwrap_or("review flagged needs_human — check the transcript's review notes")
                    .to_string();
                ExitAction::DoneNeedsHuman(detail)
            } else {
                ExitAction::Done
            }
        }
        Some(3) => ExitAction::Done,
        Some(4) => ExitAction::Failed(format!("transient: {}", result_summary(result.as_ref()))),
        Some(code) => {
            ExitAction::Failed(format!("exit {code}: {}", result_summary(result.as_ref())))
        }
        None => ExitAction::Failed(format!("exit signal: {}", result_summary(result.as_ref()))),
    }
}

pub async fn alert_on_corrupt_queue_if_needed(
    queue: &VoiceQueue,
    notifier: Option<&dyn Notifier>,
) -> Result<()> {
    let (_, stats) = queue
        .replay_with_stats()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let skipped = stats.skipped_lines;
    if skipped == 0 {
        return Ok(());
    }

    let mark_path = crate::config::arkai_home()?.join("voice_queue.corrupt");
    let previous = tokio::fs::read_to_string(&mark_path)
        .await
        .ok()
        .and_then(|contents| contents.trim().parse::<usize>().ok())
        .unwrap_or(0);

    if skipped <= previous {
        return Ok(());
    }

    let detail = format!("queue log has {skipped} unparseable line(s) — inspect voice_queue.jsonl");
    let item = synthetic_alert_item("voice_queue", "voice_queue.jsonl");
    notify_best_effort(notifier, &item, NotifyKind::NeedsHuman { detail }).await;

    if let Some(parent) = mark_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(mark_path, skipped.to_string()).await?;

    Ok(())
}

fn synthetic_alert_item(id: &str, file_name: &str) -> QueueItem {
    QueueItem {
        id: id.to_string(),
        status: VoiceQueueStatus::Failed,
        data: crate::ingest::queue::QueueItemData {
            file_path: PathBuf::from(file_name),
            file_name: file_name.to_string(),
            file_size: 0,
            detected_at: chrono::Utc::now(),
            duration_seconds: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Mutex;
    use std::time::Instant;

    use async_trait::async_trait;
    use chrono::Utc;
    use tempfile::TempDir;

    use crate::ingest::queue::{ApprovalDecision, QueueItemData};

    fn sample_item_with_path(path: PathBuf) -> QueueItem {
        QueueItem {
            id: "a1b2c3d4e5f6".to_string(),
            status: VoiceQueueStatus::Pending,
            data: QueueItemData {
                file_path: path,
                file_name: "Memo 3.m4a".to_string(),
                file_size: 10,
                detected_at: Utc::now(),
                duration_seconds: Some(10.0),
                recorded_at: None,
            },
            started_at: None,
            completed_at: None,
            error: None,
            retry_count: 0,
            approved_once: true,
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

    async fn write_executable(path: &Path, contents: &str) {
        tokio::fs::write(path, contents).await.unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    async fn enqueue_approved(queue: &VoiceQueue, audio: &Path) -> (String, QueueItem) {
        let id = queue
            .enqueue(audio, 5, Utc::now())
            .await
            .unwrap()
            .id()
            .to_string();
        queue
            .approve(&id, ApprovalDecision::default())
            .await
            .unwrap();
        let item = queue.get(&id).await.unwrap().unwrap();
        (id, item)
    }

    #[test]
    fn test_sentinel_parse_last_line_wins_and_tolerates_garbage() {
        let stdout = concat!(
            "noise\n",
            "@@ARKAI_RESULT@@ {\"message\":\"first\"}\n",
            "@@ARKAI_RESULT@@ not-json\n",
            "@@ARKAI_RESULT@@ {\"needs_human\":true,\"detail\":\"second\"}\n",
        );

        let result = parse_last_arkai_result(stdout).unwrap();

        assert!(result.needs_human);
        assert_eq!(result.detail.as_deref(), Some("second"));
    }

    #[test]
    fn test_exit_code_mapping() {
        assert_eq!(classify_exit(Some(0), "{}"), ExitAction::Done);
        assert_eq!(classify_exit(Some(3), ""), ExitAction::Done);
        assert_eq!(
            classify_exit(Some(4), "@@ARKAI_RESULT@@ {\"error\":\"try later\"}"),
            ExitAction::Failed("transient: try later".to_string())
        );
        assert_eq!(
            classify_exit(Some(7), "@@ARKAI_RESULT@@ {\"message\":\"bad\"}"),
            ExitAction::Failed("exit 7: bad".to_string())
        );
    }

    #[test]
    fn test_needs_human_exit_zero_maps_done_with_ping() {
        assert_eq!(
            classify_exit(
                Some(0),
                "@@ARKAI_RESULT@@ {\"needs_human\":true,\"detail\":\"check speakers\"}"
            ),
            ExitAction::DoneNeedsHuman("check speakers".to_string())
        );
    }

    #[test]
    fn test_transcribe_memo_args_forward_approval_overrides() {
        let mut item = sample_item_with_path(PathBuf::from("/tmp/input.m4a"));
        item.chosen_engine = Some("whisperx".to_string());
        item.overrides = Some(serde_json::json!({
            "speakers": 2,
            "names": "Alex, Sam",
            "category": "meeting",
            "hint": "budget review"
        }));
        item.data.recorded_at = Some(Utc::now());

        let args: Vec<String> = transcribe_memo_args(&item, Path::new("/tmp/input.m4a"))
            .into_iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();

        assert_eq!(args[0], "/tmp/input.m4a");
        assert!(args.windows(2).any(|pair| pair == ["--stem", "Memo 3"]));
        assert!(args.windows(2).any(|pair| pair == ["--engine", "whisperx"]));
        assert!(args.windows(2).any(|pair| pair == ["--speakers", "2"]));
        assert!(args.windows(2).any(|pair| pair == ["--names", "Alex, Sam"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--category", "meeting"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--context", "budget review"]));
        assert!(args.iter().any(|arg| arg == "--when"));
    }

    #[tokio::test]
    async fn test_needs_human_result_records_ping() {
        let temp = TempDir::new().unwrap();
        let audio = temp.path().join("memo.m4a");
        tokio::fs::write(&audio, b"audio").await.unwrap();
        let queue = VoiceQueue::new(temp.path().join("queue.jsonl"));
        let (id, item) = enqueue_approved(&queue, &audio).await;
        let script = temp.path().join("fake-transcribe.sh");
        write_executable(
            &script,
            "#!/bin/sh\nprintf '%s\\n' '@@ARKAI_RESULT@@ {\"needs_human\":true,\"detail\":\"review names\"}'\nexit 0\n",
        )
        .await;
        let notifier = RecordingNotifier {
            calls: Mutex::new(Vec::new()),
        };

        process_one_item(
            &queue,
            &item,
            Some(&notifier),
            Some(&script),
            Some(Duration::from_secs(2)),
        )
        .await
        .unwrap();

        let done = queue.get(&id).await.unwrap().unwrap();
        assert_eq!(done.status, VoiceQueueStatus::Done);
        let calls = notifier.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(matches!(
            &calls[0].1,
            NotifyKind::NeedsHuman { detail } if detail == "review names"
        ));
    }

    #[tokio::test]
    async fn test_timeout_sigkills_process_group() {
        let temp = TempDir::new().unwrap();
        let audio = temp.path().join("memo.m4a");
        let pid_file = temp.path().join("grandchild.pid");
        tokio::fs::write(&audio, b"audio").await.unwrap();
        let queue = VoiceQueue::new(temp.path().join("queue.jsonl"));
        let (id, item) = enqueue_approved(&queue, &audio).await;
        let script = temp.path().join("hang.sh");
        write_executable(
            &script,
            &format!(
                "#!/bin/sh\nsleep 30 &\necho $! > '{}'\nsleep 30\n",
                pid_file.display()
            ),
        )
        .await;
        let start = Instant::now();

        process_one_item(
            &queue,
            &item,
            None,
            Some(&script),
            Some(Duration::from_millis(500)),
        )
        .await
        .unwrap();

        assert!(start.elapsed() < Duration::from_secs(2));
        let failed = queue.get(&id).await.unwrap().unwrap();
        assert_eq!(failed.status, VoiceQueueStatus::Failed);
        assert!(failed
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("hard timeout"));

        let grandchild_pid: i32 = tokio::fs::read_to_string(&pid_file)
            .await
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let mut dead = false;
        for _ in 0..20 {
            let rc = unsafe { libc::kill(grandchild_pid, 0) };
            if rc != 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                dead = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(dead, "grandchild process should be killed with the group");
    }

    #[tokio::test]
    async fn test_per_item_isolation_continues_after_failure() {
        let temp = TempDir::new().unwrap();
        let first = temp.path().join("first.m4a");
        let second = temp.path().join("second.m4a");
        tokio::fs::write(&first, b"first").await.unwrap();
        tokio::fs::write(&second, b"second").await.unwrap();
        let queue = VoiceQueue::new(temp.path().join("queue.jsonl"));
        let (first_id, _) = enqueue_approved(&queue, &first).await;
        let (second_id, _) = enqueue_approved(&queue, &second).await;
        let script = temp.path().join("mixed.sh");
        write_executable(
            &script,
            "#!/bin/sh\ncase \"$1\" in\n  *first.m4a) echo '@@ARKAI_RESULT@@ {\"error\":\"bad first\"}'; exit 9 ;;\n  *) echo '@@ARKAI_RESULT@@ {\"message\":\"ok\"}'; exit 0 ;;\nesac\n",
        )
        .await;
        let caps = LocalRouteCaps {
            limit: Some(2),
            max_hours: None,
        };

        execute_process_local_with_bin(false, &queue, &caps, None, Some(&script))
            .await
            .unwrap();

        let first_item = queue.get(&first_id).await.unwrap().unwrap();
        let second_item = queue.get(&second_id).await.unwrap().unwrap();
        assert_eq!(first_item.status, VoiceQueueStatus::Failed);
        assert_eq!(second_item.status, VoiceQueueStatus::Done);
    }

    #[tokio::test]
    async fn test_over_cap_without_allow_large_fails_but_allow_large_runs() {
        let temp = TempDir::new().unwrap();
        let audio = temp.path().join("large.m4a");
        tokio::fs::write(&audio, b"large").await.unwrap();
        let queue = VoiceQueue::new(temp.path().join("queue.jsonl"));
        let (id, mut item) = enqueue_approved(&queue, &audio).await;
        item.data.duration_seconds = Some(PER_ITEM_MAX_SECS + 1.0);
        let script = temp.path().join("success.sh");
        write_executable(
            &script,
            "#!/bin/sh\necho '@@ARKAI_RESULT@@ {\"message\":\"ok\"}'\nexit 0\n",
        )
        .await;

        process_one_item(
            &queue,
            &item,
            None,
            Some(&script),
            Some(Duration::from_secs(2)),
        )
        .await
        .unwrap();
        let failed = queue.get(&id).await.unwrap().unwrap();
        assert_eq!(failed.status, VoiceQueueStatus::Failed);
        assert_eq!(
            failed.error.as_deref(),
            Some("over 6h cap; re-approve with --allow-large")
        );

        queue.retry(&id).await.unwrap();
        let mut retry_item = queue.get(&id).await.unwrap().unwrap();
        retry_item.data.duration_seconds = Some(PER_ITEM_MAX_SECS + 1.0);
        retry_item.allow_large = true;
        process_one_item(
            &queue,
            &retry_item,
            None,
            Some(&script),
            Some(Duration::from_secs(2)),
        )
        .await
        .unwrap();
        let done = queue.get(&id).await.unwrap().unwrap();
        assert_eq!(done.status, VoiceQueueStatus::Done);
    }

    #[tokio::test]
    async fn test_cache_copy_preferred_over_file_path() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.m4a");
        let cache = temp.path().join("cache");
        tokio::fs::create_dir_all(&cache).await.unwrap();
        tokio::fs::write(&source, b"source").await.unwrap();
        let item = sample_item_with_path(source);
        let cache_file = cache.join(format!("{}.m4a", item.id));
        tokio::fs::write(&cache_file, b"cache").await.unwrap();

        let resolved = resolve_input_path_in_cache(&item, &cache)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(resolved, cache_file);
    }
}

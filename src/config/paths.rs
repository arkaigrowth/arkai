//! Canonical paths for arkai voice pipeline.
//!
//! Single source of truth - import this instead of hardcoding paths.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use arkai::config::paths;
//!
//! let queue_file = paths::voice_queue()?;
//! let cache_dir = paths::voice_cache()?;
//! ```
//!
//! ## Path Ownership
//!
//! | Location | Owner | Purpose |
//! |----------|-------|---------|
//! | Mac paths (functions) | Mac/arkai | Engine state, library |
//! | VPS_* constants | VPS/Claudia | Artifacts, requests, results |
//! | TELEGRAM_INBOUND | Clawdbot | Read-only for pipeline |

use std::path::PathBuf;

use anyhow::Result;

// ============================================================================
// Mac paths (functions - resolved at runtime)
// ============================================================================

/// Get the arkai home directory (~/.arkai)
pub fn arkai_home() -> Result<PathBuf> {
    crate::config::arkai_home()
}

/// Get the voice queue file path (~/.arkai/voice_queue.jsonl)
pub fn voice_queue() -> Result<PathBuf> {
    Ok(arkai_home()?.join("voice_queue.jsonl"))
}

/// Get the voice cache directory (~/.arkai/voice_cache/)
/// Used for normalized audio files (.qta -> .m4a conversions)
pub fn voice_cache() -> Result<PathBuf> {
    crate::config::voice_cache_dir()
}

/// Get the library voice directory (~/AI/library/voice/)
/// Final destination for processed voice transcripts
pub fn library_voice() -> Result<PathBuf> {
    Ok(crate::config::library_dir()?.join("voice"))
}

/// Get the Apple Voice Memos recording directory
/// This is the canonical watch path for the voice watcher
pub fn voice_memos_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join("Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings")
}

// ============================================================================
// VPS paths (constants - used by Python code on VPS)
// ============================================================================

/// VPS artifacts root directory
pub const VPS_ARTIFACTS: &str = "~/clawd/artifacts/voice";

/// VPS voice request files directory
pub const VPS_REQUESTS: &str = "~/clawd/artifacts/voice/requests";

/// VPS voice result files directory
pub const VPS_RESULTS: &str = "~/clawd/artifacts/voice/results";

/// VPS audio cache directory (Telegram downloads, etc.)
pub const VPS_AUDIO_CACHE: &str = "~/clawd/artifacts/voice/audio-cache";

// ============================================================================
// Clawdbot paths (read-only for pipeline)
// ============================================================================

/// Telegram inbound media directory (Clawdbot manages this)
pub const TELEGRAM_INBOUND: &str = "~/.clawdbot/media/inbound";

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_memos_dir_is_absolute() {
        let path = voice_memos_dir();
        assert!(
            path.is_absolute() || path.starts_with("~"),
            "voice_memos_dir should be absolute or start with ~"
        );
    }

    #[test]
    fn test_voice_memos_dir_has_expected_components() {
        let path = voice_memos_dir();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("VoiceMemos") || path_str.contains("Recordings"),
            "voice_memos_dir should contain expected path components"
        );
    }

    #[test]
    fn test_vps_paths_have_expected_structure() {
        assert!(VPS_ARTIFACTS.contains("clawd/artifacts/voice"));
        assert!(VPS_REQUESTS.contains("requests"));
        assert!(VPS_RESULTS.contains("results"));
        assert!(VPS_AUDIO_CACHE.contains("audio-cache"));
    }

    #[test]
    fn test_telegram_inbound_path() {
        assert!(TELEGRAM_INBOUND.contains("clawdbot"));
        assert!(TELEGRAM_INBOUND.contains("inbound"));
    }
}

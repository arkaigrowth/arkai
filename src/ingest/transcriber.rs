//! Whisper transcription backend.
//!
//! Shells out to local whisper binary for transcription.

use std::path::Path;
use std::process::Stdio;

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::process::Command;

/// Result of transcription
#[derive(Debug, Clone)]
pub struct TranscriptResult {
    pub text: String,
    pub language: String,
    pub duration_seconds: f64,
}

/// Whisper output JSON structure
#[derive(Debug, Deserialize)]
struct WhisperOutput {
    text: String,
    #[serde(default)]
    language: String,
    #[serde(default)]
    segments: Vec<WhisperSegment>,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    #[serde(default)]
    end: f64,
}

/// Transcribe audio using local Whisper binary
pub async fn transcribe(audio_path: &Path, model: &str) -> Result<TranscriptResult> {
    let whisper_path = std::env::var("WHISPER_PATH")
        .unwrap_or_else(|_| "/opt/homebrew/bin/whisper".to_string());

    // Create temp dir for output
    let temp_dir = tempfile::tempdir().context("Failed to create temp dir")?;

    let output = Command::new(&whisper_path)
        .arg(audio_path)
        .arg("--model")
        .arg(model)
        .arg("--output_dir")
        .arg(temp_dir.path())
        .arg("--output_format")
        .arg("json")
        .arg("--language")
        .arg("en") // Default to English, can be made configurable
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to run whisper")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Whisper failed: {}", stderr);
    }

    // Find and parse JSON output
    let stem = audio_path.file_stem().unwrap_or_default().to_string_lossy();
    let json_path = temp_dir.path().join(format!("{}.json", stem));

    let json_content = tokio::fs::read_to_string(&json_path)
        .await
        .context("Failed to read whisper output")?;

    let whisper: WhisperOutput =
        serde_json::from_str(&json_content).context("Failed to parse whisper JSON")?;

    let duration = whisper
        .segments
        .last()
        .map(|s| s.end)
        .unwrap_or(0.0);

    Ok(TranscriptResult {
        text: whisper.text.trim().to_string(),
        language: if whisper.language.is_empty() {
            "en".to_string()
        } else {
            whisper.language
        },
        duration_seconds: duration,
    })
}

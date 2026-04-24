//! Durable YouTube transcript acquisition via yt-dlp audio + Whisper.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::fs;
use tokio::process::Command;

pub const DEFAULT_YT_DLP_PATH: &str = "/opt/homebrew/bin/yt-dlp";
pub const DEFAULT_WHISPER_PATH: &str = "/opt/homebrew/bin/whisper";
pub const DEFAULT_WHISPER_MODEL: &str = "large-v3-turbo";

/// Transcript artifacts produced from a YouTube URL.
#[derive(Debug, Clone)]
pub struct YouTubeTranscriptArtifacts {
    pub transcript: String,
    pub transcript_json: Option<String>,
}

pub type YouTubeTranscript = YouTubeTranscriptArtifacts;

impl YouTubeTranscriptArtifacts {
    pub fn word_count(&self) -> usize {
        self.transcript.split_whitespace().count()
    }

    pub fn persist_to_dir(&self, output_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(output_dir).with_context(|| {
            format!(
                "Failed to create transcript artifact directory: {}",
                output_dir.display()
            )
        })?;

        std::fs::write(output_dir.join("transcript.txt"), &self.transcript).with_context(|| {
            format!(
                "Failed to write transcript artifact: {}",
                output_dir.join("transcript.txt").display()
            )
        })?;

        if let Some(transcript_json) = &self.transcript_json {
            std::fs::write(output_dir.join("transcript.json"), transcript_json).with_context(
                || {
                    format!(
                        "Failed to write transcript artifact: {}",
                        output_dir.join("transcript.json").display()
                    )
                },
            )?;
        }

        Ok(())
    }
}

/// Acquire a YouTube transcript using the default local yt-dlp and Whisper binaries.
pub async fn acquire_youtube_transcript(url: &str) -> Result<YouTubeTranscriptArtifacts> {
    acquire_youtube_transcript_with(
        url,
        DEFAULT_YT_DLP_PATH,
        DEFAULT_WHISPER_PATH,
        DEFAULT_WHISPER_MODEL,
    )
    .await
}

pub async fn fetch_youtube_transcript(url: &str) -> Result<YouTubeTranscriptArtifacts> {
    acquire_youtube_transcript(url).await
}

/// Acquire a YouTube transcript using explicit binary paths.
pub async fn acquire_youtube_transcript_with(
    url: &str,
    yt_dlp_path: &str,
    whisper_path: &str,
    whisper_model: &str,
) -> Result<YouTubeTranscriptArtifacts> {
    let work_dir = tempfile::tempdir().context("Failed to create temp dir for YouTube audio")?;
    let audio_file = download_audio(url, yt_dlp_path, work_dir.path()).await?;
    transcribe_audio(&audio_file, whisper_path, whisper_model, work_dir.path()).await
}

async fn download_audio(url: &str, yt_dlp_path: &str, output_dir: &Path) -> Result<PathBuf> {
    let audio_out = output_dir.join("audio.%(ext)s");

    let output = Command::new(yt_dlp_path)
        .arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("0")
        .arg("-o")
        .arg(&audio_out)
        .arg(url)
        .kill_on_drop(true)
        .output()
        .await
        .with_context(|| format!("Failed to run yt-dlp for YouTube URL: {}", url))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("yt-dlp audio download failed: {}", stderr.trim());
    }

    let audio_file = output_dir.join("audio.mp3");
    anyhow::ensure!(
        audio_file.exists(),
        "Audio download failed — audio.mp3 not found in temp dir"
    );

    Ok(audio_file)
}

async fn transcribe_audio(
    audio_path: &Path,
    whisper_path: &str,
    whisper_model: &str,
    output_dir: &Path,
) -> Result<YouTubeTranscriptArtifacts> {
    let output = Command::new(whisper_path)
        .arg(audio_path)
        .arg("--model")
        .arg(whisper_model)
        .arg("--output_format")
        .arg("all")
        .arg("--output_dir")
        .arg(output_dir)
        .kill_on_drop(true)
        .output()
        .await
        .with_context(|| {
            format!(
                "Failed to run Whisper on downloaded audio: {}",
                audio_path.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Whisper transcription failed: {}", stderr.trim());
    }

    let stem = audio_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("audio");

    let transcript_path = output_dir.join(format!("{}.txt", stem));
    let transcript = fs::read_to_string(&transcript_path)
        .await
        .with_context(|| {
            format!(
                "Whisper transcription produced no text output: {}",
                transcript_path.display()
            )
        })?;

    let transcript_json_path = output_dir.join(format!("{}.json", stem));
    let transcript_json = if transcript_json_path.exists() {
        Some(
            fs::read_to_string(&transcript_json_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to read Whisper transcript JSON: {}",
                        transcript_json_path.display()
                    )
                })?,
        )
    } else {
        None
    };

    Ok(YouTubeTranscriptArtifacts {
        transcript,
        transcript_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn write_executable(dir: &TempDir, name: &str, script: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, script).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    #[tokio::test]
    async fn test_acquire_youtube_transcript_with_preserves_json_artifact() {
        let bin_dir = TempDir::new().unwrap();

        let yt_dlp = write_executable(
            &bin_dir,
            "yt-dlp",
            r#"#!/bin/sh
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    out="$2"
    shift 2
    continue
  fi
  shift
done
out="$(printf '%s' "$out" | sed 's/%(ext)s/mp3/')"
mkdir -p "$(dirname "$out")"
printf 'fake audio' > "$out"
"#,
        );

        let whisper = write_executable(
            &bin_dir,
            "whisper",
            r#"#!/bin/sh
audio=""
output_dir=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output_dir)
      output_dir="$2"
      shift 2
      ;;
    --model|--output_format)
      shift 2
      ;;
    *)
      if [ -z "$audio" ]; then
        audio="$1"
      fi
      shift
      ;;
  esac
done
stem="$(basename "$audio" .mp3)"
printf 'transcript body\n' > "$output_dir/$stem.txt"
printf '{"text":"transcript body"}\n' > "$output_dir/$stem.json"
"#,
        );

        let artifacts = acquire_youtube_transcript_with(
            "https://youtu.be/example",
            yt_dlp.to_str().unwrap(),
            whisper.to_str().unwrap(),
            "large-v3-turbo",
        )
        .await
        .unwrap();

        assert_eq!(artifacts.transcript, "transcript body\n");
        assert_eq!(
            artifacts.transcript_json.as_deref(),
            Some("{\"text\":\"transcript body\"}\n")
        );
    }

    #[tokio::test]
    async fn test_acquire_youtube_transcript_with_allows_missing_json_artifact() {
        let bin_dir = TempDir::new().unwrap();

        let yt_dlp = write_executable(
            &bin_dir,
            "yt-dlp",
            r#"#!/bin/sh
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    out="$2"
    shift 2
    continue
  fi
  shift
done
out="$(printf '%s' "$out" | sed 's/%(ext)s/mp3/')"
mkdir -p "$(dirname "$out")"
printf 'fake audio' > "$out"
"#,
        );

        let whisper = write_executable(
            &bin_dir,
            "whisper",
            r#"#!/bin/sh
audio=""
output_dir=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output_dir)
      output_dir="$2"
      shift 2
      ;;
    --model|--output_format)
      shift 2
      ;;
    *)
      if [ -z "$audio" ]; then
        audio="$1"
      fi
      shift
      ;;
  esac
done
stem="$(basename "$audio" .mp3)"
printf 'transcript body\n' > "$output_dir/$stem.txt"
"#,
        );

        let artifacts = acquire_youtube_transcript_with(
            "https://youtu.be/example",
            yt_dlp.to_str().unwrap(),
            whisper.to_str().unwrap(),
            "large-v3-turbo",
        )
        .await
        .unwrap();

        assert_eq!(artifacts.transcript, "transcript body\n");
        assert_eq!(artifacts.transcript_json, None);
        assert_eq!(artifacts.word_count(), 2);
    }
}

//! Telegram Bot API adapter for sending voice memos to Claudia.
//!
//! This adapter uploads audio files to a Telegram chat, where Claudia
//! can receive and transcribe them.

use std::path::Path;

use anyhow::{Context, Result};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};

/// Telegram Bot API client
pub struct TelegramClient {
    /// Bot token
    bot_token: String,
    /// Target chat ID
    chat_id: String,
    /// HTTP client
    client: reqwest::Client,
}

/// Response from Telegram API
#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

/// Message result from sendMessage/sendAudio
#[derive(Debug, Deserialize)]
struct MessageResult {
    message_id: i64,
}

/// Configuration for Telegram client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

impl TelegramClient {
    /// Create a new Telegram client
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            bot_token,
            chat_id,
            client: reqwest::Client::new(),
        }
    }

    /// Create from config
    pub fn from_config(config: TelegramConfig) -> Self {
        Self::new(config.bot_token, config.chat_id)
    }

    /// Build API URL
    fn api_url(&self, method: &str) -> String {
        format!(
            "https://api.telegram.org/bot{}/{}",
            self.bot_token, method
        )
    }

    /// Send a text message
    pub async fn send_message(&self, text: &str) -> Result<i64> {
        let url = self.api_url("sendMessage");

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": self.chat_id,
                "text": text,
            }))
            .send()
            .await
            .context("Failed to send Telegram message")?;

        let result: TelegramResponse<MessageResult> = response
            .json()
            .await
            .context("Failed to parse Telegram response")?;

        if !result.ok {
            anyhow::bail!(
                "Telegram API error: {}",
                result.description.unwrap_or_default()
            );
        }

        Ok(result.result.map(|r| r.message_id).unwrap_or(0))
    }

    /// Send an audio file
    pub async fn send_audio(&self, audio_path: &Path, caption: Option<&str>) -> Result<i64> {
        let url = self.api_url("sendAudio");

        // Read file
        let file_name = audio_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let file_bytes = tokio::fs::read(audio_path)
            .await
            .context("Failed to read audio file")?;

        // Build multipart form
        let file_part = Part::bytes(file_bytes)
            .file_name(file_name.clone())
            .mime_str("audio/mp4")?;

        let mut form = Form::new()
            .text("chat_id", self.chat_id.clone())
            .part("audio", file_part);

        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }

        let response = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .context("Failed to send Telegram audio")?;

        let result: TelegramResponse<MessageResult> = response
            .json()
            .await
            .context("Failed to parse Telegram response")?;

        if !result.ok {
            anyhow::bail!(
                "Telegram API error: {}",
                result.description.unwrap_or_default()
            );
        }

        Ok(result.result.map(|r| r.message_id).unwrap_or(0))
    }

    /// Send a voice message (for .ogg files, but we'll use audio for .m4a)
    pub async fn send_voice_memo(&self, audio_path: &Path) -> Result<i64> {
        let file_name = audio_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        // Caption includes source info
        let caption = format!("🎙️ Voice Memo: {}", file_name);

        self.send_audio(audio_path, Some(&caption)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_url() {
        let client = TelegramClient::new("TOKEN".to_string(), "123".to_string());
        assert_eq!(
            client.api_url("sendMessage"),
            "https://api.telegram.org/botTOKEN/sendMessage"
        );
    }
}

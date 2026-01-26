//! Clawdbot webhook client for sending transcripts to Claudia on VPS.
//!
//! Endpoint: POST /hooks/agent
//! Auth: Bearer token

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Clawdbot webhook client
pub struct ClawdbotClient {
    endpoint: String,
    token: String,
    client: reqwest::Client,
}

/// Payload for voice intake webhook
#[derive(Debug, Serialize)]
pub struct VoiceIntakePayload {
    /// The transcribed text (prefixed with context)
    pub message: String,
    /// Label for logs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Session key for continuity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    /// Deliver response to Telegram
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deliver: Option<bool>,
    /// Delivery channel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Telegram chat ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

/// Response from clawdbot webhook
#[derive(Debug, Deserialize)]
pub struct WebhookResponse {
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
}

impl ClawdbotClient {
    /// Create a new client
    pub fn new(endpoint: String, token: String) -> Self {
        Self {
            endpoint,
            token,
            client: reqwest::Client::new(),
        }
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let endpoint = std::env::var("CLAWDBOT_ENDPOINT")
            .unwrap_or_else(|_| "http://arkai-clawdbot:18789/hooks/agent".to_string());
        let token = std::env::var("CLAWDBOT_TOKEN")
            .context("CLAWDBOT_TOKEN environment variable required")?;
        Ok(Self::new(endpoint, token))
    }

    /// Send a voice transcript to Claudia
    pub async fn send_voice_intake(
        &self,
        transcript: &str,
        audio_hash: &str,
        duration_secs: f64,
        deliver_to_telegram: bool,
        telegram_chat_id: Option<&str>,
    ) -> Result<WebhookResponse> {
        // Format message with context
        let message = format!(
            "[Voice Memo | id:{} | {:.0}s]\n\n{}",
            &audio_hash[..8],
            duration_secs,
            transcript
        );

        let mut payload = VoiceIntakePayload {
            message,
            name: Some("Voice".to_string()),
            session_key: Some("hook:voice:main".to_string()),
            deliver: Some(deliver_to_telegram),
            channel: None,
            to: None,
        };

        if deliver_to_telegram {
            payload.channel = Some("telegram".to_string());
            payload.to = telegram_chat_id.map(|s| s.to_string());
        }

        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send to clawdbot")?;

        let status = response.status();
        if status.is_success() || status.as_u16() == 202 {
            // 202 Accepted is expected for async processing
            Ok(WebhookResponse {
                status: "accepted".to_string(),
                message: Some("Processing".to_string()),
            })
        } else {
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Clawdbot error ({}): {}", status, text)
        }
    }
}

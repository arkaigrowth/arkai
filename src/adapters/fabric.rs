//! Fabric adapter for AI pattern execution.
//!
//! MVP implementation uses subprocess mode, calling the `fabric` CLI directly.
//! Future: HTTP REST mode connecting to `fabric --serve`.
//!
//! # Special Actions
//!
//! The adapter supports special action prefixes for content fetching:
//! - `__youtube__`: Fetch YouTube transcript with timestamps (uses `fabric -y <url> --transcript-with-timestamps`)
//! - `__web__`: Fetch web page content (uses `fabric -u <url>`)
//! - All other actions are treated as pattern names (uses `fabric -p <pattern>`)

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use super::{Adapter, AdapterOutput};

/// Special action for fetching YouTube transcripts
pub const ACTION_YOUTUBE: &str = "__youtube__";

/// Special action for fetching web page content
pub const ACTION_WEB: &str = "__web__";

/// Fabric adapter using subprocess mode
pub struct FabricAdapter {
    /// Path to the fabric binary (default: "fabric")
    binary_path: String,
}

impl Default for FabricAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl FabricAdapter {
    /// Create a new Fabric adapter with default binary path
    ///
    /// Looks for a compatible Fabric AI CLI.
    ///
    /// Homebrew installs the AI CLI as `fabric-ai`, but that binary only
    /// behaves correctly in fetch mode when argv[0] is `fabric`. We normalize
    /// that internally and avoid falling back to unrelated tools like the
    /// Python SSH utility also named `fabric`.
    pub fn new() -> Self {
        let binary_path = Self::resolve_binary_path();

        Self { binary_path }
    }

    /// Create a Fabric adapter with a custom binary path
    pub fn with_binary_path(binary_path: impl Into<String>) -> Self {
        Self {
            binary_path: binary_path.into(),
        }
    }

    fn resolve_binary_path() -> String {
        if Self::supports_fabric_ai_cli("fabric-ai") {
            return "fabric-ai".to_string();
        }

        if Self::supports_fabric_ai_cli("fabric") {
            return "fabric".to_string();
        }

        // Prefer failing clearly on a missing/invalid AI Fabric binary over
        // silently selecting the unrelated Python SSH tool named `fabric`.
        "fabric-ai".to_string()
    }

    fn supports_fabric_ai_cli(candidate: &str) -> bool {
        let Ok(output) = std::process::Command::new(candidate).arg("--help").output() else {
            return false;
        };

        if !output.status.success() {
            return false;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Self::looks_like_fabric_ai_help(&stdout) || Self::looks_like_fabric_ai_help(&stderr)
    }

    fn looks_like_fabric_ai_help(help: &str) -> bool {
        help.contains("--pattern") && help.contains("--youtube") && help.contains("--scrape_url")
    }

    fn should_alias_argv0(binary_path: &str) -> bool {
        Path::new(binary_path)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "fabric-ai")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.binary_path);

        #[cfg(unix)]
        if Self::should_alias_argv0(&self.binary_path) {
            command.arg0("fabric");
        }

        command
    }

    /// Execute a pattern via subprocess
    ///
    /// This is the MVP implementation. It spawns `fabric -p <pattern>`
    /// and pipes the input to stdin, collecting output from stdout.
    async fn execute_subprocess(
        &self,
        pattern: &str,
        input: &str,
        step_timeout: Duration,
    ) -> Result<String> {
        let mut child = self
            .command()
            .args(["-p", pattern])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn fabric process for pattern '{}'", pattern))?;

        // Write input to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .await
                .context("Failed to write to fabric stdin")?;
            // Drop stdin to signal EOF
        }

        // Wait for completion with timeout
        let output = timeout(step_timeout, child.wait_with_output())
            .await
            .with_context(|| {
                format!(
                    "Fabric pattern '{}' timed out after {:?}",
                    pattern, step_timeout
                )
            })?
            .with_context(|| {
                format!(
                    "Failed to wait for fabric process for pattern '{}'",
                    pattern
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);
            anyhow::bail!(
                "Fabric pattern '{}' failed with exit code {}: {}",
                pattern,
                exit_code,
                stderr.trim()
            );
        }

        let stdout =
            String::from_utf8(output.stdout).context("Fabric output is not valid UTF-8")?;

        Ok(stdout)
    }

    /// Fetch YouTube transcript via fabric -y <url> --transcript-with-timestamps
    async fn fetch_youtube(&self, url: &str, step_timeout: Duration) -> Result<String> {
        let output = timeout(
            step_timeout,
            self.command()
                .args(["-y", url, "--transcript-with-timestamps"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .with_context(|| format!("YouTube fetch timed out for URL: {}", url))?
        .with_context(|| format!("Failed to fetch YouTube content from: {}", url))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);
            anyhow::bail!(
                "YouTube fetch failed with exit code {}: {}",
                exit_code,
                stderr.trim()
            );
        }

        let stdout =
            String::from_utf8(output.stdout).context("YouTube transcript is not valid UTF-8")?;

        Ok(stdout)
    }

    /// Fetch web page content via fabric -u <url>
    async fn fetch_web(&self, url: &str, step_timeout: Duration) -> Result<String> {
        let output = timeout(
            step_timeout,
            self.command()
                .args(["-u", url])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .with_context(|| format!("Web fetch timed out for URL: {}", url))?
        .with_context(|| format!("Failed to fetch web content from: {}", url))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);
            anyhow::bail!(
                "Web fetch failed with exit code {}: {}",
                exit_code,
                stderr.trim()
            );
        }

        let stdout = String::from_utf8(output.stdout).context("Web content is not valid UTF-8")?;

        Ok(stdout)
    }
}

#[async_trait]
impl Adapter for FabricAdapter {
    fn name(&self) -> &str {
        "fabric"
    }

    async fn execute(&self, action: &str, input: &str, timeout: Duration) -> Result<AdapterOutput> {
        // Handle special actions for content fetching
        let content = match action {
            ACTION_YOUTUBE => {
                // Input is the YouTube URL
                self.fetch_youtube(input, timeout).await?
            }
            ACTION_WEB => {
                // Input is the web URL
                self.fetch_web(input, timeout).await?
            }
            _ => {
                // Standard pattern execution
                self.execute_subprocess(action, input, timeout).await?
            }
        };

        Ok(AdapterOutput::new(content))
    }

    async fn health_check(&self) -> Result<()> {
        // Check that fabric is available and can list patterns
        let output = self
            .command()
            .arg("-l")
            .output()
            .await
            .context("Failed to run fabric health check")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Fabric health check failed: {}", stderr);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fabric_adapter_creation() {
        let adapter = FabricAdapter::new();
        assert_eq!(adapter.name(), "fabric");
    }

    #[tokio::test]
    async fn test_custom_binary_path() {
        let adapter = FabricAdapter::with_binary_path("/custom/path/fabric");
        assert_eq!(adapter.binary_path, "/custom/path/fabric");
    }

    #[test]
    fn test_detects_fabric_ai_help_signature() {
        let help = "Usage:\n  fabric-ai [OPTIONS]\n\n  -p, --pattern=\n  -y, --youtube=\n  -u, --scrape_url=\n";
        assert!(FabricAdapter::looks_like_fabric_ai_help(help));
    }

    #[test]
    fn test_rejects_non_ai_fabric_help_signature() {
        let help = "Usage: fabric [OPTIONS] COMMAND [ARGS]...\n\nCommands:\n  run\n";
        assert!(!FabricAdapter::looks_like_fabric_ai_help(help));
    }

    #[test]
    fn test_aliases_homebrew_fabric_ai_process_name() {
        assert!(FabricAdapter::should_alias_argv0("fabric-ai"));
        assert!(FabricAdapter::should_alias_argv0("/opt/homebrew/bin/fabric-ai"));
        assert!(!FabricAdapter::should_alias_argv0("fabric"));
    }

    // Note: Integration tests with actual Fabric would go in tests/
}

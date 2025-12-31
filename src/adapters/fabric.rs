//! Fabric adapter for AI pattern execution.
//!
//! MVP implementation uses subprocess mode, calling the `fabric` CLI directly.
//! Future: HTTP REST mode connecting to `fabric --serve`.

use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use super::{Adapter, AdapterOutput};

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
    /// Looks for fabric-ai first (Homebrew install), falls back to fabric
    pub fn new() -> Self {
        // Try fabric-ai first (Homebrew install name), then fabric
        let binary_path = if std::process::Command::new("fabric-ai")
            .arg("--help")
            .output()
            .is_ok()
        {
            "fabric-ai".to_string()
        } else {
            "fabric".to_string()
        };

        Self { binary_path }
    }

    /// Create a Fabric adapter with a custom binary path
    pub fn with_binary_path(binary_path: impl Into<String>) -> Self {
        Self {
            binary_path: binary_path.into(),
        }
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
        let mut child = Command::new(&self.binary_path)
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
            .with_context(|| format!("Failed to wait for fabric process for pattern '{}'", pattern))?;

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

        let stdout = String::from_utf8(output.stdout)
            .context("Fabric output is not valid UTF-8")?;

        Ok(stdout)
    }
}

#[async_trait]
impl Adapter for FabricAdapter {
    fn name(&self) -> &str {
        "fabric"
    }

    async fn execute(
        &self,
        action: &str,
        input: &str,
        timeout: Duration,
    ) -> Result<AdapterOutput> {
        // Action is the pattern name for Fabric
        let content = self.execute_subprocess(action, input, timeout).await?;
        Ok(AdapterOutput::new(content))
    }

    async fn health_check(&self) -> Result<()> {
        // Check that fabric is available and can list patterns
        let output = Command::new(&self.binary_path)
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

    // Note: Integration tests with actual Fabric would go in tests/
}

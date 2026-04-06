//! Fabric adapter for AI pattern execution.
//!
//! MVP implementation uses subprocess mode, calling the `fabric` CLI directly.
//! Future: HTTP REST mode connecting to `fabric --serve`.
//!
//! # Special Actions
//!
//! The adapter supports special action prefixes for content fetching:
//! - `__youtube__`: Fetch YouTube transcript via yt-dlp audio + Whisper
//! - `__web__`: Fetch web page content (uses `fabric -u <url>`)
//! - All other actions are treated as pattern names (uses `fabric -p <pattern>`)

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use super::{Adapter, AdapterOutput};
use crate::config::{self, FabricBinaryOverrideSource};

/// Special action for fetching YouTube transcripts
pub const ACTION_YOUTUBE: &str = "__youtube__";

/// Special action for fetching web page content
pub const ACTION_WEB: &str = "__web__";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FabricBinarySelectionSource {
    EnvOverride,
    ConfigOverride,
    ExplicitArgument,
    AutoPathFabricAi,
    AutoPathFabric,
    AutoFallback,
    ConfigError,
}

impl FabricBinarySelectionSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EnvOverride => "env_override",
            Self::ConfigOverride => "config_override",
            Self::ExplicitArgument => "explicit_argument",
            Self::AutoPathFabricAi => "auto_path_fabric_ai",
            Self::AutoPathFabric => "auto_path_fabric",
            Self::AutoFallback => "auto_fallback",
            Self::ConfigError => "config_error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FabricBinaryDiagnostics {
    pub requested_binary: Option<String>,
    pub selected_binary: String,
    pub selection_source: FabricBinarySelectionSource,
    pub signature_passed: bool,
    pub argv0_alias: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
struct CandidateProbe {
    selected_binary: String,
    signature_passed: bool,
    error: Option<String>,
}

/// Fabric adapter using subprocess mode
pub struct FabricAdapter {
    /// Path to the fabric binary (default: "fabric")
    binary_path: String,
    diagnostics: FabricBinaryDiagnostics,
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
        let diagnostics = Self::resolve_binary_diagnostics();
        let binary_path = diagnostics.selected_binary.clone();

        Self {
            binary_path,
            diagnostics,
        }
    }

    /// Create a Fabric adapter with a custom binary path
    pub fn with_binary_path(binary_path: impl Into<String>) -> Self {
        let diagnostics = Self::resolve_explicit_binary(
            binary_path.into(),
            FabricBinarySelectionSource::ExplicitArgument,
        );
        Self {
            binary_path: diagnostics.selected_binary.clone(),
            diagnostics,
        }
    }

    pub fn binary_diagnostics(&self) -> &FabricBinaryDiagnostics {
        &self.diagnostics
    }

    fn resolve_binary_diagnostics() -> FabricBinaryDiagnostics {
        match config::fabric_binary_override() {
            Ok(Some(binary_override)) => {
                let source = match binary_override.source {
                    FabricBinaryOverrideSource::Env => FabricBinarySelectionSource::EnvOverride,
                    FabricBinaryOverrideSource::Config => {
                        FabricBinarySelectionSource::ConfigOverride
                    }
                };
                Self::resolve_explicit_binary(binary_override.value, source)
            }
            Ok(None) => {
                let fabric_ai_probe = Self::probe_candidate("fabric-ai");
                if fabric_ai_probe.signature_passed {
                    return Self::diagnostics_from_probe(
                        None,
                        FabricBinarySelectionSource::AutoPathFabricAi,
                        fabric_ai_probe,
                    );
                }

                let fabric_probe = Self::probe_candidate("fabric");
                if fabric_probe.signature_passed {
                    return Self::diagnostics_from_probe(
                        None,
                        FabricBinarySelectionSource::AutoPathFabric,
                        fabric_probe,
                    );
                }

                let selected_binary = fabric_ai_probe.selected_binary.clone();
                let error = format!(
                    "No compatible AI Fabric CLI found. Checked {} and {}. Set ARKAI_FABRIC_BIN or `fabric.binary` to override. {} {}",
                    fabric_ai_probe.selected_binary,
                    fabric_probe.selected_binary,
                    fabric_ai_probe.error.as_deref().unwrap_or(""),
                    fabric_probe.error.as_deref().unwrap_or("")
                )
                .trim()
                .to_string();

                FabricBinaryDiagnostics {
                    requested_binary: None,
                    selected_binary: selected_binary.clone(),
                    selection_source: FabricBinarySelectionSource::AutoFallback,
                    signature_passed: false,
                    argv0_alias: Self::should_alias_argv0(&selected_binary),
                    error: Some(error),
                }
            }
            Err(error) => {
                let selected_binary = "fabric-ai".to_string();
                FabricBinaryDiagnostics {
                    requested_binary: None,
                    selected_binary: selected_binary.clone(),
                    selection_source: FabricBinarySelectionSource::ConfigError,
                    signature_passed: false,
                    argv0_alias: Self::should_alias_argv0(&selected_binary),
                    error: Some(format!(
                        "Failed to load Arkai config while resolving Fabric binary: {}",
                        error
                    )),
                }
            }
        }
    }

    fn resolve_explicit_binary(
        binary_path: String,
        source: FabricBinarySelectionSource,
    ) -> FabricBinaryDiagnostics {
        let probe = Self::probe_candidate(&binary_path);
        Self::diagnostics_from_probe(Some(binary_path), source, probe)
    }

    fn diagnostics_from_probe(
        requested_binary: Option<String>,
        source: FabricBinarySelectionSource,
        probe: CandidateProbe,
    ) -> FabricBinaryDiagnostics {
        FabricBinaryDiagnostics {
            requested_binary,
            argv0_alias: Self::should_alias_argv0(&probe.selected_binary),
            selected_binary: probe.selected_binary,
            selection_source: source,
            signature_passed: probe.signature_passed,
            error: probe.error,
        }
    }

    fn probe_candidate(candidate: &str) -> CandidateProbe {
        let selected_binary =
            Self::resolve_candidate_path(candidate).unwrap_or_else(|| candidate.to_string());

        let output = std::process::Command::new(&selected_binary)
            .arg("--help")
            .output();

        let output = match output {
            Ok(output) => output,
            Err(error) => {
                return CandidateProbe {
                    selected_binary,
                    signature_passed: false,
                    error: Some(format!("Failed to run '{} --help': {}", candidate, error)),
                };
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);
            return CandidateProbe {
                selected_binary,
                signature_passed: false,
                error: Some(format!(
                    "'{} --help' failed with exit code {}: {}",
                    candidate,
                    exit_code,
                    stderr.trim()
                )),
            };
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let signature_passed =
            Self::looks_like_fabric_ai_help(&stdout) || Self::looks_like_fabric_ai_help(&stderr);

        let error = if signature_passed {
            None
        } else {
            Some(format!(
                "Selected Fabric binary '{}' is incompatible: expected AI Fabric CLI help signature containing --pattern, --youtube, and --scrape_url",
                selected_binary
            ))
        };

        CandidateProbe {
            selected_binary,
            signature_passed,
            error,
        }
    }

    fn resolve_candidate_path(candidate: &str) -> Option<String> {
        let path = Path::new(candidate);
        let looks_like_path = path.is_absolute()
            || candidate.starts_with('.')
            || candidate.contains(std::path::MAIN_SEPARATOR);

        if looks_like_path {
            let resolved = if path.is_absolute() {
                PathBuf::from(path)
            } else {
                std::env::current_dir().ok()?.join(path)
            };
            return Some(
                resolved
                    .canonicalize()
                    .unwrap_or(resolved)
                    .display()
                    .to_string(),
            );
        }

        let path_var = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path_var) {
            let full_path = dir.join(candidate);
            if full_path.is_file() {
                return Some(full_path.display().to_string());
            }
        }

        None
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

    fn ensure_compatible(&self) -> Result<()> {
        if self.diagnostics.signature_passed {
            return Ok(());
        }

        anyhow::bail!(
            "{}",
            self.diagnostics.error.as_deref().unwrap_or(
                "Selected Fabric binary is incompatible with Arkai's AI Fabric integration"
            )
        )
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
        self.ensure_compatible()?;

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

    /// Fetch YouTube transcript via the durable yt-dlp audio + Whisper path.
    async fn fetch_youtube(&self, url: &str, step_timeout: Duration) -> Result<AdapterOutput> {
        self.ensure_compatible()?;

        let transcript = timeout(
            step_timeout,
            crate::ingest::youtube::acquire_youtube_transcript(url),
        )
        .await
        .with_context(|| format!("YouTube fetch timed out for URL: {}", url))?
        .with_context(|| format!("Failed to fetch YouTube content from: {}", url))?;

        let transcript_text = transcript.transcript;
        let mut output = AdapterOutput::new(transcript_text.clone())
            .with_artifact("transcript.txt", transcript_text);

        if let Some(transcript_json) = transcript.transcript_json {
            output = output.with_artifact("transcript.json", transcript_json);
        }

        Ok(output)
    }

    /// Fetch web page content via fabric -u <url>
    async fn fetch_web(&self, url: &str, step_timeout: Duration) -> Result<String> {
        self.ensure_compatible()?;

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
        let output = match action {
            ACTION_YOUTUBE => {
                // Input is the YouTube URL
                self.fetch_youtube(input, timeout).await?
            }
            ACTION_WEB => {
                // Input is the web URL
                AdapterOutput::new(self.fetch_web(input, timeout).await?)
            }
            _ => {
                // Standard pattern execution
                AdapterOutput::new(self.execute_subprocess(action, input, timeout).await?)
            }
        };

        Ok(output)
    }

    async fn health_check(&self) -> Result<()> {
        self.ensure_compatible()?;

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
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn write_executable(dir: &TempDir, name: &str, script: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, script).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }

    #[tokio::test]
    async fn test_fabric_adapter_creation() {
        let adapter = FabricAdapter::new();
        assert_eq!(adapter.name(), "fabric");
    }

    #[tokio::test]
    async fn test_custom_binary_path() {
        let adapter = FabricAdapter::with_binary_path("/custom/path/fabric");
        assert_eq!(adapter.binary_path, "/custom/path/fabric");
        assert!(!adapter.binary_diagnostics().signature_passed);
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
        assert!(FabricAdapter::should_alias_argv0(
            "/opt/homebrew/bin/fabric-ai"
        ));
        assert!(!FabricAdapter::should_alias_argv0("fabric"));
    }

    #[test]
    fn test_explicit_binary_accepts_compatible_ai_fabric_help() {
        let dir = TempDir::new().unwrap();
        let binary = write_executable(
            &dir,
            "fabric-ai",
            r#"#!/bin/sh
if [ "$1" = "--help" ]; then
  printf '%s\n' '--pattern --youtube --scrape_url'
  exit 0
fi
exit 0
"#,
        );

        let adapter = FabricAdapter::with_binary_path(binary.to_string_lossy());
        let diagnostics = adapter.binary_diagnostics();

        assert!(diagnostics.signature_passed);
        assert_eq!(
            diagnostics.selection_source,
            FabricBinarySelectionSource::ExplicitArgument
        );
    }

    #[test]
    fn test_explicit_binary_rejects_incompatible_help() {
        let dir = TempDir::new().unwrap();
        let binary = write_executable(
            &dir,
            "fabric",
            r#"#!/bin/sh
if [ "$1" = "--help" ]; then
  printf '%s\n' 'Usage: fabric run'
  exit 0
fi
exit 0
"#,
        );

        let adapter = FabricAdapter::with_binary_path(binary.to_string_lossy());
        let diagnostics = adapter.binary_diagnostics();

        assert!(!diagnostics.signature_passed);
        assert!(diagnostics
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("incompatible"));
    }

    // Note: Integration tests with actual Fabric would go in tests/
}

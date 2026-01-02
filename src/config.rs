//! Configuration for arkai paths.
//!
//! Configuration sources (highest priority first):
//! 1. Environment variables (ARKAI_HOME, ARKAI_LIBRARY)
//! 2. Config file (.arkai/config.yaml)
//! 3. Defaults (~/.arkai)
//!
//! Config file discovery:
//! - Searches current directory and parents for .arkai/config.yaml
//! - Paths in config file are relative to the config file's parent directory

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::library::content::ContentType;

/// Global cached configuration (stores Result to handle init errors)
static CONFIG: OnceLock<Result<ResolvedConfig, String>> = OnceLock::new();

/// Raw config file schema (matches YAML structure)
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub version: String,
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub fabric: Option<FabricConfig>,
    #[serde(default)]
    pub safety: Option<SafetyConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PathsConfig {
    /// Engine state directory (relative to config file)
    pub home: Option<String>,
    /// Library directory (relative to config file)
    pub library: Option<String>,
    /// Content type subdirectory mapping
    #[serde(default)]
    pub content_types: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FabricConfig {
    pub patterns_dir: Option<String>,
    pub custom_patterns: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SafetyConfig {
    pub max_steps: Option<u32>,
    pub timeout_seconds: Option<u64>,
    pub max_input_size_bytes: Option<usize>,
}

/// Resolved configuration with absolute paths
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    /// Absolute path to arkai home (engine state)
    pub home: PathBuf,
    /// Absolute path to library
    pub library: PathBuf,
    /// Content type to subdirectory mapping
    pub content_types: HashMap<String, String>,
    /// Path to config file (if found)
    pub config_file: Option<PathBuf>,
    /// Safety settings
    pub safety: SafetySettings,
}

#[derive(Debug, Clone)]
pub struct SafetySettings {
    pub max_steps: u32,
    pub timeout_seconds: u64,
    pub max_input_size_bytes: usize,
}

impl Default for SafetySettings {
    fn default() -> Self {
        Self {
            max_steps: 50,
            timeout_seconds: 600,
            max_input_size_bytes: 1_048_576, // 1MB
        }
    }
}

impl ResolvedConfig {
    /// Get content-type subdirectory for a given content type
    pub fn content_type_dir(&self, content_type: ContentType) -> PathBuf {
        let type_key = match content_type {
            ContentType::YouTube => "youtube",
            ContentType::Web => "articles", // web -> articles folder
            ContentType::Other => "other",
        };

        if let Some(subdir) = self.content_types.get(type_key) {
            self.library.join(subdir)
        } else {
            // Default: use content type name as subdirectory
            self.library.join(type_key)
        }
    }
}

/// Find config file by searching current directory and parents
fn find_config_file() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    loop {
        let config_path = current.join(".arkai").join("config.yaml");
        if config_path.exists() {
            return Some(config_path);
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Load and parse config file
fn load_config_file(path: &Path) -> Result<ConfigFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))
}

/// Resolve a path that may be relative to the config file's parent
fn resolve_path(base: &Path, path_str: &str) -> PathBuf {
    let path = PathBuf::from(path_str);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
            .canonicalize()
            .unwrap_or_else(|_| base.join(path_str))
    }
}

/// Load configuration from all sources
fn load_config() -> Result<ResolvedConfig> {
    // Default home directory
    let default_home = dirs::home_dir()
        .context("Failed to determine home directory")?
        .join(".arkai");

    // Check for config file
    let config_file = find_config_file();

    let (home, library, content_types, safety) = if let Some(ref config_path) = config_file {
        // Config file found - use it as base
        let config = load_config_file(config_path)?;

        // Base directory is the parent of .arkai/ (i.e., grandparent of config.yaml)
        let base_dir = config_path
            .parent() // .arkai/
            .and_then(|p| p.parent()) // project root
            .unwrap_or(Path::new("."));

        // Resolve home path
        let home = if let Ok(env_home) = std::env::var("ARKAI_HOME") {
            PathBuf::from(env_home)
        } else if let Some(ref home_path) = config.paths.home {
            // home is relative to .arkai/ directory
            let arkai_dir = config_path.parent().unwrap_or(Path::new("."));
            resolve_path(arkai_dir, home_path)
        } else {
            default_home.clone()
        };

        // Resolve library path
        let library = if let Ok(env_lib) = std::env::var("ARKAI_LIBRARY") {
            PathBuf::from(env_lib)
        } else if let Some(ref lib_path) = config.paths.library {
            resolve_path(base_dir, lib_path)
        } else {
            home.join("library")
        };

        // Content type mappings
        let content_types = config.paths.content_types;

        // Safety settings
        let safety = SafetySettings {
            max_steps: config
                .safety
                .as_ref()
                .and_then(|s| s.max_steps)
                .unwrap_or(50),
            timeout_seconds: config
                .safety
                .as_ref()
                .and_then(|s| s.timeout_seconds)
                .unwrap_or(600),
            max_input_size_bytes: config
                .safety
                .as_ref()
                .and_then(|s| s.max_input_size_bytes)
                .unwrap_or(1_048_576),
        };

        (home, library, content_types, safety)
    } else {
        // No config file - use env vars or defaults
        let home = std::env::var("ARKAI_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_home.clone());

        let library = std::env::var("ARKAI_LIBRARY")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join("library"));

        (home, library, HashMap::new(), SafetySettings::default())
    };

    Ok(ResolvedConfig {
        home,
        library,
        content_types,
        config_file,
        safety,
    })
}

/// Get the global configuration (loads once, then cached)
pub fn config() -> Result<&'static ResolvedConfig> {
    let result = CONFIG.get_or_init(|| {
        load_config().map_err(|e| e.to_string())
    });

    match result {
        Ok(config) => Ok(config),
        Err(e) => anyhow::bail!("{}", e),
    }
}

/// Force reload configuration (useful for testing)
pub fn reload_config() -> Result<ResolvedConfig> {
    load_config()
}

// ============================================================================
// Convenience functions (backward compatible API)
// ============================================================================

/// Get the arkai home directory (engine state).
pub fn arkai_home() -> Result<PathBuf> {
    Ok(config()?.home.clone())
}

/// Get the runs directory ($ARKAI_HOME/runs)
pub fn runs_dir() -> Result<PathBuf> {
    Ok(config()?.home.join("runs"))
}

/// Get the library directory.
pub fn library_dir() -> Result<PathBuf> {
    Ok(config()?.library.clone())
}

/// Get the catalog path ($ARKAI_HOME/catalog.json)
pub fn catalog_path() -> Result<PathBuf> {
    Ok(config()?.home.join("catalog.json"))
}

/// Get the content directory for a specific content type
pub fn content_type_dir(content_type: ContentType) -> Result<PathBuf> {
    Ok(config()?.content_type_dir(content_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_default_config_without_file() {
        // Without a config file or env vars, should use defaults
        let config = load_config().unwrap();

        // Should fall back to ~/.arkai
        let expected_home = dirs::home_dir().unwrap().join(".arkai");
        assert_eq!(config.home, expected_home);
        assert_eq!(config.library, expected_home.join("library"));
        assert!(config.config_file.is_none());
    }

    #[test]
    fn test_config_file_parsing() {
        let temp = TempDir::new().unwrap();
        let arkai_dir = temp.path().join(".arkai");
        std::fs::create_dir_all(&arkai_dir).unwrap();

        let config_path = arkai_dir.join("config.yaml");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
version: "1.0"
paths:
  home: ./
  library: ../library
  content_types:
    youtube: youtube
    articles: articles
safety:
  max_steps: 100
"#
        )
        .unwrap();

        let config = load_config_file(&config_path).unwrap();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.paths.home, Some("./".to_string()));
        assert_eq!(config.paths.library, Some("../library".to_string()));
        assert_eq!(
            config.paths.content_types.get("youtube"),
            Some(&"youtube".to_string())
        );
        assert_eq!(config.safety.unwrap().max_steps, Some(100));
    }

    #[test]
    fn test_content_type_dir_mapping() {
        let config = ResolvedConfig {
            home: PathBuf::from("/test/.arkai"),
            library: PathBuf::from("/test/library"),
            content_types: [
                ("youtube".to_string(), "yt-videos".to_string()),
                ("articles".to_string(), "web-articles".to_string()),
            ]
            .into_iter()
            .collect(),
            config_file: None,
            safety: SafetySettings::default(),
        };

        assert_eq!(
            config.content_type_dir(ContentType::YouTube),
            PathBuf::from("/test/library/yt-videos")
        );
        assert_eq!(
            config.content_type_dir(ContentType::Web),
            PathBuf::from("/test/library/web-articles")
        );
        // Other falls back to default name
        assert_eq!(
            config.content_type_dir(ContentType::Other),
            PathBuf::from("/test/library/other")
        );
    }

    #[test]
    fn test_resolve_relative_path() {
        let base = PathBuf::from("/home/user/project");

        assert_eq!(
            resolve_path(&base, "./subdir"),
            PathBuf::from("/home/user/project/subdir")
        );
        assert_eq!(
            resolve_path(&base, "../sibling"),
            PathBuf::from("/home/user/project/../sibling")
        );
        assert_eq!(
            resolve_path(&base, "/absolute/path"),
            PathBuf::from("/absolute/path")
        );
    }
}

//! Content storage for the library.
//!
//! Manages the storage and retrieval of processed content artifacts.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;

/// Content identifier (SHA256(url)[0:16])
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentId(String);

impl ContentId {
    /// Create a content ID from a URL
    pub fn from_url(url: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let result = hasher.finalize();

        // Take first 8 bytes (16 hex chars)
        let hash: String = result[..8].iter().map(|b| format!("{:02x}", b)).collect();
        Self(hash)
    }

    /// Get the raw string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    /// YouTube video
    YouTube,

    /// Web page/article
    Web,

    /// Other/generic content
    Other,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::YouTube => write!(f, "youtube"),
            ContentType::Web => write!(f, "web"),
            ContentType::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for ContentType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "youtube" | "yt" => Ok(ContentType::YouTube),
            "web" | "webpage" | "article" => Ok(ContentType::Web),
            "other" => Ok(ContentType::Other),
            _ => anyhow::bail!("Unknown content type: {}", s),
        }
    }
}

/// Library content with storage operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryContent {
    /// Content identifier
    pub id: ContentId,

    /// Human-readable title
    pub title: String,

    /// Original source URL
    pub url: String,

    /// Type of content
    pub content_type: ContentType,

    /// When the content was processed
    pub processed_at: DateTime<Utc>,

    /// User-provided tags
    #[serde(default)]
    pub tags: Vec<String>,
}

impl LibraryContent {
    /// Create new library content
    pub fn new(url: impl Into<String>, title: impl Into<String>, content_type: ContentType) -> Self {
        let url = url.into();
        Self {
            id: ContentId::from_url(&url),
            title: title.into(),
            url,
            content_type,
            processed_at: Utc::now(),
            tags: Vec::new(),
        }
    }

    /// Get the base library directory
    pub fn library_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to determine home directory")?;
        Ok(home.join(".arkai").join("library"))
    }

    /// Get the content directory for this item
    pub fn content_dir(&self) -> Result<PathBuf> {
        Ok(Self::library_dir()?.join(self.id.as_str()))
    }

    /// Get the path to a specific artifact
    pub fn artifact_path(&self, artifact_name: &str) -> Result<PathBuf> {
        Ok(self.content_dir()?.join(format!("{}.md", artifact_name)))
    }

    /// Get the metadata file path
    pub fn metadata_path(&self) -> Result<PathBuf> {
        Ok(self.content_dir()?.join("metadata.json"))
    }

    /// Ensure the content directory exists
    pub async fn ensure_dir(&self) -> Result<PathBuf> {
        let dir = self.content_dir()?;
        fs::create_dir_all(&dir)
            .await
            .with_context(|| format!("Failed to create content directory: {}", dir.display()))?;
        Ok(dir)
    }

    /// Save metadata to disk
    pub async fn save_metadata(&self) -> Result<()> {
        self.ensure_dir().await?;

        let path = self.metadata_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)
            .await
            .with_context(|| format!("Failed to write metadata: {}", path.display()))?;

        Ok(())
    }

    /// Load metadata from disk
    pub async fn load_metadata(id: &ContentId) -> Result<Self> {
        let path = Self::library_dir()?.join(id.as_str()).join("metadata.json");

        let content = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read metadata: {}", path.display()))?;

        serde_json::from_str(&content).context("Failed to parse metadata JSON")
    }

    /// Store an artifact
    pub async fn store_artifact(&self, name: &str, content: &str) -> Result<PathBuf> {
        self.ensure_dir().await?;

        let path = self.artifact_path(name)?;
        fs::write(&path, content)
            .await
            .with_context(|| format!("Failed to write artifact: {}", path.display()))?;

        Ok(path)
    }

    /// Load an artifact
    pub async fn load_artifact(&self, name: &str) -> Result<Option<String>> {
        let path = self.artifact_path(name)?;

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read artifact: {}", path.display()))?;

        Ok(Some(content))
    }

    /// List all artifacts for this content
    pub async fn list_artifacts(&self) -> Result<Vec<String>> {
        let dir = self.content_dir()?;

        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut artifacts = Vec::new();
        let mut entries = fs::read_dir(&dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".md") {
                    artifacts.push(name.trim_end_matches(".md").to_string());
                }
            }
        }

        Ok(artifacts)
    }

    /// Check if content exists in the library
    pub async fn exists(id: &ContentId) -> Result<bool> {
        let path = Self::library_dir()?.join(id.as_str()).join("metadata.json");
        Ok(path.exists())
    }

    /// Copy artifacts from a run to the library
    pub async fn copy_from_run(&self, run_id: uuid::Uuid) -> Result<Vec<String>> {
        let run_artifacts_dir = dirs::home_dir()
            .context("Failed to determine home directory")?
            .join(".arkai")
            .join("runs")
            .join(run_id.to_string())
            .join("artifacts");

        if !run_artifacts_dir.exists() {
            return Ok(Vec::new());
        }

        let mut copied = Vec::new();
        let mut entries = fs::read_dir(&run_artifacts_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".md") {
                    let artifact_name = name.trim_end_matches(".md");
                    let content = fs::read_to_string(entry.path()).await?;
                    self.store_artifact(artifact_name, &content).await?;
                    copied.push(artifact_name.to_string());
                }
            }
        }

        Ok(copied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_id_from_url() {
        let id1 = ContentId::from_url("https://youtube.com/watch?v=abc123");
        let id2 = ContentId::from_url("https://youtube.com/watch?v=abc123");
        let id3 = ContentId::from_url("https://youtube.com/watch?v=xyz789");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_eq!(id1.as_str().len(), 16); // 8 bytes = 16 hex chars
    }

    #[test]
    fn test_content_type_from_str() {
        assert_eq!(
            "youtube".parse::<ContentType>().unwrap(),
            ContentType::YouTube
        );
        assert_eq!("yt".parse::<ContentType>().unwrap(), ContentType::YouTube);
        assert_eq!("web".parse::<ContentType>().unwrap(), ContentType::Web);
        assert_eq!(
            "webpage".parse::<ContentType>().unwrap(),
            ContentType::Web
        );
        assert!("invalid".parse::<ContentType>().is_err());
    }

    #[test]
    fn test_library_content_creation() {
        let content = LibraryContent::new(
            "https://youtube.com/watch?v=abc",
            "Test Video",
            ContentType::YouTube,
        );

        assert_eq!(content.title, "Test Video");
        assert_eq!(content.url, "https://youtube.com/watch?v=abc");
        assert_eq!(content.content_type, ContentType::YouTube);
    }
}

//! Catalog for tracking all processed content.
//!
//! Simple JSON-based index that can be searched and filtered.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

use super::content::{ContentId, ContentType};

/// Catalog of all processed content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    /// Catalog format version
    pub version: u32,

    /// All cataloged items
    pub items: Vec<CatalogItem>,
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

impl Catalog {
    /// Create a new empty catalog
    pub fn new() -> Self {
        Self {
            version: 1,
            items: Vec::new(),
        }
    }

    /// Get the catalog file path
    pub fn catalog_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to determine home directory")?;
        Ok(home.join(".arkai").join("catalog.json"))
    }

    /// Load the catalog from disk
    pub async fn load() -> Result<Self> {
        let path = Self::catalog_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read catalog: {}", path.display()))?;

        serde_json::from_str(&content).context("Failed to parse catalog JSON")
    }

    /// Save the catalog to disk
    pub async fn save(&self) -> Result<()> {
        let path = Self::catalog_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)
            .await
            .with_context(|| format!("Failed to write catalog: {}", path.display()))?;

        Ok(())
    }

    /// Add an item to the catalog
    pub fn add(&mut self, item: CatalogItem) {
        // Check for duplicates by content_id
        if let Some(existing) = self.items.iter_mut().find(|i| i.id == item.id) {
            // Update existing item
            *existing = item;
        } else {
            self.items.push(item);
        }
    }

    /// Get an item by ID
    pub fn get(&self, id: &ContentId) -> Option<&CatalogItem> {
        self.items.iter().find(|i| &i.id == id)
    }

    /// Remove an item by ID
    pub fn remove(&mut self, id: &ContentId) -> Option<CatalogItem> {
        if let Some(pos) = self.items.iter().position(|i| &i.id == id) {
            Some(self.items.remove(pos))
        } else {
            None
        }
    }

    /// Search items by query (case-insensitive substring match)
    pub fn search(&self, query: &str) -> Vec<&CatalogItem> {
        let query_lower = query.to_lowercase();

        self.items
            .iter()
            .filter(|item| {
                item.title.to_lowercase().contains(&query_lower)
                    || item.url.to_lowercase().contains(&query_lower)
                    || item.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Filter items by content type
    pub fn filter_by_type(&self, content_type: ContentType) -> Vec<&CatalogItem> {
        self.items
            .iter()
            .filter(|item| item.content_type == content_type)
            .collect()
    }

    /// Get all items sorted by processed_at (most recent first)
    pub fn list(&self, limit: Option<usize>) -> Vec<&CatalogItem> {
        let mut items: Vec<_> = self.items.iter().collect();
        items.sort_by(|a, b| b.processed_at.cmp(&a.processed_at));

        if let Some(limit) = limit {
            items.truncate(limit);
        }

        items
    }

    /// Get the number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the catalog is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// A single item in the catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogItem {
    /// Unique content identifier (SHA256(url)[0:16])
    pub id: ContentId,

    /// Human-readable title
    pub title: String,

    /// Original source URL
    pub url: String,

    /// Type of content
    pub content_type: ContentType,

    /// When the content was processed
    pub processed_at: DateTime<Utc>,

    /// User-provided or extracted tags
    #[serde(default)]
    pub tags: Vec<String>,

    /// Available artifact files
    #[serde(default)]
    pub artifacts: Vec<String>,

    /// Run ID that produced this content (for traceability)
    pub run_id: Option<String>,
}

impl CatalogItem {
    /// Create a new catalog item
    pub fn new(
        url: impl Into<String>,
        title: impl Into<String>,
        content_type: ContentType,
    ) -> Self {
        let url = url.into();
        let id = ContentId::from_url(&url);

        Self {
            id,
            title: title.into(),
            url,
            content_type,
            processed_at: Utc::now(),
            tags: Vec::new(),
            artifacts: Vec::new(),
            run_id: None,
        }
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    /// Add an artifact
    pub fn with_artifact(mut self, artifact: impl Into<String>) -> Self {
        self.artifacts.push(artifact.into());
        self
    }

    /// Set the run ID
    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_add_and_get() {
        let mut catalog = Catalog::new();
        let item = CatalogItem::new(
            "https://youtube.com/watch?v=abc123",
            "Test Video",
            ContentType::YouTube,
        );

        let id = item.id.clone();
        catalog.add(item);

        assert_eq!(catalog.len(), 1);
        assert!(catalog.get(&id).is_some());
    }

    #[test]
    fn test_catalog_search() {
        let mut catalog = Catalog::new();

        catalog.add(
            CatalogItem::new(
                "https://youtube.com/watch?v=abc123",
                "Introduction to Rust",
                ContentType::YouTube,
            )
            .with_tag("programming")
            .with_tag("rust"),
        );

        catalog.add(
            CatalogItem::new(
                "https://example.com/article",
                "Web Development Tips",
                ContentType::Web,
            )
            .with_tag("web"),
        );

        // Search by title
        let results = catalog.search("rust");
        assert_eq!(results.len(), 1);

        // Search by tag
        let results = catalog.search("programming");
        assert_eq!(results.len(), 1);

        // Case insensitive
        let results = catalog.search("RUST");
        assert_eq!(results.len(), 1);

        // No match
        let results = catalog.search("python");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_catalog_filter_by_type() {
        let mut catalog = Catalog::new();

        catalog.add(CatalogItem::new(
            "https://youtube.com/1",
            "Video 1",
            ContentType::YouTube,
        ));
        catalog.add(CatalogItem::new(
            "https://youtube.com/2",
            "Video 2",
            ContentType::YouTube,
        ));
        catalog.add(CatalogItem::new(
            "https://example.com/1",
            "Article 1",
            ContentType::Web,
        ));

        let youtube = catalog.filter_by_type(ContentType::YouTube);
        assert_eq!(youtube.len(), 2);

        let web = catalog.filter_by_type(ContentType::Web);
        assert_eq!(web.len(), 1);
    }

    #[test]
    fn test_catalog_remove() {
        let mut catalog = Catalog::new();
        let item = CatalogItem::new("https://example.com", "Test", ContentType::Web);
        let id = item.id.clone();

        catalog.add(item);
        assert_eq!(catalog.len(), 1);

        let removed = catalog.remove(&id);
        assert!(removed.is_some());
        assert_eq!(catalog.len(), 0);
    }
}

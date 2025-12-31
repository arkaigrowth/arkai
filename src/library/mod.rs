//! Content library management for processed artifacts.
//!
//! The library stores processed content (YouTube videos, web pages, etc.)
//! in an organized, searchable structure.
//!
//! # Storage Layout
//!
//! ```text
//! ~/.arkai/
//! ├── catalog.json              # Index of all processed content
//! └── library/
//!     └── <content_id>/         # SHA256(url)[0:16]
//!         ├── metadata.json     # Title, URL, type, processed_at
//!         ├── source.md         # Original transcript/markdown
//!         ├── wisdom.md         # extract_wisdom output
//!         └── summary.md        # summarize output
//! ```

pub mod catalog;
pub mod content;

pub use catalog::{Catalog, CatalogItem};
pub use content::{ContentId, ContentType, LibraryContent};

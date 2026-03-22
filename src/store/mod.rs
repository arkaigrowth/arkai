//! Canonical SQLite store for arkai.
//!
//! This is arkai's unified query layer. The JSONL event logs remain the
//! append-only source of truth for pipeline runs. This store provides:
//! - Indexed catalog (replaces catalog.json)
//! - Cross-content entity resolution
//! - Evidence persistence with provenance
//! - Embedding storage for semantic search
//! - Schema migrations for forward compatibility
//!
//! # Design Decisions
//!
//! - `rusqlite` (not sqlx): synchronous is fine for a local CLI tool
//! - Bundled SQLite: no system dependency, reproducible builds
//! - WAL mode: safe concurrent reads from multiple processes
//! - Schema migrations table: prevents the #1 SQLite tech debt pattern
//! - Embedding model metadata in config table: model swaps are config changes

pub mod capture;
pub mod chunking;
pub mod db;
pub mod embedding;
pub mod import;
pub mod migrations;
pub mod queries;
pub mod search;

pub use db::{Store, StoreConfig};
pub use embedding::{cosine_similarity, EmbeddingProvider, OllamaProvider};
pub use search::{hybrid_search, vector_search, HybridSearchResult};

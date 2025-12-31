//! Adapter interfaces for external systems.
//!
//! Adapters provide a unified interface for interacting with external
//! AI services like Fabric.

pub mod fabric;

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

// Re-export the Fabric adapter
pub use fabric::FabricAdapter;

/// Output from an adapter execution
#[derive(Debug, Clone)]
pub struct AdapterOutput {
    /// The content returned by the adapter
    pub content: String,

    /// Tokens used (if available)
    pub tokens_used: Option<u64>,

    /// Cost in USD (if available)
    pub cost_usd: Option<f64>,
}

impl AdapterOutput {
    /// Create a new adapter output with just content
    pub fn new(content: String) -> Self {
        Self {
            content,
            tokens_used: None,
            cost_usd: None,
        }
    }
}

/// Trait for external adapters
#[async_trait]
pub trait Adapter: Send + Sync {
    /// Human-readable adapter name
    fn name(&self) -> &str;

    /// Execute an action with input
    async fn execute(
        &self,
        action: &str,
        input: &str,
        timeout: Duration,
    ) -> Result<AdapterOutput>;

    /// Health check (for HTTP adapters)
    async fn health_check(&self) -> Result<()>;
}

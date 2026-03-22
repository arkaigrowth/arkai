//! Embedding provider for vector similarity operations.
//!
//! Provides a trait-based abstraction over embedding models, with a concrete
//! implementation for the local Ollama API. Designed for extensibility —
//! additional providers (OpenAI, etc.) can implement `EmbeddingProvider`.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a single text string, returning a float vector.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for the embedding subsystem, typically read from the
/// `store_config` table keyed under `embedding.*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider name, e.g. "ollama".
    pub provider: String,
    /// Model identifier understood by the provider.
    pub model: String,
    /// Expected dimensionality of the output vectors.
    pub dimensions: usize,
    /// Base URL for the provider API.
    #[serde(default = "default_ollama_url")]
    pub base_url: String,
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

impl EmbeddingConfig {
    /// Build an `EmbeddingConfig` from key-value pairs typically stored in the
    /// `store_config` table. Expected keys:
    ///
    /// - `embedding.provider`   (e.g. "ollama")
    /// - `embedding.model`      (e.g. "mxbai-embed-large")
    /// - `embedding.dimensions` (e.g. "1024")
    /// - `embedding.base_url`   (optional, defaults to localhost:11434)
    pub fn from_store_config(pairs: &[(String, String)]) -> Result<Self> {
        let get = |key: &str| -> Option<String> {
            pairs
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        };

        let provider = get("embedding.provider")
            .ok_or_else(|| anyhow!("missing store_config key: embedding.provider"))?;
        let model = get("embedding.model")
            .ok_or_else(|| anyhow!("missing store_config key: embedding.model"))?;
        let dimensions: usize = get("embedding.dimensions")
            .ok_or_else(|| anyhow!("missing store_config key: embedding.dimensions"))?
            .parse()
            .context("embedding.dimensions must be a positive integer")?;
        let base_url = get("embedding.base_url").unwrap_or_else(default_ollama_url);

        Ok(Self {
            provider,
            model,
            dimensions,
            base_url,
        })
    }
}

// ---------------------------------------------------------------------------
// Ollama provider
// ---------------------------------------------------------------------------

/// Calls the local Ollama REST API to produce embeddings.
#[derive(Debug)]
pub struct OllamaProvider {
    client: reqwest::Client,
    config: EmbeddingConfig,
}

#[derive(Serialize)]
struct OllamaEmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

impl OllamaProvider {
    /// Create a provider from an already-parsed config.
    pub fn new(config: EmbeddingConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self { client, config }
    }

    /// Convenience: build directly from store_config key-value pairs.
    pub fn from_store_config(pairs: &[(String, String)]) -> Result<Self> {
        let config = EmbeddingConfig::from_store_config(pairs)?;
        if config.provider != "ollama" {
            return Err(anyhow!(
                "OllamaProvider requires provider=\"ollama\", got \"{}\"",
                config.provider
            ));
        }
        Ok(Self::new(config))
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embed", self.config.base_url);

        let body = OllamaEmbedRequest {
            model: &self.config.model,
            input: text,
        };

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    anyhow!(
                        "cannot connect to Ollama at {} — is it running? ({})",
                        self.config.base_url,
                        e
                    )
                } else if e.is_timeout() {
                    anyhow!("Ollama request timed out — model may be loading")
                } else {
                    anyhow!("Ollama HTTP error: {}", e)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            if body_text.contains("not found") || body_text.contains("pull") {
                return Err(anyhow!(
                    "model \"{}\" not available in Ollama — run: ollama pull {}",
                    self.config.model,
                    self.config.model
                ));
            }
            return Err(anyhow!(
                "Ollama returned HTTP {}: {}",
                status.as_u16(),
                body_text
            ));
        }

        let parsed: OllamaEmbedResponse = response
            .json()
            .await
            .context("failed to parse Ollama embed response")?;

        let vec = parsed
            .embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Ollama returned empty embeddings array"))?;

        if vec.len() != self.config.dimensions {
            return Err(anyhow!(
                "dimension mismatch: expected {}, got {} — check embedding.dimensions config",
                self.config.dimensions,
                vec.len()
            ));
        }

        Ok(vec)
    }
}

// ---------------------------------------------------------------------------
// Vector utilities
// ---------------------------------------------------------------------------

/// Cosine similarity between two vectors. Returns a value in [-1.0, 1.0].
///
/// Returns 0.0 if either vector has zero magnitude (avoids division by zero).
/// Panics in debug mode if vectors have different lengths.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "vectors must have equal length");

    let mut dot = 0.0_f32;
    let mut mag_a = 0.0_f32;
    let mut mag_b = 0.0_f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        mag_a += x * x;
        mag_b += y * y;
    }

    let denom = mag_a.sqrt() * mag_b.sqrt();
    if denom == 0.0 {
        return 0.0;
    }
    dot / denom
}

/// Normalize a vector to unit length in-place. No-op for zero vectors.
pub fn normalize(v: &mut [f32]) {
    let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag > 0.0 {
        for x in v.iter_mut() {
            *x /= mag;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- cosine_similarity -------------------------------------------------

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6, "identical vectors should have similarity 1.0");
    }

    #[test]
    fn cosine_opposite_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-6, "opposite vectors should have similarity -1.0");
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "orthogonal vectors should have similarity ~0.0");
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        let a = vec![1.0, 2.0, 3.0];
        let zero = vec![0.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &zero), 0.0);
        assert_eq!(cosine_similarity(&zero, &a), 0.0);
        assert_eq!(cosine_similarity(&zero, &zero), 0.0);
    }

    #[test]
    fn cosine_scaled_vectors_equal() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![2.0, 4.0, 6.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "scaled vectors should have similarity 1.0, got {}",
            sim
        );
    }

    #[test]
    fn cosine_known_angle() {
        // 45-degree angle: cos(pi/4) ~ 0.7071
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        let expected = std::f32::consts::FRAC_1_SQRT_2;
        assert!(
            (sim - expected).abs() < 1e-5,
            "expected ~{}, got {}",
            expected,
            sim
        );
    }

    // -- normalize ---------------------------------------------------------

    #[test]
    fn normalize_produces_unit_vector() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (mag - 1.0).abs() < 1e-6,
            "normalized vector should have magnitude 1.0, got {}",
            mag
        );
        assert!((v[0] - 0.6).abs() < 1e-6);
        assert!((v[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn normalize_zero_vector_unchanged() {
        let mut v = vec![0.0, 0.0, 0.0];
        normalize(&mut v);
        assert!(v.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn normalize_already_unit() {
        let mut v = vec![1.0, 0.0, 0.0];
        normalize(&mut v);
        assert!((v[0] - 1.0).abs() < 1e-6);
        assert!(v[1].abs() < 1e-6);
        assert!(v[2].abs() < 1e-6);
    }

    #[test]
    fn cosine_after_normalize() {
        let mut a = vec![3.0, 4.0, 0.0];
        let mut b = vec![3.0, 4.0, 0.0];
        let sim_before = cosine_similarity(&a, &b);
        normalize(&mut a);
        normalize(&mut b);
        let sim_after = cosine_similarity(&a, &b);
        assert!(
            (sim_before - sim_after).abs() < 1e-6,
            "normalization should not change cosine similarity"
        );
    }

    // -- EmbeddingConfig ---------------------------------------------------

    #[test]
    fn config_from_store_config_complete() {
        let pairs = vec![
            ("embedding.provider".into(), "ollama".into()),
            ("embedding.model".into(), "mxbai-embed-large".into()),
            ("embedding.dimensions".into(), "1024".into()),
        ];
        let cfg = EmbeddingConfig::from_store_config(&pairs).unwrap();
        assert_eq!(cfg.provider, "ollama");
        assert_eq!(cfg.model, "mxbai-embed-large");
        assert_eq!(cfg.dimensions, 1024);
        assert_eq!(cfg.base_url, "http://localhost:11434");
    }

    #[test]
    fn config_from_store_config_custom_url() {
        let pairs = vec![
            ("embedding.provider".into(), "ollama".into()),
            ("embedding.model".into(), "nomic-embed-text".into()),
            ("embedding.dimensions".into(), "768".into()),
            ("embedding.base_url".into(), "http://gpu-box:11434".into()),
        ];
        let cfg = EmbeddingConfig::from_store_config(&pairs).unwrap();
        assert_eq!(cfg.base_url, "http://gpu-box:11434");
        assert_eq!(cfg.dimensions, 768);
    }

    #[test]
    fn config_missing_provider_errors() {
        let pairs = vec![
            ("embedding.model".into(), "mxbai-embed-large".into()),
            ("embedding.dimensions".into(), "1024".into()),
        ];
        let err = EmbeddingConfig::from_store_config(&pairs).unwrap_err();
        assert!(
            err.to_string().contains("embedding.provider"),
            "error should mention missing key: {}",
            err
        );
    }

    #[test]
    fn config_missing_model_errors() {
        let pairs = vec![
            ("embedding.provider".into(), "ollama".into()),
            ("embedding.dimensions".into(), "1024".into()),
        ];
        let err = EmbeddingConfig::from_store_config(&pairs).unwrap_err();
        assert!(err.to_string().contains("embedding.model"));
    }

    #[test]
    fn config_missing_dimensions_errors() {
        let pairs = vec![
            ("embedding.provider".into(), "ollama".into()),
            ("embedding.model".into(), "mxbai-embed-large".into()),
        ];
        let err = EmbeddingConfig::from_store_config(&pairs).unwrap_err();
        assert!(err.to_string().contains("embedding.dimensions"));
    }

    #[test]
    fn config_bad_dimensions_errors() {
        let pairs = vec![
            ("embedding.provider".into(), "ollama".into()),
            ("embedding.model".into(), "mxbai-embed-large".into()),
            ("embedding.dimensions".into(), "not_a_number".into()),
        ];
        let err = EmbeddingConfig::from_store_config(&pairs).unwrap_err();
        assert!(err.to_string().contains("integer"));
    }

    #[test]
    fn config_ignores_unrelated_keys() {
        let pairs = vec![
            ("embedding.provider".into(), "ollama".into()),
            ("embedding.model".into(), "mxbai-embed-large".into()),
            ("embedding.dimensions".into(), "1024".into()),
            ("store.path".into(), "/tmp/db".into()),
            ("unrelated.key".into(), "value".into()),
        ];
        let cfg = EmbeddingConfig::from_store_config(&pairs).unwrap();
        assert_eq!(cfg.provider, "ollama");
    }

    // -- OllamaProvider construction ---------------------------------------

    #[test]
    fn ollama_provider_rejects_wrong_provider() {
        let pairs = vec![
            ("embedding.provider".into(), "openai".into()),
            ("embedding.model".into(), "text-embedding-3-small".into()),
            ("embedding.dimensions".into(), "1536".into()),
        ];
        let err = OllamaProvider::from_store_config(&pairs).unwrap_err();
        assert!(err.to_string().contains("ollama"));
    }

    #[test]
    fn ollama_provider_constructs_from_valid_config() {
        let pairs = vec![
            ("embedding.provider".into(), "ollama".into()),
            ("embedding.model".into(), "mxbai-embed-large".into()),
            ("embedding.dimensions".into(), "1024".into()),
        ];
        let provider = OllamaProvider::from_store_config(&pairs).unwrap();
        assert_eq!(provider.config.model, "mxbai-embed-large");
        assert_eq!(provider.config.dimensions, 1024);
    }
}

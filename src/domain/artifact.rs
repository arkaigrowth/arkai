//! Artifacts produced by pipeline steps.
//!
//! Artifacts are the outputs of steps that can be used as inputs to subsequent steps.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An artifact produced by a pipeline step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Name of the step that produced this artifact
    pub step_name: String,

    /// Type of artifact
    pub artifact_type: ArtifactType,

    /// Content or path to the artifact
    pub content: String,

    /// When the artifact was created
    pub created_at: DateTime<Utc>,

    /// Size in bytes (for tracking)
    pub size_bytes: u64,
}

impl Artifact {
    /// Create a new artifact
    pub fn new(step_name: String, artifact_type: ArtifactType, content: String) -> Self {
        let size_bytes = content.len() as u64;
        Self {
            step_name,
            artifact_type,
            content,
            created_at: Utc::now(),
            size_bytes,
        }
    }

    /// Create an artifact from step output
    pub fn from_output(step_name: String, output: String) -> Self {
        Self::new(step_name, ArtifactType::StepOutput, output)
    }
}

/// Types of artifacts that can be produced
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// Raw output from a step
    StepOutput,

    /// Transcript (e.g., from YouTube)
    Transcript,

    /// Extracted wisdom/insights
    Wisdom,

    /// Summary of content
    Summary,

    /// List of tasks
    TaskList,

    /// Reference to external document (e.g., RAGFlow doc ID)
    DocumentReference,
}

impl Default for ArtifactType {
    fn default() -> Self {
        Self::StepOutput
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_creation() {
        let artifact = Artifact::new(
            "summarize".to_string(),
            ArtifactType::Summary,
            "This is a summary of the input.".to_string(),
        );

        assert_eq!(artifact.step_name, "summarize");
        assert_eq!(artifact.artifact_type, ArtifactType::Summary);
        assert_eq!(artifact.size_bytes, 31);
    }

    #[test]
    fn test_artifact_serialization() {
        let artifact = Artifact::from_output("test".to_string(), "output content".to_string());

        let json = serde_json::to_string(&artifact).unwrap();
        let parsed: Artifact = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.step_name, "test");
        assert_eq!(parsed.content, "output content");
    }
}

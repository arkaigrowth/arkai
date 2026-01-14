//! Evidence and span data types
//!
//! These types represent the evidence.jsonl and entities.json schemas.

use serde::{Deserialize, Serialize};

/// Resolution status for a quote match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Exactly one exact match found, span computed
    Resolved,
    /// Multiple exact matches found, deterministic selection made
    Ambiguous,
    /// No exact match found, no span
    Unresolved,
}

/// Method used to resolve the quote
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionMethod {
    /// Exact byte match found
    Exact,
    /// No match found
    None,
    /// Normalized match found but no span generated (hint only)
    NormalizedHint,
}

/// Reason for unresolved status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnresolvedReason {
    /// No match found at all
    NoMatch,
    /// Multiple matches found
    MultipleMatches,
    /// Only normalized match found
    NormalizedMatchOnly,
}

/// Resolution details for a quote match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    /// Method used to resolve
    pub method: ResolutionMethod,
    /// Number of matches found
    pub match_count: usize,
    /// Rank of selected match (1-indexed)
    pub match_rank: usize,
    /// Reason if unresolved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<UnresolvedReason>,
}

/// A span in an artifact file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Artifact file path (relative to content directory)
    pub artifact: String,
    /// UTF-8 byte offset range [start, end]
    pub utf8_byte_offset: [usize; 2],
    /// SHA256 hash of the slice bytes
    pub slice_sha256: String,
    /// Context around the span (~80 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_text: Option<String>,
    /// Video timestamp if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_timestamp: Option<String>,
}

/// An evidence line in evidence.jsonl
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Deterministic evidence ID
    pub id: String,
    /// Content ID this evidence belongs to
    pub content_id: String,
    /// The extracted claim
    pub claim: String,
    /// Verbatim quote from transcript
    pub quote: String,
    /// SHA256 hash of the quote
    pub quote_sha256: String,
    /// Resolution status
    pub status: Status,
    /// Resolution details
    pub resolution: Resolution,
    /// Span in artifact (present if resolved/ambiguous)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    /// Confidence score from extractor
    pub confidence: f64,
    /// Name of the extraction pattern
    pub extractor: String,
    /// Timestamp when evidence was created
    pub ts: String,
}

/// A mention of an entity in the transcript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMention {
    /// Verbatim quote where entity is mentioned
    pub quote: String,
    /// SHA256 hash of the quote
    pub quote_sha256: String,
    /// Resolution status
    pub status: Status,
    /// Resolution details
    pub resolution: Resolution,
    /// Span in artifact (present if resolved/ambiguous)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

/// An extracted entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Entity name
    pub name: String,
    /// Entity type (person, org, concept, product, location, event)
    #[serde(rename = "type")]
    pub entity_type: String,
    /// Confidence score from extractor
    pub confidence: f64,
    /// Mentions of this entity in the transcript
    #[serde(default)]
    pub mentions: Vec<EntityMention>,
}

/// The entities.json file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitiesFile {
    /// Schema version
    pub schema_version: u32,
    /// Name of the extraction pattern
    pub extracted_by: String,
    /// Timestamp when entities were extracted
    pub extracted_at: String,
    /// List of extracted entities
    pub entities: Vec<Entity>,
}

/// Evidence-related events for events.jsonl
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EvidenceEvent {
    /// Evidence line was appended
    EvidenceAppended {
        content_id: String,
        evidence_id: String,
        status: Status,
        extractor: String,
    },
    /// Evidence was validated for a content item
    EvidenceValidated {
        content_id: String,
        artifact: String,
        digest_ok: bool,
        valid_count: usize,
        stale_count: usize,
        unresolved_count: usize,
    },
}

impl Evidence {
    /// Create a new resolved evidence entry
    pub fn new_resolved(
        id: String,
        content_id: String,
        claim: String,
        quote: String,
        quote_sha256: String,
        span: Span,
        confidence: f64,
        extractor: String,
        ts: String,
    ) -> Self {
        Self {
            id,
            content_id,
            claim,
            quote,
            quote_sha256,
            status: Status::Resolved,
            resolution: Resolution {
                method: ResolutionMethod::Exact,
                match_count: 1,
                match_rank: 1,
                reason: None,
            },
            span: Some(span),
            confidence,
            extractor,
            ts,
        }
    }

    /// Create a new ambiguous evidence entry
    pub fn new_ambiguous(
        id: String,
        content_id: String,
        claim: String,
        quote: String,
        quote_sha256: String,
        span: Span,
        match_count: usize,
        confidence: f64,
        extractor: String,
        ts: String,
    ) -> Self {
        Self {
            id,
            content_id,
            claim,
            quote,
            quote_sha256,
            status: Status::Ambiguous,
            resolution: Resolution {
                method: ResolutionMethod::Exact,
                match_count,
                match_rank: 1,
                reason: Some(UnresolvedReason::MultipleMatches),
            },
            span: Some(span),
            confidence,
            extractor,
            ts,
        }
    }

    /// Create a new unresolved evidence entry
    pub fn new_unresolved(
        id: String,
        content_id: String,
        claim: String,
        quote: String,
        quote_sha256: String,
        normalized_hint: bool,
        confidence: f64,
        extractor: String,
        ts: String,
    ) -> Self {
        let (method, reason) = if normalized_hint {
            (
                ResolutionMethod::NormalizedHint,
                UnresolvedReason::NormalizedMatchOnly,
            )
        } else {
            (ResolutionMethod::None, UnresolvedReason::NoMatch)
        };

        Self {
            id,
            content_id,
            claim,
            quote,
            quote_sha256,
            status: Status::Unresolved,
            resolution: Resolution {
                method,
                match_count: 0,
                match_rank: 0,
                reason: Some(reason),
            },
            span: None,
            confidence,
            extractor,
            ts,
        }
    }
}

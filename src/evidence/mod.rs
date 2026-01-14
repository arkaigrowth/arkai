//! Evidence and provenance system for grounding extracted claims
//!
//! This module provides tools for creating auditable, verifiable evidence
//! from extracted claims and entities. Every claim is grounded to a specific
//! span in the source transcript with cryptographic verification.
//!
//! # Design Principles
//!
//! - **Honest unresolved**: Never generate wrong spans. If no exact match, record unresolved.
//! - **Append-only**: Evidence is stored in JSONL format, never modified.
//! - **Hash verification**: Each span includes slice_sha256 for drift detection.
//! - **Deterministic IDs**: Same input always produces same evidence ID.
//!
//! # Example
//!
//! ```ignore
//! use arkai::evidence::{spans, types::Evidence};
//!
//! // Find quote in transcript
//! let result = spans::find_quote(&transcript, &quote);
//!
//! // Create evidence based on match result
//! let evidence = match result.status() {
//!     spans::MatchStatus::Resolved => {
//!         let (start, end) = result.selected_match().unwrap();
//!         Evidence::new_resolved(/* ... */)
//!     }
//!     spans::MatchStatus::Ambiguous => {
//!         Evidence::new_ambiguous(/* ... */)
//!     }
//!     spans::MatchStatus::Unresolved => {
//!         Evidence::new_unresolved(/* ... */)
//!     }
//! };
//! ```

pub mod spans;
pub mod types;

pub use spans::{
    compute_evidence_id, compute_hash, compute_slice_hash, extract_anchor_text,
    find_exact_matches, find_nearest_timestamp, find_quote, offset_to_line_col, LineCol,
    MatchResult, MatchStatus,
};

pub use types::{
    Entity, EntityMention, EntitiesFile, Evidence, EvidenceEvent, Resolution, ResolutionMethod,
    Span, Status, UnresolvedReason,
};

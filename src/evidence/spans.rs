//! Span computation utilities for evidence grounding
//!
//! This module provides functions for locating quotes in transcripts,
//! computing hashes, and converting byte offsets to line/column positions.
//!
//! # Design Decisions (V1)
//!
//! - **Exact match only**: We only generate spans for exact byte matches
//! - **Honest unresolved**: If no match found, we record status=unresolved
//! - **No normalization mapping**: Normalized search is hint-only, no offset conversion
//! - **UTF-8 byte offsets**: All offsets are byte indices into raw file bytes

use sha2::{Digest, Sha256};

/// Result of searching for a quote in transcript
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// All byte offset ranges where the quote was found
    pub matches: Vec<(usize, usize)>,
    /// Whether a normalized match was found (hint for unresolved reason)
    pub normalized_hint: bool,
}

impl MatchResult {
    /// Returns the status based on match count
    pub fn status(&self) -> MatchStatus {
        match self.matches.len() {
            0 => MatchStatus::Unresolved,
            1 => MatchStatus::Resolved,
            _ => MatchStatus::Ambiguous,
        }
    }

    /// Returns the selected match (first one, deterministic)
    pub fn selected_match(&self) -> Option<(usize, usize)> {
        self.matches.first().copied()
    }

    /// Returns match_count and match_rank (1-indexed)
    pub fn match_info(&self) -> (usize, usize) {
        (self.matches.len(), 1) // Always select rank 1 (first match)
    }
}

/// Status of quote resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStatus {
    /// Exactly one match found
    Resolved,
    /// Multiple matches found, first selected
    Ambiguous,
    /// No match found
    Unresolved,
}

impl MatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchStatus::Resolved => "resolved",
            MatchStatus::Ambiguous => "ambiguous",
            MatchStatus::Unresolved => "unresolved",
        }
    }
}

/// Find all exact matches of quote bytes in transcript bytes
///
/// Returns all (start, end) byte offset pairs where the quote appears.
/// This is a simple sliding window search - O(n*m) worst case.
///
/// # Arguments
/// * `transcript` - The full transcript bytes
/// * `quote` - The quote bytes to search for
///
/// # Returns
/// * `Vec<(usize, usize)>` - All (start, end) byte offset pairs
pub fn find_exact_matches(transcript: &[u8], quote: &[u8]) -> Vec<(usize, usize)> {
    if quote.is_empty() || quote.len() > transcript.len() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let quote_len = quote.len();

    // Simple sliding window search
    for i in 0..=(transcript.len() - quote_len) {
        if &transcript[i..i + quote_len] == quote {
            matches.push((i, i + quote_len));
        }
    }

    matches
}

/// Check if a normalized version of the quote exists in transcript
///
/// This is used as a hint for unresolved_reason only.
/// Does NOT attempt offset mapping - just returns true/false.
///
/// Normalization: collapse whitespace, trim (no lowercasing in V1)
fn has_normalized_match(transcript: &str, quote: &str) -> bool {
    let normalized_transcript = normalize_whitespace(transcript);
    let normalized_quote = normalize_whitespace(quote);

    normalized_transcript.contains(&normalized_quote)
}

/// Normalize whitespace: collapse runs of whitespace to single space, trim
fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Find quote in transcript with full match result
///
/// This is the main entry point for span resolution.
///
/// # Arguments
/// * `transcript` - The full transcript as string
/// * `quote` - The quote to search for
///
/// # Returns
/// * `MatchResult` with all matches and normalized hint
pub fn find_quote(transcript: &str, quote: &str) -> MatchResult {
    let matches = find_exact_matches(transcript.as_bytes(), quote.as_bytes());

    let normalized_hint = if matches.is_empty() {
        has_normalized_match(transcript, quote)
    } else {
        false
    };

    MatchResult {
        matches,
        normalized_hint,
    }
}

/// Compute SHA256 hash of a byte slice, returning hex string with prefix
///
/// # Arguments
/// * `bytes` - The bytes to hash
///
/// # Returns
/// * String in format "sha256:abc123..."
pub fn compute_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    format!("sha256:{}", hex::encode(result))
}

/// Compute slice hash for a span
///
/// # Arguments
/// * `transcript` - The full transcript bytes
/// * `start` - Start byte offset
/// * `end` - End byte offset
///
/// # Returns
/// * SHA256 hash of the slice
pub fn compute_slice_hash(transcript: &[u8], start: usize, end: usize) -> String {
    let slice = &transcript[start..end];
    compute_hash(slice)
}

/// Extract anchor text around a span
///
/// Returns ~80 characters of context around the span.
///
/// # Arguments
/// * `transcript` - The full transcript as string
/// * `start` - Start byte offset of span
/// * `end` - End byte offset of span
/// * `window` - Total characters of context (default 80)
///
/// # Returns
/// * String with context around the span
pub fn extract_anchor_text(transcript: &str, start: usize, end: usize, window: usize) -> String {
    let bytes = transcript.as_bytes();

    // Calculate how much context to add on each side
    let span_len = end - start;
    let remaining = window.saturating_sub(span_len);
    let each_side = remaining / 2;

    // Expand backwards, respecting UTF-8 boundaries
    let mut anchor_start = start.saturating_sub(each_side);
    // Find valid UTF-8 boundary going backwards
    while anchor_start > 0 && !transcript.is_char_boundary(anchor_start) {
        anchor_start -= 1;
    }

    // Expand forwards, respecting UTF-8 boundaries
    let mut anchor_end = (end + each_side).min(bytes.len());
    // Find valid UTF-8 boundary going forwards
    while anchor_end < bytes.len() && !transcript.is_char_boundary(anchor_end) {
        anchor_end += 1;
    }

    // Extract the anchor text
    let anchor = &transcript[anchor_start..anchor_end];

    // Add ellipsis if we truncated
    let prefix = if anchor_start > 0 { "..." } else { "" };
    let suffix = if anchor_end < bytes.len() { "..." } else { "" };

    format!("{}{}{}", prefix, anchor, suffix)
}

/// Line and column position (1-indexed for editor compatibility)
#[derive(Debug, Clone, Copy)]
pub struct LineCol {
    pub line: usize,
    pub col: usize,
}

/// Convert byte offset to line/column position
///
/// Line and column are 1-indexed for editor compatibility.
///
/// # Arguments
/// * `transcript` - The full transcript as string
/// * `offset` - Byte offset to convert
///
/// # Returns
/// * `LineCol` with 1-indexed line and column
pub fn offset_to_line_col(transcript: &str, offset: usize) -> LineCol {
    let prefix = &transcript[..offset.min(transcript.len())];

    let line = prefix.matches('\n').count() + 1;

    // Find the start of the current line
    let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);

    // Column is the number of characters (not bytes) from line start
    let col = transcript[line_start..offset].chars().count() + 1;

    LineCol { line, col }
}

/// Parse video timestamp from transcript near a given offset
///
/// Looks for timestamp patterns like [HH:MM:SS] or [MM:SS] before the offset.
///
/// # Arguments
/// * `transcript` - The full transcript as string
/// * `offset` - Byte offset to search near
///
/// # Returns
/// * Optional timestamp string (e.g., "00:12:34")
pub fn find_nearest_timestamp(transcript: &str, offset: usize) -> Option<String> {
    // Look in the text before the offset
    let prefix = &transcript[..offset.min(transcript.len())];

    // Find all timestamp patterns [HH:MM:SS] or [MM:SS]
    // Regex pattern: \[(\d{1,2}:\d{2}(?::\d{2})?)\]
    let mut last_timestamp = None;

    // Simple pattern matching without regex dependency
    let mut i = 0;
    let bytes = prefix.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'[' {
            // Look for closing bracket
            if let Some(end) = bytes[i..].iter().position(|&b| b == b']') {
                let content = &prefix[i + 1..i + end];
                if is_timestamp(content) {
                    last_timestamp = Some(content.to_string());
                }
                i += end;
            }
        }
        i += 1;
    }

    last_timestamp
}

/// Check if a string looks like a timestamp (HH:MM:SS or MM:SS)
fn is_timestamp(s: &str) -> bool {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    parts.iter().all(|p| p.len() <= 2 && p.chars().all(|c| c.is_ascii_digit()))
}

/// Compute deterministic evidence ID
///
/// Two-tier strategy:
/// - Unresolved: sha256(content_id + extractor + quote_sha256)[0:16]
/// - Resolved: sha256(content_id + extractor + quote_sha256 + start + end)[0:16]
///
/// # Arguments
/// * `content_id` - The content ID
/// * `extractor` - Name of the extraction pattern
/// * `quote_sha256` - Hash of the quote
/// * `span` - Optional (start, end) if resolved
///
/// # Returns
/// * 16-character hex ID
pub fn compute_evidence_id(
    content_id: &str,
    extractor: &str,
    quote_sha256: &str,
    span: Option<(usize, usize)>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content_id.as_bytes());
    hasher.update(extractor.as_bytes());
    hasher.update(quote_sha256.as_bytes());

    if let Some((start, end)) = span {
        hasher.update(start.to_string().as_bytes());
        hasher.update(end.to_string().as_bytes());
    }

    let result = hasher.finalize();
    hex::encode(&result[..8]) // 16 hex chars = 8 bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_exact_matches_single() {
        let transcript = b"Hello world, this is a test.";
        let quote = b"this is";
        let matches = find_exact_matches(transcript, quote);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], (13, 20));
    }

    #[test]
    fn test_find_exact_matches_multiple() {
        let transcript = b"foo bar foo baz foo";
        let quote = b"foo";
        let matches = find_exact_matches(transcript, quote);
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0], (0, 3));
        assert_eq!(matches[1], (8, 11));
        assert_eq!(matches[2], (16, 19));
    }

    #[test]
    fn test_find_exact_matches_none() {
        let transcript = b"Hello world";
        let quote = b"xyz";
        let matches = find_exact_matches(transcript, quote);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_match_status() {
        let result = MatchResult {
            matches: vec![(0, 5)],
            normalized_hint: false,
        };
        assert_eq!(result.status(), MatchStatus::Resolved);

        let result = MatchResult {
            matches: vec![(0, 5), (10, 15)],
            normalized_hint: false,
        };
        assert_eq!(result.status(), MatchStatus::Ambiguous);

        let result = MatchResult {
            matches: vec![],
            normalized_hint: true,
        };
        assert_eq!(result.status(), MatchStatus::Unresolved);
    }

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash(b"hello");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    #[test]
    fn test_offset_to_line_col() {
        let transcript = "line1\nline2\nline3";

        let pos = offset_to_line_col(transcript, 0);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.col, 1);

        let pos = offset_to_line_col(transcript, 6);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.col, 1);

        let pos = offset_to_line_col(transcript, 8);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.col, 3);
    }

    #[test]
    fn test_is_timestamp() {
        assert!(is_timestamp("12:34"));
        assert!(is_timestamp("1:23:45"));
        assert!(is_timestamp("00:00:00"));
        assert!(!is_timestamp("abc"));
        assert!(!is_timestamp("12:34:56:78"));
    }

    #[test]
    fn test_find_nearest_timestamp() {
        let transcript = "[00:00] Hello [01:30] World [02:45] End";

        let ts = find_nearest_timestamp(transcript, 15);
        assert_eq!(ts, Some("00:00".to_string()));

        let ts = find_nearest_timestamp(transcript, 30);
        assert_eq!(ts, Some("01:30".to_string()));
    }

    #[test]
    fn test_evidence_id_deterministic() {
        let id1 = compute_evidence_id("abc", "extract_claims", "sha256:xyz", Some((10, 20)));
        let id2 = compute_evidence_id("abc", "extract_claims", "sha256:xyz", Some((10, 20)));
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 16);
    }

    #[test]
    fn test_evidence_id_different_for_different_spans() {
        let id1 = compute_evidence_id("abc", "extract_claims", "sha256:xyz", Some((10, 20)));
        let id2 = compute_evidence_id("abc", "extract_claims", "sha256:xyz", Some((30, 40)));
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_extract_anchor_text() {
        let transcript = "This is a long transcript with many words and content for testing.";
        let anchor = extract_anchor_text(transcript, 10, 20, 40);
        assert!(anchor.len() <= 50); // 40 + ellipsis
        assert!(anchor.contains("long transcript"));
    }

    #[test]
    fn test_normalized_hint() {
        let transcript = "Hello   world  with   extra   spaces";
        let quote = "world with extra";
        let result = find_quote(transcript, quote);
        assert!(result.matches.is_empty());
        assert!(result.normalized_hint);
    }
}

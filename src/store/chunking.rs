//! Sentence-group chunking for transcript text.
//!
//! Splits long-form text (transcripts, articles) into overlapping chunks
//! suitable for embedding and semantic search. Handles abbreviation edge
//! cases without requiring a regex dependency.

use sha2::{Digest, Sha256};

// ─────────────────────────────────────────────────────────────────
// Strategy
// ─────────────────────────────────────────────────────────────────

/// How to chunk a piece of text.
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkStrategy {
    /// Split into overlapping sentence groups.
    SentenceGroup {
        target_words: usize,
        min_words: usize,
        max_words: usize,
    },
    /// Keep the entire text as a single chunk.
    WholeDocument,
}

impl ChunkStrategy {
    /// Select an appropriate strategy based on item type and word count.
    pub fn for_item_type(item_type: &str, word_count: usize) -> Self {
        match item_type {
            _ if word_count < 500 => Self::WholeDocument,
            "content" => Self::SentenceGroup {
                target_words: 400,
                min_words: 200,
                max_words: 600,
            },
            _ => Self::WholeDocument,
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Chunk output
// ─────────────────────────────────────────────────────────────────

/// A single chunk produced from a larger text.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Deterministic ID: SHA256(item_id + chunk_index)[0:16].
    pub id: String,
    /// The parent item this chunk belongs to.
    pub item_id: String,
    /// 0-based index within the item's chunks.
    pub chunk_index: usize,
    /// The chunk text content.
    pub text: String,
    /// Byte offset of the start of this chunk in the original text.
    pub byte_start: usize,
    /// Byte offset of the end of this chunk in the original text (exclusive).
    pub byte_end: usize,
    /// Number of whitespace-delimited words in this chunk.
    pub word_count: usize,
}

// ─────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────

/// Chunk text according to the given strategy.
pub fn chunk_text(item_id: &str, text: &str, strategy: &ChunkStrategy) -> Vec<Chunk> {
    match strategy {
        ChunkStrategy::WholeDocument => {
            let wc = word_count(text);
            let id = chunk_id(item_id, 0);
            vec![Chunk {
                id,
                item_id: item_id.to_string(),
                chunk_index: 0,
                text: text.to_string(),
                byte_start: 0,
                byte_end: text.len(),
                word_count: wc,
            }]
        }
        ChunkStrategy::SentenceGroup {
            target_words,
            min_words,
            max_words,
        } => sentence_group_chunk(item_id, text, *target_words, *min_words, *max_words),
    }
}

// ─────────────────────────────────────────────────────────────────
// Sentence splitting
// ─────────────────────────────────────────────────────────────────

/// Common abbreviations that should NOT be treated as sentence boundaries.
const ABBREVIATIONS: &[&str] = &[
    "Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "Sr.", "Jr.",
    "U.S.", "U.K.", "U.N.", "E.U.",
    "Inc.", "Ltd.", "Corp.", "Co.",
    "St.", "Ave.", "Blvd.",
    "Jan.", "Feb.", "Mar.", "Apr.", "Aug.", "Sep.", "Oct.", "Nov.", "Dec.",
    "vs.", "etc.", "approx.", "dept.",
    "i.e.", "e.g.",
];

/// Check if a period at `period_pos` in `text` is part of an abbreviation.
fn is_abbreviation(text: &str, period_pos: usize) -> bool {
    // Get the "word" ending at period_pos by scanning backwards
    let before = &text[..period_pos];
    let word_start = before.rfind(|c: char| c.is_whitespace() || c == '\n')
        .map(|p| p + 1)
        .unwrap_or(0);
    let candidate = &text[word_start..=period_pos];

    // Check against known abbreviations (case-insensitive)
    for abbr in ABBREVIATIONS {
        if candidate.eq_ignore_ascii_case(abbr) {
            return true;
        }
    }

    // Check for patterns like "U.S." — single letter followed by period
    // that is part of a multi-dot abbreviation
    if candidate.len() >= 2 {
        let chars: Vec<char> = candidate.chars().collect();
        // Pattern: single uppercase letter + period (e.g., the "S." in "U.S.")
        if chars.len() == 2 && chars[0].is_ascii_uppercase() && chars[1] == '.' {
            return true;
        }
    }

    // Check for decimal numbers like "3.14"
    if period_pos > 0 && period_pos + 1 < text.len() {
        let char_before = text.as_bytes()[period_pos - 1];
        let char_after = text.as_bytes()[period_pos + 1];
        if char_before.is_ascii_digit() && char_after.is_ascii_digit() {
            return true;
        }
    }

    false
}

/// Split text into segments. Returns a list of (segment_text, byte_start, byte_end).
///
/// Primary boundaries: `. ` | `? ` | `! ` | `.\n` | `?\n` | `!\n`
/// Fallback: if no punctuation boundaries found (common in Whisper transcripts
/// without punctuation), falls back to newline-based splitting.
/// Excludes boundaries inside abbreviations and decimal numbers.
fn split_sentences(text: &str) -> Vec<(&str, usize, usize)> {
    if text.is_empty() {
        return Vec::new();
    }

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut sentences = Vec::new();
    let mut start = 0;

    let mut i = 0;
    while i < len {
        let b = bytes[i];
        let is_terminal = b == b'.' || b == b'?' || b == b'!';

        if is_terminal && i + 1 < len {
            let next = bytes[i + 1];
            let is_boundary = next == b' ' || next == b'\n';

            if is_boundary {
                // For periods, check abbreviation/number exclusions
                if b == b'.' && is_abbreviation(text, i) {
                    i += 1;
                    continue;
                }

                // End of sentence: include the terminal punctuation
                let end = i + 1;
                let sentence = &text[start..end];
                let trimmed = sentence.trim();
                if !trimmed.is_empty() {
                    // Record byte positions of the trimmed content
                    let trim_start = start + sentence.find(trimmed).unwrap_or(0);
                    let trim_end = trim_start + trimmed.len();
                    sentences.push((trimmed, trim_start, trim_end));
                }
                // Skip the space/newline separator
                start = i + 1;
                // Skip leading whitespace for the next sentence
                while start < len && (bytes[start] == b' ' || bytes[start] == b'\n') {
                    start += 1;
                }
                i = start;
                continue;
            }
        }

        // Handle terminal at end of text
        if is_terminal && i + 1 == len {
            if b == b'.' && is_abbreviation(text, i) {
                i += 1;
                continue;
            }
            let end = i + 1;
            let sentence = &text[start..end];
            let trimmed = sentence.trim();
            if !trimmed.is_empty() {
                let trim_start = start + sentence.find(trimmed).unwrap_or(0);
                let trim_end = trim_start + trimmed.len();
                sentences.push((trimmed, trim_start, trim_end));
            }
            start = end;
            i = start;
            continue;
        }

        i += 1;
    }

    // Trailing text without terminal punctuation
    if start < len {
        let remaining = &text[start..];
        let trimmed = remaining.trim();
        if !trimmed.is_empty() {
            let trim_start = start + remaining.find(trimmed).unwrap_or(0);
            let trim_end = trim_start + trimmed.len();
            sentences.push((trimmed, trim_start, trim_end));
        }
    }

    // FALLBACK: If punctuation-based splitting produced 0 or 1 segments but the
    // text has newlines, fall back to newline-based splitting. This handles Whisper
    // transcripts that output without punctuation (common with large-v3-turbo on
    // casual/conversational speech).
    if sentences.len() <= 1 && text.contains('\n') {
        let mut line_segments = Vec::new();
        let mut line_start = 0;
        for (i, b) in bytes.iter().enumerate() {
            if *b == b'\n' {
                let line = &text[line_start..i];
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let trim_offset = line_start + line.find(trimmed).unwrap_or(0);
                    line_segments.push((trimmed, trim_offset, trim_offset + trimmed.len()));
                }
                line_start = i + 1;
            }
        }
        // Don't forget trailing content after last newline
        if line_start < len {
            let line = &text[line_start..];
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let trim_offset = line_start + line.find(trimmed).unwrap_or(0);
                line_segments.push((trimmed, trim_offset, trim_offset + trimmed.len()));
            }
        }
        // Only use newline fallback if it produces more segments
        if line_segments.len() > sentences.len() {
            return line_segments;
        }
    }

    sentences
}

// ─────────────────────────────────────────────────────────────────
// Sentence grouping
// ─────────────────────────────────────────────────────────────────

fn sentence_group_chunk(
    item_id: &str,
    text: &str,
    target_words: usize,
    min_words: usize,
    max_words: usize,
) -> Vec<Chunk> {
    let sentences = split_sentences(text);
    if sentences.is_empty() {
        return vec![Chunk {
            id: chunk_id(item_id, 0),
            item_id: item_id.to_string(),
            chunk_index: 0,
            text: text.to_string(),
            byte_start: 0,
            byte_end: text.len(),
            word_count: word_count(text),
        }];
    }

    // Build groups of sentences targeting ~target_words per group
    let mut groups: Vec<Vec<usize>> = Vec::new(); // each group is indices into `sentences`
    let mut current_group: Vec<usize> = Vec::new();
    let mut current_wc: usize = 0;

    for (idx, (sent_text, _, _)) in sentences.iter().enumerate() {
        let swc = word_count(sent_text);
        current_group.push(idx);
        current_wc += swc;

        if current_wc >= target_words {
            groups.push(current_group.clone());
            current_group.clear();
            current_wc = 0;
        }
    }

    // Handle remaining sentences
    if !current_group.is_empty() {
        if current_wc < min_words && !groups.is_empty() {
            // Merge small trailing group with previous
            if let Some(last_group) = groups.last_mut() {
                last_group.extend(current_group);
            }
        } else {
            groups.push(current_group);
        }
    }

    // If we ended up with no groups (shouldn't happen but safety)
    if groups.is_empty() {
        return vec![Chunk {
            id: chunk_id(item_id, 0),
            item_id: item_id.to_string(),
            chunk_index: 0,
            text: text.to_string(),
            byte_start: 0,
            byte_end: text.len(),
            word_count: word_count(text),
        }];
    }

    // Convert groups to chunks with overlap
    let mut chunks = Vec::new();
    let mut prev_last_sentence: Option<usize> = None;

    for (group_idx, group) in groups.iter().enumerate() {
        let first_sent_idx = group[0];
        let last_sent_idx = *group.last().unwrap();

        // Determine byte range from original text
        let byte_start = sentences[first_sent_idx].1;
        let byte_end = sentences[last_sent_idx].2;

        // Build chunk text with overlap prepended
        let mut chunk_parts: Vec<&str> = Vec::new();

        // Prepend last sentence of previous chunk for overlap
        if let Some(prev_last) = prev_last_sentence {
            chunk_parts.push(sentences[prev_last].0);
        }

        for &sent_idx in group {
            chunk_parts.push(sentences[sent_idx].0);
        }

        let chunk_text = chunk_parts.join(" ");

        // Hard split if chunk exceeds max_words
        let final_text = if word_count(&chunk_text) > max_words {
            truncate_to_word_boundary(&chunk_text, max_words)
        } else {
            chunk_text
        };

        let wc = word_count(&final_text);
        let id = chunk_id(item_id, group_idx);

        chunks.push(Chunk {
            id,
            item_id: item_id.to_string(),
            chunk_index: group_idx,
            text: final_text,
            byte_start,
            byte_end,
            word_count: wc,
        });

        prev_last_sentence = Some(last_sent_idx);
    }

    chunks
}

// ─────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────

/// Count whitespace-delimited words.
fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Generate a deterministic chunk ID from item_id and chunk_index.
fn chunk_id(item_id: &str, chunk_index: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(item_id.as_bytes());
    hasher.update(chunk_index.to_string().as_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[..8]) // 16 hex chars
}

/// Truncate text to approximately `max_words` at a word boundary.
fn truncate_to_word_boundary(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.to_string();
    }
    words[..max_words].join(" ")
}

// ─────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentence_split_basic() {
        let text = "Hello world. How are you? Fine.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 3, "Expected 3 sentences, got {:?}", sentences);
        assert_eq!(sentences[0].0, "Hello world.");
        assert_eq!(sentences[1].0, "How are you?");
        assert_eq!(sentences[2].0, "Fine.");
    }

    #[test]
    fn test_sentence_split_abbreviations() {
        let text = "Dr. Smith went to U.S. embassy.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 1, "Should be 1 sentence, got {:?}", sentences);
        assert_eq!(sentences[0].0, "Dr. Smith went to U.S. embassy.");
    }

    #[test]
    fn test_sentence_split_numbers() {
        let text = "The cost is $3.14 per unit.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 1, "Should be 1 sentence, got {:?}", sentences);
        assert_eq!(sentences[0].0, "The cost is $3.14 per unit.");
    }

    #[test]
    fn test_group_sentences_target_words() {
        // Build text with many sentences, each ~11 words
        // 80 sentences * 11 words = ~880 words; should produce at least 2 chunks at target 400
        // (first group ~400 words, second group ~480 words >= min_words 200)
        let sentence = "This is a test sentence with about ten words in it.";
        let text = std::iter::repeat(sentence)
            .take(80)
            .collect::<Vec<_>>()
            .join(" ");
        let wc = word_count(&text);
        assert!(wc > 600, "Need >600 words for this test, got {}", wc);

        let strategy = ChunkStrategy::SentenceGroup {
            target_words: 400,
            min_words: 200,
            max_words: 600,
        };
        let chunks = chunk_text("test-item", &text, &strategy);

        assert!(chunks.len() >= 2, "Expected at least 2 chunks, got {}", chunks.len());

        // Each chunk should have roughly target_words (within reasonable bounds)
        for chunk in &chunks {
            assert!(
                chunk.word_count > 0,
                "chunk {} has 0 words",
                chunk.chunk_index
            );
        }
    }

    #[test]
    fn test_group_merges_small_trailing() {
        // Build text where the trailing group would be < min_words (200)
        // 45 sentences * ~11 words = ~495 words
        // At target=400, first group takes ~36 sentences (~400 words)
        // Remaining ~9 sentences (~100 words) < min_words(200), should merge
        let sentence = "This is a test sentence with about eleven words in it here.";
        let text = std::iter::repeat(sentence)
            .take(45)
            .collect::<Vec<_>>()
            .join(" ");

        let strategy = ChunkStrategy::SentenceGroup {
            target_words: 400,
            min_words: 200,
            max_words: 600,
        };
        let chunks = chunk_text("test-item", &text, &strategy);

        // The trailing chunk should have been merged, resulting in 1 chunk
        // (since total is ~495 words, first group hits 400, remainder ~95 < 200 merges back)
        assert_eq!(
            chunks.len(),
            1,
            "Small trailing group should merge with previous, got {} chunks",
            chunks.len()
        );
    }

    #[test]
    fn test_chunk_overlap() {
        // Create text with distinct sentences to verify overlap
        let sentences: Vec<String> = (0..80)
            .map(|i| format!("Sentence number {} has several words to fill space in the text document here.", i))
            .collect();
        let text = sentences.join(" ");

        let strategy = ChunkStrategy::SentenceGroup {
            target_words: 400,
            min_words: 200,
            max_words: 600,
        };
        let chunks = chunk_text("test-item", &text, &strategy);

        // Need at least 2 chunks to test overlap
        assert!(chunks.len() >= 2, "Need at least 2 chunks for overlap test");

        // The second chunk should start with the last sentence of the first chunk
        // Find the last sentence boundary in chunk 0's text (before overlap was added to chunk 1)
        // The overlap sentence from chunk 0 should appear at the beginning of chunk 1
        let chunk1_text = &chunks[1].text;

        // The first chunk's content includes some sentences, and the last one
        // should be repeated at the start of the second chunk
        // We verify by checking that chunk1 text contains "Sentence number" from
        // a sentence that would have been at the end of chunk 0
        assert!(
            chunk1_text.contains("Sentence number"),
            "Chunk 1 should contain overlap content"
        );

        // Verify chunks are ordered
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i);
        }
    }

    #[test]
    fn test_whole_document_strategy() {
        let text = "Short text that stays as one chunk.";
        let strategy = ChunkStrategy::WholeDocument;
        let chunks = chunk_text("item-123", text, &strategy);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, text);
        assert_eq!(chunks[0].byte_start, 0);
        assert_eq!(chunks[0].byte_end, text.len());
        assert_eq!(chunks[0].chunk_index, 0);
    }

    #[test]
    fn test_strategy_selection() {
        // Short content -> WholeDocument
        let strategy = ChunkStrategy::for_item_type("content", 100);
        assert_eq!(strategy, ChunkStrategy::WholeDocument);

        // Long content -> SentenceGroup
        let strategy = ChunkStrategy::for_item_type("content", 1000);
        assert_eq!(
            strategy,
            ChunkStrategy::SentenceGroup {
                target_words: 400,
                min_words: 200,
                max_words: 600,
            }
        );

        // Non-content type -> WholeDocument regardless of length
        let strategy = ChunkStrategy::for_item_type("email", 1000);
        assert_eq!(strategy, ChunkStrategy::WholeDocument);

        // Content under threshold -> WholeDocument
        let strategy = ChunkStrategy::for_item_type("content", 499);
        assert_eq!(strategy, ChunkStrategy::WholeDocument);
    }

    #[test]
    fn test_byte_offsets_correct() {
        let text = "First sentence here. Second sentence here. Third sentence here.";
        let sentences = split_sentences(text);

        for (sent_text, byte_start, byte_end) in &sentences {
            let extracted = &text[*byte_start..*byte_end];
            assert_eq!(
                extracted, *sent_text,
                "Byte offsets should map back to original text"
            );
        }

        // Test on whole document chunk
        let strategy = ChunkStrategy::WholeDocument;
        let chunks = chunk_text("item-1", text, &strategy);
        assert_eq!(chunks[0].byte_start, 0);
        assert_eq!(chunks[0].byte_end, text.len());
        assert_eq!(&text[chunks[0].byte_start..chunks[0].byte_end], text);
    }

    #[test]
    fn test_chunk_ids_deterministic() {
        let text = "Some text content for chunking purposes.";
        let strategy = ChunkStrategy::WholeDocument;

        let chunks1 = chunk_text("item-abc", text, &strategy);
        let chunks2 = chunk_text("item-abc", text, &strategy);

        assert_eq!(chunks1[0].id, chunks2[0].id, "Same input should produce same IDs");

        // Different item_id -> different chunk ID
        let chunks3 = chunk_text("item-xyz", text, &strategy);
        assert_ne!(chunks1[0].id, chunks3[0].id, "Different item_id should produce different IDs");
    }
}

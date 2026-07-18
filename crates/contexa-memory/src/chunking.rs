//! Chunk splitter for `visible_text` — `docs/07_Memory_Engine.md` §11.1.
//!
//! Token counts are estimated as `chars/4` — the same fallback docs/10
//! §12.1 sanctions for providers without a real tokenizer; Memory Engine has
//! no LLM-provider context to ask for an exact count either.

const MAX_CHUNK_TOKENS: usize = 512;
const CHUNK_OVERLAP_TOKENS: usize = 50;
const MIN_CHUNK_TOKENS: usize = 50;
const CHARS_PER_TOKEN: usize = 4;

#[must_use]
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(CHARS_PER_TOKEN).max(1)
}

/// Splits `text` into chunks of at most `MAX_CHUNK_TOKENS`, overlapping by
/// `CHUNK_OVERLAP_TOKENS`. A trailing chunk under `MIN_CHUNK_TOKENS` is
/// merged into its predecessor rather than stored on its own.
#[must_use]
pub fn chunk_text(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let max_chars = MAX_CHUNK_TOKENS * CHARS_PER_TOKEN;
    let overlap_chars = CHUNK_OVERLAP_TOKENS * CHARS_PER_TOKEN;
    let min_chars = MIN_CHUNK_TOKENS * CHARS_PER_TOKEN;

    if chars.len() <= max_chars {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    loop {
        let end = (start + max_chars).min(chars.len());
        chunks.push(chars[start..end].iter().collect::<String>());
        if end == chars.len() {
            break;
        }
        start = end.saturating_sub(overlap_chars);
    }

    if chunks.len() > 1 {
        let last_len = chunks.last().map_or(0, |c| c.chars().count());
        if last_len < min_chars {
            if let Some(last) = chunks.pop() {
                if let Some(prev) = chunks.last_mut() {
                    prev.push_str(&last);
                }
            }
        }
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_stays_a_single_chunk() {
        let text = "hello world";
        assert_eq!(chunk_text(text), vec![text.to_string()]);
    }

    #[test]
    fn long_text_splits_into_multiple_overlapping_chunks() {
        let text = "a".repeat(3000); // ~750 estimated tokens, over the 512 max
        let chunks = chunk_text(&text);
        assert!(chunks.len() > 1);
        // Reassembled length should exceed the original due to overlap.
        let total: usize = chunks.iter().map(|c| c.chars().count()).sum();
        assert!(total >= text.chars().count());
    }

    #[test]
    fn estimate_tokens_is_roughly_chars_over_four() {
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens(""), 1);
    }
}

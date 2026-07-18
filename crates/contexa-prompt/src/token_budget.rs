//! `TokenBudgetManager` — `docs/10_Prompt_Builder.md` §5, §7.3.
//!
//! Token counts are the `chars/4` fallback estimate (§12.1's own sanctioned
//! fallback for "Unknown" providers) — not a real per-provider tokenizer.
//! Getting an exact count means asking each `contexa-llm` adapter, which
//! doesn't expose one today; flagged as future work rather than guessed at.

use crate::types::SourceType;

const CHARS_PER_TOKEN: usize = 4;
// docs/10 §5's "Response Reserve" slice of the pie (1500 of an 8K budget).
const DEFAULT_RESPONSE_RESERVE: usize = 1500;

#[must_use]
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    text.chars().count().div_ceil(CHARS_PER_TOKEN)
}

#[must_use]
pub fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens * CHARS_PER_TOKEN;
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push('…');
    out
}

/// One named slice of the prompt, in priority order (highest first — see
/// §5.1's table) so callers build the `Vec` already sorted the way
/// `allocate` needs it.
pub struct Section {
    pub source_type: SourceType,
    pub label: String,
    pub content: String,
    pub max_tokens: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct AllocationResult {
    pub total_tokens: usize,
    pub truncated: bool,
}

pub struct TokenBudgetManager {
    max_tokens: usize,
    response_reserve: usize,
}

impl TokenBudgetManager {
    #[must_use]
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            response_reserve: DEFAULT_RESPONSE_RESERVE,
        }
    }

    /// Truncates each section's content in place to fit within its own
    /// `max_tokens` cap and the overall remaining budget, processing
    /// `sections` in the order given (docs/10 §7.3 — priority order,
    /// highest first, so lower-priority sections get cut first when the
    /// budget runs out).
    pub fn allocate(&self, sections: &mut [Section]) -> AllocationResult {
        let available = self.max_tokens.saturating_sub(self.response_reserve);
        let mut used = 0;
        let mut truncated = false;

        for section in sections.iter_mut() {
            let tokens = estimate_tokens(&section.content);
            if tokens > section.max_tokens {
                section.content = truncate_to_tokens(&section.content, section.max_tokens);
                truncated = true;
            }

            let tokens = estimate_tokens(&section.content);
            let remaining = available.saturating_sub(used);
            if tokens > remaining {
                section.content = truncate_to_tokens(&section.content, remaining);
                truncated = true;
            }

            used += estimate_tokens(&section.content);
        }

        AllocationResult {
            total_tokens: used,
            truncated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(label: &str, content: &str, max_tokens: usize) -> Section {
        Section {
            source_type: SourceType::Context,
            label: label.to_string(),
            content: content.to_string(),
            max_tokens,
        }
    }

    #[test]
    fn fits_within_budget_untouched() {
        let manager = TokenBudgetManager::new(8000);
        let mut sections = vec![section("system", "short system prompt", 500)];
        let result = manager.allocate(&mut sections);
        assert!(!result.truncated);
        assert_eq!(sections[0].content, "short system prompt");
    }

    #[test]
    fn truncates_a_section_over_its_own_max_tokens() {
        let manager = TokenBudgetManager::new(8000);
        let long_content = "a".repeat(10_000); // way over any per-section cap
        let mut sections = vec![section("context", &long_content, 100)];
        let result = manager.allocate(&mut sections);
        assert!(result.truncated);
        assert!(estimate_tokens(&sections[0].content) <= 101); // +1 for the "…" marker char
    }

    #[test]
    fn lower_priority_sections_are_cut_first_when_budget_is_tight() {
        // Tiny overall budget: only the first (highest-priority) section fits.
        let manager = TokenBudgetManager::new(20); // available = 20 - 1500 -> saturates to 0
        let mut sections = vec![
            section("system", &"x".repeat(400), 500),
            section("context", &"y".repeat(400), 2000),
        ];
        let result = manager.allocate(&mut sections);
        assert!(result.truncated);
        assert_eq!(sections[0].content, "…"); // available saturated to 0
        assert_eq!(sections[1].content, "…");
    }

    #[test]
    fn estimate_tokens_of_empty_string_is_zero() {
        assert_eq!(estimate_tokens(""), 0);
    }
}

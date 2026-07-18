//! Source formatters — `docs/10_Prompt_Builder.md` §7.
//!
//! These build full (untruncated) text; `TokenBudgetManager` truncates
//! centrally afterward via `Section` (see `builder.rs`) rather than each
//! formatter re-implementing its own truncation, unlike the spec's inline
//! `format(&self, x, max_tokens)` sketch.

use std::fmt::Write as _;

use contexa_core::ContextSnapshot;
use contexa_db::{ScoredChunk, TimelineEvent};
use contexa_vision::OcrResult;

use crate::types::SearchResults;
#[cfg(test)]
use crate::types::SearchResultItem;

fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars).collect();
    out.push('…');
    out
}

pub struct ContextFormatter;

impl ContextFormatter {
    /// Falls back to OCR text (docs/05's selective-OCR result) when there's
    /// no UIA-extracted `visible_text` — `docs/08` §5.1's decision rule
    /// triggers OCR precisely when UIA confidence is low.
    #[must_use]
    pub fn format(context: &ContextSnapshot, ocr: Option<&OcrResult>) -> String {
        let mut parts = vec![
            format!("Application: {}", context.process_name),
            format!("Window: {}", context.window_title),
        ];
        if let Some(url) = &context.url {
            parts.push(format!("URL: {url}"));
        }
        if let Some(path) = &context.document_path {
            parts.push(format!("Document: {path}"));
        }
        if let Some(lang) = &context.language {
            parts.push(format!("Language: {lang}"));
        }
        let header = parts.join("\n");

        let text = match (context.visible_text.as_deref(), ocr) {
            (Some(t), _) if !t.trim().is_empty() => t,
            (_, Some(ocr)) if !ocr.text.trim().is_empty() => ocr.text.as_str(),
            _ => "[No visible text]",
        };

        format!("{header}\n\n{text}")
    }
}

pub struct MemoryFormatter;

impl MemoryFormatter {
    #[must_use]
    pub fn format(chunks: &[ScoredChunk]) -> String {
        if chunks.is_empty() {
            return String::new();
        }
        let mut section = String::from("## Relevant Memory\n");
        for (i, chunk) in chunks.iter().enumerate() {
            // `contexa_db::ScoredChunk` carries `distance` (lower = better),
            // not a `score` — same reuse decision as `contexa-memory`'s
            // `SemanticSearch`. The embedding columns are declared
            // `distance_metric=cosine`, so `1.0 - distance` is cosine
            // similarity, consistent with the "relevance %" framing here.
            let relevance_pct = ((1.0 - chunk.distance).clamp(0.0, 1.0) * 100.0).round();
            let _ = writeln!(
                section,
                "{}. [{}] {} (relevance: {:.0}%)\n   {}",
                i + 1,
                chunk.timestamp.format("%H:%M"),
                chunk.application,
                relevance_pct,
                truncate_chars(&chunk.content, 200),
            );
        }
        section
    }
}

pub struct TimelineFormatter;

impl TimelineFormatter {
    #[must_use]
    pub fn format(events: &[TimelineEvent]) -> String {
        if events.is_empty() {
            return String::new();
        }
        let mut section = String::new();
        for event in events {
            let duration = event
                .duration_ms
                .map_or_else(|| "-".to_string(), |ms| format!("{}s", ms / 1000));
            let _ = writeln!(
                section,
                "- [{}] {} ({}, {duration})",
                event.timestamp.format("%H:%M"),
                event.summary,
                event.application,
            );
        }
        section
    }
}

pub struct SearchFormatter;

impl SearchFormatter {
    #[must_use]
    pub fn format(results: Option<&SearchResults>) -> String {
        let Some(results) = results else {
            return String::new();
        };
        if results.items.is_empty() {
            return String::new();
        }
        // docs/09 §12's citation format: title, snippet, then the URL so
        // the LLM can cite it back to the user.
        let mut section = String::from("## Web Search Results\n");
        for (i, item) in results.items.iter().enumerate() {
            let _ = writeln!(
                section,
                "{}. **{}**\n   {}\n   URL: {}",
                i + 1,
                item.title,
                item.snippet,
                item.url,
            );
        }
        section
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use contexa_core::CaptureMethod;
    use uuid::Uuid;

    use super::*;

    fn bare_context() -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            window_title: "main.rs — VS Code".to_string(),
            process_name: "Code.exe".to_string(),
            process_id: 1,
            hwnd: None,
            url: None,
            document_path: None,
            visible_text: None,
            selected_text: None,
            metadata: HashMap::new(),
            language: None,
            capture_method: CaptureMethod::Uia,
        }
    }

    #[test]
    fn context_formatter_falls_back_to_ocr_when_no_visible_text() {
        let context = bare_context();
        let ocr = OcrResult {
            text: "recognized text".to_string(),
            regions: vec![],
            confidence: 0.9,
            cached: false,
            duration_ms: 10,
        };
        let formatted = ContextFormatter::format(&context, Some(&ocr));
        assert!(formatted.contains("recognized text"));
    }

    #[test]
    fn context_formatter_reports_no_text_when_neither_is_present() {
        let formatted = ContextFormatter::format(&bare_context(), None);
        assert!(formatted.contains("[No visible text]"));
    }

    #[test]
    fn memory_formatter_is_empty_for_no_chunks() {
        assert_eq!(MemoryFormatter::format(&[]), "");
    }

    #[test]
    fn timeline_formatter_is_empty_for_no_events() {
        assert_eq!(TimelineFormatter::format(&[]), "");
    }

    #[test]
    fn search_formatter_is_empty_when_none() {
        assert_eq!(SearchFormatter::format(None), "");
    }

    #[test]
    fn search_formatter_lists_titles_snippets_and_urls() {
        let results = SearchResults {
            items: vec![
                SearchResultItem {
                    title: "Tokio docs".to_string(),
                    url: "https://docs.rs/tokio".to_string(),
                    snippet: "Async runtime for Rust.".to_string(),
                },
                SearchResultItem {
                    title: "Second".to_string(),
                    url: "https://example.com".to_string(),
                    snippet: "Snippet two.".to_string(),
                },
            ],
        };
        let formatted = SearchFormatter::format(Some(&results));
        assert!(formatted.contains("1. **Tokio docs**"));
        assert!(formatted.contains("URL: https://docs.rs/tokio"));
        assert!(formatted.contains("2. **Second**"));
    }
}

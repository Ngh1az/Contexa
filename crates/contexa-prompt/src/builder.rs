//! `PromptBuilder` — `docs/10_Prompt_Builder.md` §9, assembling formatted
//! sections, a token budget, and a template into an `AssembledPrompt`.

use contexa_core::{RequestAction, Result};
use contexa_llm::{Message, Role};

use crate::formatters::{ContextFormatter, MemoryFormatter, SearchFormatter, TimelineFormatter};
use crate::templates::{render, TemplateSections, SYSTEM_PROMPT};
use crate::token_budget::{estimate_tokens, Section, TokenBudgetManager};
use crate::types::{AssembledPrompt, PromptInput, SourceRef, SourceType};

pub trait PromptBuilder: Send + Sync {
    /// # Errors
    /// Returns an error if prompt assembly fails.
    fn build(&self, input: PromptInput) -> Result<AssembledPrompt>;
}

pub struct ContexaPromptBuilder {
    max_tokens: usize,
}

impl ContexaPromptBuilder {
    #[must_use]
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }
}

impl Default for ContexaPromptBuilder {
    fn default() -> Self {
        // 8K context, matching docs/10 §5's pie-chart example budget.
        Self::new(8192)
    }
}

fn date_range_label(timeline: Option<&[contexa_db::TimelineEvent]>) -> String {
    let Some(events) = timeline else {
        return "recent activity".to_string();
    };
    let start = events.iter().map(|e| e.timestamp).min();
    let end = events.iter().map(|e| e.timestamp).max();
    match (start, end) {
        (Some(start), Some(end)) => format!(
            "{} to {}",
            start.format("%Y-%m-%d"),
            end.format("%Y-%m-%d")
        ),
        _ => "recent activity".to_string(),
    }
}

/// Priority order per docs/10 §5.1 — highest first, so
/// `TokenBudgetManager` truncates lower-priority sections first.
fn build_sections(input: &PromptInput) -> Vec<Section> {
    let context_text = ContextFormatter::format(&input.context, input.ocr.as_ref());
    let memory_text = MemoryFormatter::format(&input.memory);
    let search_text = SearchFormatter::format(input.search.as_ref());
    let timeline_text = input
        .timeline
        .as_deref()
        .map(TimelineFormatter::format)
        .unwrap_or_default();
    let selected_text = input
        .context
        .selected_text
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_default()
        .to_string();
    let user_query = input.request.query.clone().unwrap_or_default();

    vec![
        Section {
            source_type: SourceType::Context,
            label: "user_query".to_string(),
            content: user_query,
            max_tokens: 500,
        },
        Section {
            source_type: SourceType::Selection,
            label: "selected_text".to_string(),
            content: selected_text,
            max_tokens: 1000,
        },
        Section {
            source_type: SourceType::Context,
            label: "context".to_string(),
            content: context_text,
            max_tokens: 2000,
        },
        Section {
            source_type: SourceType::Memory,
            label: "memory".to_string(),
            content: memory_text,
            max_tokens: 1500,
        },
        Section {
            source_type: SourceType::Search,
            label: "search".to_string(),
            content: search_text,
            max_tokens: 1000,
        },
        Section {
            source_type: SourceType::Timeline,
            label: "timeline".to_string(),
            content: timeline_text,
            max_tokens: 1000,
        },
    ]
}

impl PromptBuilder for ContexaPromptBuilder {
    fn build(&self, input: PromptInput) -> Result<AssembledPrompt> {
        let target_lang = match &input.request.action {
            RequestAction::Translate { target_lang } => target_lang.as_str(),
            _ => "",
        };
        let date_range = date_range_label(input.timeline.as_deref());

        let mut sections = build_sections(&input);
        let allocation = TokenBudgetManager::new(self.max_tokens).allocate(&mut sections);

        let get = |label: &str| -> &str {
            sections
                .iter()
                .find(|s| s.label == label)
                .map_or("", |s| s.content.as_str())
        };
        let template_sections = TemplateSections {
            context: get("context"),
            selected_text: Some(get("selected_text")).filter(|t| !t.is_empty()),
            memory_section: get("memory"),
            search_section: get("search"),
            timeline_section: get("timeline"),
            user_query: get("user_query"),
            date_range: &date_range,
            target_lang,
        };
        let body = render(&input.request.action, &template_sections);

        let sources: Vec<SourceRef> = sections
            .iter()
            .filter(|s| !s.content.is_empty())
            .map(|s| SourceRef {
                source_type: s.source_type,
                id: s.label.clone(),
                label: s.label.clone(),
                token_count: estimate_tokens(&s.content),
            })
            .collect();

        // System prompt tokens count toward the total even though they're
        // not one of the truncatable `sections` (docs/10 §5's budget
        // reserves 300-500 tokens for it up front).
        let total_tokens = allocation.total_tokens + estimate_tokens(SYSTEM_PROMPT);

        Ok(AssembledPrompt {
            system: SYSTEM_PROMPT.to_string(),
            messages: vec![
                Message {
                    role: Role::System,
                    content: SYSTEM_PROMPT.to_string(),
                },
                Message {
                    role: Role::User,
                    content: body,
                },
            ],
            token_count: total_tokens,
            sources,
            truncated: allocation.truncated,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use contexa_core::{CaptureMethod, ContextSnapshot, RequestAction, RequestPreferences, UserRequest};
    use uuid::Uuid;

    use super::*;

    fn bare_input(action: RequestAction, query: Option<&str>) -> PromptInput {
        PromptInput {
            request: UserRequest {
                id: Uuid::new_v4(),
                action,
                query: query.map(str::to_string),
                context_override: None,
                preferences: RequestPreferences::default(),
            },
            context: ContextSnapshot {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                window_title: "main.rs — VS Code".to_string(),
                process_name: "Code.exe".to_string(),
                process_id: 1,
                hwnd: None,
                url: None,
                document_path: None,
                visible_text: Some("fn main() {}".to_string()),
                selected_text: None,
                metadata: HashMap::new(),
                language: Some("rust".to_string()),
                capture_method: CaptureMethod::Uia,
            },
            memory: vec![],
            search: None,
            timeline: None,
            ocr: None,
        }
    }

    #[test]
    fn builds_a_two_message_prompt_with_system_and_user_roles() {
        let builder = ContexaPromptBuilder::default();
        let prompt = builder
            .build(bare_input(RequestAction::Explain, None))
            .expect("build should succeed");
        assert_eq!(prompt.messages.len(), 2);
        assert_eq!(prompt.messages[0].role, Role::System);
        assert_eq!(prompt.messages[1].role, Role::User);
        assert!(prompt.messages[1].content.contains("fn main() {}"));
    }

    #[test]
    fn chat_prompt_includes_the_user_query() {
        let builder = ContexaPromptBuilder::default();
        let prompt = builder
            .build(bare_input(RequestAction::Chat, Some("what does this do?")))
            .expect("build should succeed");
        assert!(prompt.messages[1].content.contains("what does this do?"));
    }

    #[test]
    fn sources_omit_empty_sections() {
        let builder = ContexaPromptBuilder::default();
        let prompt = builder
            .build(bare_input(RequestAction::Explain, None))
            .expect("build should succeed");
        // No memory/search/timeline supplied — those sections should be
        // empty and therefore absent from `sources`.
        assert!(!prompt.sources.iter().any(|s| s.label == "memory"));
        assert!(!prompt.sources.iter().any(|s| s.label == "search"));
        assert!(prompt.sources.iter().any(|s| s.label == "context"));
    }

    #[test]
    fn tiny_budget_truncates() {
        let builder = ContexaPromptBuilder::new(50);
        let mut input = bare_input(RequestAction::Explain, None);
        input.context.visible_text = Some("word ".repeat(2000));
        let prompt = builder.build(input).expect("build should succeed");
        assert!(prompt.truncated);
    }
}

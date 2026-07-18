//! `DecisionEngine` — `docs/08_AI_Orchestrator.md` §5.1's rule table.
//!
//! Two inputs the spec's pseudocode calls (`context.uia_confidence()`,
//! `self.should_search(&request.query, context)`) aren't backed by real
//! signals anywhere in the codebase yet — `ContextSnapshot` has no
//! confidence score (Vision Engine computes one internally during capture
//! but doesn't surface it), and there's no search-intent classifier.
//! `uia_confidence`/`query_implies_history`/`should_search` below are
//! documented heuristic stand-ins, not the real thing.

use contexa_core::{ContextSnapshot, ExecutionPlan, RequestAction, UserRequest};

const UIA_CONFIDENCE_THRESHOLD: f32 = 0.5;

pub struct DecisionEngine;

impl DecisionEngine {
    #[must_use]
    pub fn decide(&self, request: &UserRequest, context: &ContextSnapshot) -> ExecutionPlan {
        let mut plan = ExecutionPlan {
            need_context: true,
            ..ExecutionPlan::default()
        };

        match &request.action {
            RequestAction::Explain | RequestAction::Summarize => {
                plan.need_ocr = uia_confidence(context) < UIA_CONFIDENCE_THRESHOLD;
                plan.need_memory = matches!(request.action, RequestAction::Explain);
            }
            RequestAction::Translate { .. } => {
                plan.need_ocr = context.selected_text.is_none()
                    && uia_confidence(context) < UIA_CONFIDENCE_THRESHOLD;
            }
            RequestAction::Search => {
                plan.need_memory = true;
                plan.need_search = true;
            }
            RequestAction::Recall => {
                plan.need_timeline = true;
                plan.need_memory = true;
            }
            RequestAction::Chat => {
                plan.need_ocr = uia_confidence(context) < UIA_CONFIDENCE_THRESHOLD;
                plan.need_memory = query_implies_history(request.query.as_deref());
                plan.need_search = request.preferences.force_search
                    || should_search(request.query.as_deref());
            }
        }

        plan
    }
}

/// `1.0` if UIA text extraction produced usable content, `0.0` otherwise —
/// a coarse stand-in for a real per-window confidence score (see module doc).
fn uia_confidence(context: &ContextSnapshot) -> f32 {
    use contexa_core::CaptureMethod;
    if context.capture_method == CaptureMethod::Ocr {
        return 0.0;
    }
    match context.visible_text.as_deref() {
        Some(t) if !t.trim().is_empty() => 1.0,
        _ => 0.0,
    }
}

const HISTORY_KEYWORDS: &[&str] = &[
    "yesterday",
    "earlier",
    "before",
    "previously",
    "last time",
    "remember",
    "what did i",
];

fn query_implies_history(query: Option<&str>) -> bool {
    let Some(query) = query else { return false };
    let lower = query.to_lowercase();
    HISTORY_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

const SEARCH_KEYWORDS: &[&str] = &["search for", "look up", "latest", "current price", "news about"];

fn should_search(query: Option<&str>) -> bool {
    let Some(query) = query else { return false };
    let lower = query.to_lowercase();
    SEARCH_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use contexa_core::{CaptureMethod, RequestPreferences};
    use uuid::Uuid;

    use super::*;

    fn context(visible_text: Option<&str>, selected_text: Option<&str>) -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            window_title: "t".to_string(),
            process_name: "p".to_string(),
            process_id: 1,
            hwnd: None,
            url: None,
            document_path: None,
            visible_text: visible_text.map(str::to_string),
            selected_text: selected_text.map(str::to_string),
            metadata: HashMap::new(),
            language: None,
            capture_method: CaptureMethod::Uia,
        }
    }

    fn request(action: RequestAction, query: Option<&str>) -> UserRequest {
        UserRequest {
            id: Uuid::new_v4(),
            action,
            query: query.map(str::to_string),
            context_override: None,
            preferences: RequestPreferences::default(),
        }
    }

    #[test]
    fn explain_needs_memory_and_context_but_not_search() {
        let plan = DecisionEngine.decide(&request(RequestAction::Explain, None), &context(Some("text"), None));
        assert!(plan.need_context);
        assert!(plan.need_memory);
        assert!(!plan.need_search);
        assert!(!plan.need_ocr);
    }

    #[test]
    fn explain_needs_ocr_when_no_visible_text() {
        let plan = DecisionEngine.decide(&request(RequestAction::Explain, None), &context(None, None));
        assert!(plan.need_ocr);
    }

    #[test]
    fn summarize_does_not_need_memory() {
        let plan = DecisionEngine.decide(&request(RequestAction::Summarize, None), &context(Some("text"), None));
        assert!(!plan.need_memory);
    }

    #[test]
    fn translate_skips_ocr_when_text_is_selected() {
        let action = RequestAction::Translate { target_lang: "fr".to_string() };
        let plan = DecisionEngine.decide(&request(action, None), &context(None, Some("selected")));
        assert!(!plan.need_ocr);
    }

    #[test]
    fn translate_needs_ocr_when_nothing_selected_and_no_visible_text() {
        let action = RequestAction::Translate { target_lang: "fr".to_string() };
        let plan = DecisionEngine.decide(&request(action, None), &context(None, None));
        assert!(plan.need_ocr);
    }

    #[test]
    fn search_action_needs_memory_and_search() {
        let plan = DecisionEngine.decide(&request(RequestAction::Search, Some("rust ownership")), &context(Some("x"), None));
        assert!(plan.need_memory);
        assert!(plan.need_search);
    }

    #[test]
    fn recall_needs_timeline_and_memory_but_not_context_gated_extras() {
        let plan = DecisionEngine.decide(&request(RequestAction::Recall, None), &context(Some("x"), None));
        assert!(plan.need_timeline);
        assert!(plan.need_memory);
        assert!(!plan.need_ocr);
    }

    #[test]
    fn chat_triggers_memory_for_history_flavored_queries() {
        let plan = DecisionEngine.decide(
            &request(RequestAction::Chat, Some("what did I work on yesterday?")),
            &context(Some("x"), None),
        );
        assert!(plan.need_memory);
    }

    #[test]
    fn chat_does_not_trigger_memory_for_unrelated_queries() {
        let plan = DecisionEngine.decide(
            &request(RequestAction::Chat, Some("fix this bug")),
            &context(Some("x"), None),
        );
        assert!(!plan.need_memory);
    }

    #[test]
    fn chat_force_search_preference_overrides_keyword_heuristic() {
        let mut req = request(RequestAction::Chat, Some("fix this bug"));
        req.preferences.force_search = true;
        let plan = DecisionEngine.decide(&req, &context(Some("x"), None));
        assert!(plan.need_search);
    }
}

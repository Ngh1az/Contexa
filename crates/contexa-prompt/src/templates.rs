//! Templates — `docs/10_Prompt_Builder.md` §6.
//!
//! The system prompt is its own `Message{role: System}` (see `builder.rs`),
//! not interpolated into the body text the way §6's `{{system_prompt}}`
//! placeholder suggests — `AssembledPrompt.messages: Vec<Message>` is
//! role-based, so splitting system from user avoids sending the same text
//! twice.

use contexa_core::RequestAction;

pub const SYSTEM_PROMPT: &str = "You are Contexa, an AI assistant with access to the user's desktop context.\n\
You can see what application they are using, what text is visible, and their recent work history.\n\
Answer based on the provided context. If context is insufficient, say so clearly.\n\
Always cite sources when using search results or memory.";

pub struct TemplateSections<'a> {
    pub context: &'a str,
    pub selected_text: Option<&'a str>,
    pub memory_section: &'a str,
    pub search_section: &'a str,
    pub timeline_section: &'a str,
    pub user_query: &'a str,
    pub date_range: &'a str,
    pub target_lang: &'a str,
}

#[must_use]
pub fn render(action: &RequestAction, s: &TemplateSections) -> String {
    match action {
        RequestAction::Explain => explain(s),
        RequestAction::Summarize => summarize(s),
        RequestAction::Translate { .. } => translate(s),
        RequestAction::Recall => recall(s),
        // docs/10 §6 has no dedicated "Search" template; `Search` and `Chat`
        // both mean "answer the query using whatever context/memory/search
        // is available" (docs/08 §5.1's decision table treats them the same
        // way), so `Search` reuses the chat template.
        RequestAction::Chat | RequestAction::Search => chat(s),
    }
}

fn explain(s: &TemplateSections) -> String {
    let selected = s
        .selected_text
        .map(|t| format!("## Selected Text\n{t}\n\n"))
        .unwrap_or_default();
    format!(
        "## Current Context\n{}\n\n{selected}{}\n{}\n\n\
## Task\n\
Explain the content the user is currently viewing. Be concise and specific.\n\
Focus on the selected text if available, otherwise explain the visible content.",
        s.context, s.memory_section, s.search_section
    )
}

fn summarize(s: &TemplateSections) -> String {
    let content = s.selected_text.unwrap_or(s.context);
    format!(
        "## Content to Summarize\n{content}\n\n\
## Task\n\
Provide a concise summary of the above content. Use bullet points for key takeaways."
    )
}

fn translate(s: &TemplateSections) -> String {
    let text = s.selected_text.unwrap_or("[No text selected]");
    format!(
        "## Text to Translate\n{text}\n\n\
## Task\n\
Translate the above text to {}. Preserve formatting and technical terms.\n\
Provide only the translation, no explanation.",
        s.target_lang
    )
}

fn recall(s: &TemplateSections) -> String {
    format!(
        "## Timeline ({})\n{}\n\n{}\n\n## Task\n{}\n\
Summarize the user's activity based on the timeline and memory above.",
        s.date_range, s.timeline_section, s.memory_section, s.user_query
    )
}

fn chat(s: &TemplateSections) -> String {
    format!(
        "## Current Context\n{}\n\n{}\n{}\n\n## User Message\n{}",
        s.context, s.memory_section, s.search_section, s.user_query
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sections<'a>() -> TemplateSections<'a> {
        TemplateSections {
            context: "Application: Code.exe",
            selected_text: None,
            memory_section: "",
            search_section: "",
            timeline_section: "",
            user_query: "what is this?",
            date_range: "today",
            target_lang: "",
        }
    }

    #[test]
    fn explain_focuses_on_selected_text_when_present() {
        let mut s = sections();
        s.selected_text = Some("fn main() {}");
        let rendered = explain(&s);
        assert!(rendered.contains("## Selected Text"));
        assert!(rendered.contains("fn main() {}"));
    }

    #[test]
    fn translate_uses_target_lang() {
        let mut s = sections();
        s.selected_text = Some("hello");
        s.target_lang = "Vietnamese";
        let rendered = translate(&s);
        assert!(rendered.contains("Translate the above text to Vietnamese"));
    }

    #[test]
    fn translate_with_no_selection_says_so() {
        let rendered = translate(&sections());
        assert!(rendered.contains("[No text selected]"));
    }

    #[test]
    fn chat_includes_user_message() {
        let rendered = chat(&sections());
        assert!(rendered.contains("what is this?"));
    }

    #[test]
    fn search_action_renders_the_chat_template() {
        let chat_rendered = render(&RequestAction::Chat, &sections());
        let search_rendered = render(&RequestAction::Search, &sections());
        assert_eq!(chat_rendered, search_rendered);
    }
}

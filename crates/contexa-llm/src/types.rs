//! Shared LLM message/completion types — `docs/10_Prompt_Builder.md` §10,
//! `docs/08_AI_Orchestrator.md` §7.

use contexa_core::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct CompletionOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            max_tokens: None,
            temperature: None,
            stream: true,
        }
    }
}

/// docs/02, docs/03 §7.5, and docs/08 all reference `ResponseStream` as a
/// return type but never define it concretely (verified: grepped every doc
/// for a struct/type-alias definition, found none). An unbounded mpsc
/// receiver of token chunks is simplest — no `futures`/`tokio-stream`
/// dependency needed, and callers (Tauri commands) just loop `.recv().await`
/// and emit each chunk as an event.
pub type ResponseStream = tokio::sync::mpsc::UnboundedReceiver<Result<String>>;

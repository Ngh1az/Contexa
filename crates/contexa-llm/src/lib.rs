//! LLM Adapters — provider abstraction, streaming completions, credential
//! storage — see `docs/08_AI_Orchestrator.md` §7, `docs/02_System_Architecture.md` §7.

mod anthropic;
mod credential_vault;
mod gemini;
mod lm_studio;
mod ollama;
mod openai;
mod provider;
mod selector;
mod types;

pub use anthropic::AnthropicProvider;
pub use credential_vault::CredentialVault;
pub use gemini::GeminiProvider;
pub use lm_studio::LmStudioProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use provider::LlmProvider;
pub use selector::ProviderSelector;
pub use types::{CompletionOptions, Message, ResponseStream, Role};

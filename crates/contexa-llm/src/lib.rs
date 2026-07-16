//! LLM Adapters — provider abstraction, streaming completions, credential
//! storage — see `docs/08_AI_Orchestrator.md` §7, `docs/02_System_Architecture.md` §7.

mod credential_vault;
mod ollama;
mod provider;
mod selector;
mod types;

pub use credential_vault::CredentialVault;
pub use ollama::OllamaProvider;
pub use provider::LlmProvider;
pub use selector::ProviderSelector;
pub use types::{CompletionOptions, Message, ResponseStream, Role};

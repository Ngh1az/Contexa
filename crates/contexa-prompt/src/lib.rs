//! Prompt Builder — templates, formatters, token budgeting — see `docs/10_Prompt_Builder.md`

mod builder;
mod formatters;
mod templates;
mod token_budget;
mod types;

pub use builder::{ContexaPromptBuilder, PromptBuilder};
pub use formatters::{ContextFormatter, MemoryFormatter, SearchFormatter, TimelineFormatter};
pub use token_budget::{Section, TokenBudgetManager};
pub use types::{AssembledPrompt, PromptInput, SearchResultItem, SearchResults, SourceRef, SourceType};

//! Shared error type — see `docs/19_Coding_Standards.md` §4.5

#[derive(Debug, thiserror::Error)]
pub enum ContexaError {
    #[error("Capture failed: {reason}")]
    CaptureFailed { reason: String },

    #[error("Context unavailable")]
    ContextUnavailable,

    #[error("LLM provider error ({provider}): {message}")]
    LlmProviderError { provider: String, message: String },

    #[error("Database error: {source}")]
    Database {
        #[from]
        source: rusqlite::Error,
    },

    #[error("Migration error: {source}")]
    Migration {
        #[from]
        source: refinery::Error,
    },

    #[error("Serialization error: {source}")]
    Serialization {
        #[from]
        source: serde_json::Error,
    },

    #[error("Conversion error: {0}")]
    Conversion(String),

    #[error("Background task failed: {0}")]
    TaskJoin(String),
}

pub type Result<T> = std::result::Result<T, ContexaError>;

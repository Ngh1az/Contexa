//! `AuditLogger` — `docs/11_MCP_Runtime.md` §5.3/§12: every tool invocation
//! logged with token id, tool name, and a request summary.

use std::sync::Arc;

use contexa_core::Result;
use contexa_db::McpRepository;

pub struct AuditLogger {
    repo: Arc<dyn McpRepository>,
}

impl AuditLogger {
    #[must_use]
    pub fn new(repo: Arc<dyn McpRepository>) -> Self {
        Self { repo }
    }

    /// # Errors
    /// Returns an error if the audit write fails.
    pub async fn log(&self, token_id: &str, tool_name: &str, request_summary: &str) -> Result<()> {
        self.repo.log_tool_call(token_id, tool_name, request_summary).await
    }
}

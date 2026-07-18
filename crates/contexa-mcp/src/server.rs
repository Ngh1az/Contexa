//! `ContexaMcpServer` — `docs/11_MCP_Runtime.md` §5, ADR-0004's 5 exposed
//! tools, over stdio (`rmcp`, same macro pattern validated in
//! `spikes/SP-06-mcp-cursor/src/main.rs`).
//!
//! Reads via `contexa-db` repositories directly, not the live in-process
//! `ContextEngine`/`MemoryEngine` — this server runs as a **separate OS
//! process** spawned by the MCP client (Cursor/Claude Desktop), so it can't
//! reach into the desktop app's in-memory state. See the module doc on
//! `src/bin/contexa_mcp_server.rs` for the full reasoning.

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};

use contexa_core::ContexaError;
use contexa_db::{ContextRepository, MemoryRepository, Pagination, TimeRange, TimelineRepository};
use contexa_memory::Embedder;

use crate::audit::AuditLogger;

// docs/09 §9's default recall window when the caller omits `minutes`.
const DEFAULT_RECENT_MINUTES: u32 = 30;
const DEFAULT_TIMELINE_LIMIT: u32 = 50;
const DEFAULT_SEARCH_LIMIT: usize = 10;
const DEFAULT_MAX_VISIBLE_TEXT: usize = 10_000;
// Wide open — no `min_score` argument in docs/11 §5.2's `search_context`
// input schema, and the cosine-distance range is [0, 2] (see contexa-memory's
// `semantic_search.rs`), so 2.0 genuinely means "no filtering."
const SEARCH_MAX_DISTANCE: f32 = 2.0;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetVisibleTextRequest {
    #[schemars(description = "max characters to return, default 10000")]
    pub max_length: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetRecentContextRequest {
    #[schemars(description = "minutes to look back, default 30, max 1440")]
    pub minutes: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetTimelineRequest {
    #[schemars(description = "RFC3339 start timestamp")]
    pub start: String,
    #[schemars(description = "RFC3339 end timestamp")]
    pub end: String,
    #[schemars(description = "max events to return, default 50")]
    pub limit: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchContextRequest {
    #[schemars(description = "semantic search query")]
    pub query: String,
    #[schemars(description = "max results to return, default 10")]
    pub limit: Option<u32>,
}

#[derive(Clone)]
pub struct ContexaMcpServer {
    context_repo: Arc<dyn ContextRepository>,
    memory_repo: Arc<dyn MemoryRepository>,
    timeline_repo: Arc<dyn TimelineRepository>,
    embedder: Embedder,
    audit: Arc<AuditLogger>,
    // Validated once at process startup (see the binary's module doc for
    // why per-call bearer tokens don't map onto stdio JSON-RPC) — every
    // audit log entry for this process's lifetime uses this id.
    token_id: String,
    // Read by the `#[tool_handler]`-generated `ServerHandler` impl below —
    // rustc's dead-code pass doesn't see through that macro expansion.
    #[allow(dead_code)]
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

#[tool_router]
impl ContexaMcpServer {
    #[must_use]
    pub fn new(
        context_repo: Arc<dyn ContextRepository>,
        memory_repo: Arc<dyn MemoryRepository>,
        timeline_repo: Arc<dyn TimelineRepository>,
        embedder: Embedder,
        audit: Arc<AuditLogger>,
        token_id: String,
    ) -> Self {
        Self {
            context_repo,
            memory_repo,
            timeline_repo,
            embedder,
            audit,
            token_id,
            tool_router: Self::tool_router(),
        }
    }

    async fn log(&self, tool_name: &str, summary: &str) {
        if let Err(e) = self.audit.log(&self.token_id, tool_name, summary).await {
            tracing::warn!(error = %e, tool_name, "audit log write failed");
        }
    }

    #[tool(description = "Returns the current desktop context snapshot")]
    async fn get_current_context(&self) -> Result<String, McpError> {
        self.log("get_current_context", "{}").await;
        let snapshots = self
            .context_repo
            .get_recent(DEFAULT_RECENT_MINUTES)
            .await
            .map_err(db_err)?;
        let latest = snapshots.first();
        serde_json::to_string(&latest).map_err(json_err)
    }

    #[tool(description = "Returns visible text from the active window")]
    async fn get_visible_text(
        &self,
        Parameters(GetVisibleTextRequest { max_length }): Parameters<GetVisibleTextRequest>,
    ) -> Result<String, McpError> {
        let max_length = max_length.map_or(DEFAULT_MAX_VISIBLE_TEXT, |v| v as usize);
        self.log("get_visible_text", &format!(r#"{{"max_length":{max_length}}}"#))
            .await;

        let snapshots = self
            .context_repo
            .get_recent(DEFAULT_RECENT_MINUTES)
            .await
            .map_err(db_err)?;
        let Some(latest) = snapshots.first() else {
            return serde_json::to_string(&serde_json::json!({
                "text": "", "capture_method": null, "truncated": false,
            }))
            .map_err(json_err);
        };
        let text = latest.visible_text.clone().unwrap_or_default();
        let truncated = text.chars().count() > max_length;
        let truncated_text: String = text.chars().take(max_length).collect();
        serde_json::to_string(&serde_json::json!({
            "text": truncated_text,
            "capture_method": latest.capture_method.as_str(),
            "truncated": truncated,
        }))
        .map_err(json_err)
    }

    #[tool(description = "Returns context snapshots from the last N minutes")]
    async fn get_recent_context(
        &self,
        Parameters(GetRecentContextRequest { minutes }): Parameters<GetRecentContextRequest>,
    ) -> Result<String, McpError> {
        let minutes = minutes.unwrap_or(DEFAULT_RECENT_MINUTES);
        self.log("get_recent_context", &format!(r#"{{"minutes":{minutes}}}"#))
            .await;

        let snapshots = self.context_repo.get_recent(minutes).await.map_err(db_err)?;
        serde_json::to_string(&serde_json::json!({
            "snapshots": snapshots,
            "count": snapshots.len(),
        }))
        .map_err(json_err)
    }

    #[tool(description = "Returns chronological timeline events in a time range")]
    async fn get_timeline(
        &self,
        Parameters(GetTimelineRequest { start, end, limit }): Parameters<GetTimelineRequest>,
    ) -> Result<String, McpError> {
        let limit = limit.unwrap_or(DEFAULT_TIMELINE_LIMIT);
        self.log(
            "get_timeline",
            &format!(r#"{{"start":"{start}","end":"{end}","limit":{limit}}}"#),
        )
        .await;

        let start = parse_rfc3339(&start)?;
        let end = parse_rfc3339(&end)?;
        let page = self
            .timeline_repo
            .get_range(
                TimeRange { start, end },
                Pagination { limit, offset: 0 },
            )
            .await
            .map_err(db_err)?;
        serde_json::to_string(&page.items).map_err(json_err)
    }

    #[tool(description = "Semantic search over stored memory")]
    async fn search_context(
        &self,
        Parameters(SearchContextRequest { query, limit }): Parameters<SearchContextRequest>,
    ) -> Result<String, McpError> {
        let limit = limit.map_or(DEFAULT_SEARCH_LIMIT, |l| l as usize);
        self.log(
            "search_context",
            &format!(r#"{{"query":{query:?},"limit":{limit}}}"#),
        )
        .await;

        let vector = self.embedder.embed_one(&query).await.map_err(db_err)?;
        let results = self
            .memory_repo
            .search_similar(&vector, limit, SEARCH_MAX_DISTANCE)
            .await
            .map_err(db_err)?;
        serde_json::to_string(&results).map_err(json_err)
    }
}

#[tool_handler]
impl ServerHandler for ContexaMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Contexa AI Context Platform — real-time desktop context and memory for AI \
             (docs/11_MCP_Runtime.md).",
        )
    }
}

fn parse_rfc3339(s: &str) -> Result<chrono::DateTime<chrono::Utc>, McpError> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| McpError::invalid_params(format!("invalid RFC3339 timestamp {s:?}: {e}"), None))
}

// Cheap to take by value (map_err's closure arrives owned regardless) —
// same rationale as contexa-llm's llm_err / contexa-search's ddg_err.
#[allow(clippy::needless_pass_by_value)]
fn db_err(e: ContexaError) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

#[allow(clippy::needless_pass_by_value)]
fn json_err(e: serde_json::Error) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

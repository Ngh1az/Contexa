//! MCP Runtime — MCP server — see `docs/11_MCP_Runtime.md`
//!
//! MCP Client (docs/11 §6) isn't built: `DecisionEngine::decide()` (in
//! `contexa-orchestrator`) never sets `need_mcp`, so nothing calls it yet.

mod audit;
mod auth;
mod server;

pub use audit::AuditLogger;
pub use auth::AuthMiddleware;
pub use server::ContexaMcpServer;

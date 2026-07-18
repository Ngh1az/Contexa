//! MCP server binary — spawned as its own process by MCP clients
//! (Cursor/Claude Desktop configure `command: contexa_mcp_server.exe`),
//! serving over stdio. See `contexa_mcp::server`'s module doc for why this
//! is a separate process rather than running in-process with the Tauri app.
//!
//! Auth: real per-call bearer tokens don't map onto stdio JSON-RPC tool
//! calls the way `docs/11` §7's pseudocode assumes (MCP's `tools/call`
//! request has no generic auth field, and Cursor/Claude Desktop don't know
//! to supply one) — the standard pattern for stdio-transport MCP servers
//! needing auth is validating once at spawn time via an env var the client
//! config sets (`env: { "CONTEXA_MCP_TOKEN": "..." }` in `mcp.json`), which
//! is what this binary does.
//!
//! Settings UI (Phase 3, not built yet) is how `docs/11` §7.2 expects users
//! to generate a token. Until then: `contexa_mcp_server.exe --generate-token
//! <label>` is a stopgap so the auth loop isn't completely closed with no
//! way in.

use std::sync::Arc;

use rmcp::transport::stdio;
use rmcp::ServiceExt;

use contexa_db::{
    Database, SqliteContextRepository, SqliteMcpRepository, SqliteMemoryRepository,
    SqliteTimelineRepository,
};
use contexa_mcp::{AuditLogger, AuthMiddleware, ContexaMcpServer};
use contexa_memory::Embedder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let db_path = contexa_db::default_path();
    if let Some(dir) = db_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let db = Arc::new(Database::open(&db_path, None)?);
    let mcp_repo: Arc<dyn contexa_db::McpRepository> = Arc::new(SqliteMcpRepository(Arc::clone(&db)));
    let auth = AuthMiddleware::new(Arc::clone(&mcp_repo));

    let args: Vec<String> = std::env::args().collect();
    if let Some(label) = args
        .iter()
        .position(|a| a == "--generate-token")
        .and_then(|i| args.get(i + 1))
    {
        let raw = auth.generate_token(label).await?;
        println!("Generated token for {label:?}: {raw}");
        println!("Set this as CONTEXA_MCP_TOKEN in your MCP client's config — shown once.");
        return Ok(());
    }

    let token = std::env::var("CONTEXA_MCP_TOKEN")
        .map_err(|_| anyhow::anyhow!("CONTEXA_MCP_TOKEN env var not set — see this binary's module doc"))?;
    let token_id = auth.validate(&token).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    let context_repo = Arc::new(SqliteContextRepository(Arc::clone(&db)));
    let memory_repo = Arc::new(SqliteMemoryRepository(Arc::clone(&db)));
    let timeline_repo = Arc::new(SqliteTimelineRepository(Arc::clone(&db)));
    let embedder = Embedder::new()?;
    let audit = Arc::new(AuditLogger::new(mcp_repo));

    tracing::info!("Starting Contexa MCP server (stdio)");

    let server = ContexaMcpServer::new(
        context_repo,
        memory_repo,
        timeline_repo,
        embedder,
        audit,
        token_id,
    );

    // `Result::inspect_err` needs Rust 1.76 (workspace MSRV is 1.75) — plain
    // match instead.
    let service = match server.serve(stdio()).await {
        Ok(service) => service,
        Err(e) => {
            tracing::error!("serving error: {e:?}");
            return Err(e.into());
        }
    };

    service.waiting().await?;
    Ok(())
}

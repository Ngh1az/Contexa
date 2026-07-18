//! Spawns the real compiled `contexa_mcp_server` binary as a child process
//! and calls a tool over the real MCP stdio wire protocol — mirrors
//! `spikes/SP-06-mcp-cursor/src/client.rs`'s `TokioChildProcess` pattern,
//! now against real data in a real (temp) DB rather than a stub.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use rmcp::model::CallToolRequestParams;
use rmcp::service::ServiceExt;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use tokio::process::Command;
use uuid::Uuid;

use contexa_core::{CaptureMethod, ContextSnapshot};
use contexa_db::{ContextRepository, Database, McpRepository, SqliteContextRepository, SqliteMcpRepository};

#[tokio::test]
async fn spawned_server_returns_the_real_current_context() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Mirrors the binary's own `%APPDATA%\Contexa\contexa.db` layout under a
    // temp root passed via the child's APPDATA env var.
    let contexa_dir = dir.path().join("Contexa");
    std::fs::create_dir_all(&contexa_dir).expect("create dir");
    let db_path = contexa_dir.join("contexa.db");

    let db = Arc::new(Database::open(&db_path, None).expect("open database"));
    let context_repo = SqliteContextRepository(Arc::clone(&db));
    let mcp_repo = SqliteMcpRepository(Arc::clone(&db));

    let snapshot = ContextSnapshot {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        window_title: "integration test window".to_string(),
        process_name: "mcp_integration_test.exe".to_string(),
        process_id: 4242,
        hwnd: None,
        url: None,
        document_path: None,
        visible_text: Some("real content for the MCP integration test".to_string()),
        selected_text: None,
        metadata: HashMap::new(),
        language: None,
        capture_method: CaptureMethod::Uia,
    };
    context_repo.insert_snapshot(&snapshot).await.expect("insert_snapshot");

    let raw_token = "ctx_test_token_for_integration";
    let hash = bcrypt::hash(raw_token, bcrypt::DEFAULT_COST).expect("hash token");
    mcp_repo.create_token("integration-test", &hash).await.expect("create_token");
    drop(db); // release before the child process opens its own connections

    let server_bin = env!("CARGO_BIN_EXE_contexa_mcp_server");
    let appdata = dir.path().to_string_lossy().to_string();
    let raw_token_owned = raw_token.to_string();

    let client = ()
        .serve(
            TokioChildProcess::new(Command::new(server_bin).configure(move |cmd| {
                cmd.env("APPDATA", &appdata);
                cmd.env("CONTEXA_MCP_TOKEN", &raw_token_owned);
            }))
            .expect("spawn contexa_mcp_server"),
        )
        .await
        .expect("connect to spawned server");

    let tools = client.list_tools(Option::default()).await.expect("list_tools");
    let tool_names: Vec<_> = tools.tools.iter().map(|t| t.name.clone()).collect();
    assert!(tool_names.iter().any(|n| n == "get_current_context"));
    assert!(tool_names.iter().any(|n| n == "search_context"));

    let result = client
        .call_tool(CallToolRequestParams::new("get_current_context"))
        .await
        .expect("call_tool get_current_context");

    let content = result.content.first().expect("tool call should return content");
    let text = format!("{content:?}");
    assert!(
        text.contains("integration test window"),
        "expected the real snapshot's window_title in the tool result, got: {text}"
    );

    client.cancel().await.expect("cancel client");
}

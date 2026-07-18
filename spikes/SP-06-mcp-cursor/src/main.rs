use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetCurrentContextRequest {
    #[schemars(description = "optional max characters to return")]
    pub max_chars: Option<u32>,
}

/// Stub context server for SP-06: validates the MCP wire protocol + tool-call
/// path (docs/22 SP-06), not the real vision/context pipeline (crates/contexa-context).
#[derive(Debug, Clone)]
pub struct ContexaSpikeServer {
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

#[tool_router]
impl ContexaSpikeServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get the current desktop context (stub for SP-06 spike)")]
    fn get_current_context(
        &self,
        Parameters(GetCurrentContextRequest { max_chars }): Parameters<GetCurrentContextRequest>,
    ) -> Result<String, McpError> {
        let full = serde_json::json!({
            "app": "Notepad.exe",
            "window_title": "SP-06 spike ground truth",
            "visible_text": "This is stub context text returned by the SP-06 MCP spike server.",
            "timestamp": "2026-07-18T00:00:00Z",
        });
        let mut text = full.to_string();
        if let Some(max) = max_chars {
            text.truncate(max as usize);
        }
        Ok(text)
    }
}

#[tool_handler]
impl ServerHandler for ContexaSpikeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "SP-06 spike server exposing a stub get_current_context tool.".into(),
            ),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting SP-06 MCP spike server (stdio)");

    let service = ContexaSpikeServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("serving error: {:?}", e))?;

    service.waiting().await?;
    Ok(())
}

use rmcp::{
    RmcpError,
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use std::time::Instant;
use tokio::process::Command;

fn percentile(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[allow(clippy::result_large_err)]
#[tokio::main]
async fn main() -> Result<(), RmcpError> {
    let server_bin = std::env::var("SP06_SERVER_BIN")
        .unwrap_or_else(|_| "target/release/sp06_mcp_cursor.exe".to_string());

    // Spawn the server as a child process over stdio — mirrors how Cursor/Claude
    // Desktop launch a configured MCP server (docs/22 SP-06 method step 2).
    let client = ()
        .serve(
            TokioChildProcess::new(Command::new(&server_bin).configure(|_cmd| {}))
                .map_err(RmcpError::transport_creation::<TokioChildProcess>)?,
        )
        .await?;

    let server_info = client.peer_info();
    println!("Connected. Server info: {server_info:#?}");

    let tools = client.list_tools(Default::default()).await?;
    let tool_names: Vec<_> = tools.tools.iter().map(|t| t.name.clone()).collect();
    println!("Tools recognized: {tool_names:?}");
    assert!(
        tool_names.iter().any(|n| n == "get_current_context"),
        "get_current_context tool not found"
    );

    let iterations = 50usize;
    let mut last_result_json: Option<String> = None;
    let mut lat_us: Vec<u128> = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let t0 = Instant::now();
        let result = client
            .call_tool(CallToolRequestParam {
                name: "get_current_context".into(),
                arguments: serde_json::json!({ "max_chars": 500 })
                    .as_object()
                    .cloned(),
            })
            .await?;
        lat_us.push(t0.elapsed().as_micros());
        if let Some(content) = result.content.first() {
            last_result_json = Some(format!("{content:?}"));
        }
    }

    lat_us.sort_unstable();
    let p50 = percentile(&lat_us, 0.50);
    let p95 = percentile(&lat_us, 0.95);
    let p99 = percentile(&lat_us, 0.99);
    println!(
        "Tool call latency (in-process round-trip, {iterations} calls): p50={:.2}ms p95={:.2}ms p99={:.2}ms",
        p50 as f64 / 1000.0,
        p95 as f64 / 1000.0,
        p99 as f64 / 1000.0
    );
    println!("Sample result: {:?}", last_result_json);

    client.cancel().await?;
    Ok(())
}

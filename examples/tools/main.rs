//! tools — list the MCP tools this key can reach.
//!
//! Operation: GET /v1/tools (cloud_get_v1_tools).
//!
//! This is the served tool list: every tool the caller's org can see, which is
//! what an MCP tools/list answers. To CALL one, the next route is
//! POST /v1/tools/call (cloud_post_v1_tools_call).
//!
//! ```bash
//! HANZO_API_KEY=sk-... cargo run -p hanzo-examples --example tools
//! ```

use hanzo_client::apis::tools_api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = hanzo_examples::config(None);

    let list = tools_api::cloud_get_v1_tools(&cfg, None, None).await?;

    let tools = list.tools.unwrap_or_default();
    if tools.is_empty() {
        return Err("tools: the list is empty".into());
    }
    println!("{} tools", tools.len());
    for tool in tools.iter().take(3) {
        println!(
            "  {:<32} {}",
            tool.name.clone().unwrap_or_default(),
            tool.description.clone().unwrap_or_default()
        );
    }
    Ok(())
}

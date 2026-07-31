//! tools — list the MCP tools this key can call.
//!
//! The Hanzo MCP surface is JSON-RPC 2.0 (HIP-0300) over a single endpoint.
//! `tools/list` is the discovery call; every connector action shows up as a
//! tool named <connector>_<action>.
//!
//! Operation: POST /v1/automations/mcp (automations_mcp), method "tools/list".
//!
//! ```bash
//! HANZO_API_KEY=sk-... cargo run -p hanzo-examples --example tools
//! ```

use hanzo_client::apis::mcp_api;
use hanzo_client::models::{automations_mcp_request::Method, AutomationsMcpRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = hanzo_examples::config(None);

    let mut req = AutomationsMcpRequest::new("2.0".into(), Method::ToolsSlashList);
    req.id = Some(Some(serde_json::json!(1)));

    let resp = mcp_api::automations_mcp(&cfg, req).await?;

    // JSON-RPC reports call errors in the body, with HTTP 200.
    if let Some(err) = resp.error {
        return Err(format!("tools/list: {}", err.message.unwrap_or_default()).into());
    }
    println!("{}", serde_json::to_string_pretty(&resp.result)?);
    Ok(())
}

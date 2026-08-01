//! tools — which MCP servers this key can reach.
//!
//! Operation: GET /v1/mcp/servers (get_v1_mcp_servers).
//!
//! These are the external MCP servers the caller's org has enabled — the half
//! of the tool surface that is per-org configuration rather than a property of
//! the binary.
//!
//! THE JSON-RPC DOOR IS NOT IN THE DOCUMENT. `POST /v1/mcp` is what an MCP
//! client actually speaks and it answers `tools/list` with 833 tools, but
//! hanzoai/cloud's openapi.yaml declares only /v1/mcp/servers and
//! /v1/mcp/servers/{id}. A generated client cannot carry a method for an
//! operation the document does not have, and hand-rolling the call would make
//! this SDK stop being a projection — the one property it exists to have.
//!
//! WHEN THE DOOR IS DECLARED, MOVE THIS FLOW ONTO IT. The test is one line:
//! does `paths['/v1/mcp']` exist in the document?
//!
//! ```bash
//! HANZO_API_KEY=sk-... cargo run -p hanzo-examples --example tools
//! ```

use hanzo_client::apis::mcp_api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = hanzo_examples::config(None);

    let list = mcp_api::get_v1_mcp_servers(&cfg).await?;

    let servers = list.servers.unwrap_or_default();
    println!("{} MCP server(s) enabled for this org", servers.len());
    for s in servers.iter().take(5) {
        println!(
            "  {}  {}",
            s.name.clone().unwrap_or_else(|| "(unnamed)".into()),
            s.url
                .clone()
                .or_else(|| s.source.clone())
                .unwrap_or_else(|| "(no endpoint)".into())
        );
    }
    if servers.len() > 5 {
        println!("  … and {} more", servers.len() - 5);
    }
    Ok(())
}

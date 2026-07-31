//! money — what do I have, and what have I spent?
//!
//! Operations: GET /v1/billing/balance (cloud_get_v1_billing_balance) and
//! GET /v1/billing/usage (cloud_get_v1_billing_usage).
//!
//! Both declare only a `default` response with no content schema in hanzo.yaml,
//! so the generated functions return `Result<(), _>` and drop the body. The
//! calls below prove the route and the credential; `get_json` then reads what
//! the API actually sent, using the same configuration. When those two
//! operations regain response schemas, the generated call returns the value and
//! `get_json` goes away.
//!
//! ```bash
//! HANZO_API_KEY=sk-... cargo run -p hanzo-examples --example money
//! ```

use hanzo_client::apis::billing_api;
use hanzo_examples::{config, get_json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config(None);

    billing_api::cloud_get_v1_billing_balance(&cfg).await?;
    let balance = get_json(&cfg, "/v1/billing/balance").await?;
    println!("balance  {balance}");

    billing_api::cloud_get_v1_billing_usage(&cfg).await?;
    let usage = get_json(&cfg, "/v1/billing/usage").await?;
    println!("usage    {usage}");
    Ok(())
}

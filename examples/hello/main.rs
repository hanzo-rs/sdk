//! hello — who am I?
//!
//! The smallest complete round trip: authenticate with HANZO_API_KEY and read
//! back the identity the API resolved it to.
//!
//! Operation: GET /v1/iam/oauth/userinfo (get_v1_iam_oauth_userinfo)
//!
//! The route has to FAIL on a bad key — that is the whole point of the flow —
//! and this one does it the way OIDC requires: 401
//! `{"error":"invalid_token"}` for a bogus bearer, verified against
//! api.hanzo.ai. Two nearby routes look right and are NOT, because both answer
//! 200 and a generated client resolves the call:
//!
//!   /v1/iam/whoami  200 `{"status":"error","msg":"please sign in first"}`
//!   /v1/ai/account  200 `type="anonymous-user"`, with no Authorization at all
//!
//! It used to call GET /v1/bot/auth/me, which is behind the bot relay, is
//! absent from the document this client is generated from, and 404s for an
//! ordinary IAM principal because it reads the bot user table.
//!
//! userinfo declares no response schema — one of 684 operations in the document
//! that state the route and not its shape — so the generated function returns
//! `Result<(), _>` and drops the body. The call proves the route and the
//! credential; `get_json` then reads what the API actually sent, over the same
//! configuration. When the schema lands, this collapses into one typed call.
//!
//! ```bash
//! HANZO_API_KEY=sk-... cargo run -p hanzo-examples --example hello
//! ```

use hanzo_client::apis::iam_api;
use hanzo_examples::{config, get_json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config(None);

    iam_api::get_v1_iam_oauth_userinfo(&cfg).await?;
    let me = get_json(&cfg, "/v1/iam/oauth/userinfo").await?;

    println!("sub     {}", me["sub"].as_str().unwrap_or("(none)"));
    println!("name    {}", me["name"].as_str().unwrap_or("(unnamed)"));
    println!("email   {}", me["email"].as_str().unwrap_or("(no email)"));
    Ok(())
}

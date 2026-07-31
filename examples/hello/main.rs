//! hello — who am I?
//!
//! The smallest complete round trip: authenticate with HANZO_API_KEY and read
//! back the identity the API resolved it to.
//!
//! Operation: GET /v1/bot/auth/me (bot_authMe)
//!
//! The route has to FAIL on a bad key — that is the whole point of the flow.
//! This one answers 403 "no validated principal" with no key and with a bogus
//! one, and the typed user with a real one.
//!
//! ```bash
//! HANZO_API_KEY=sk-... cargo run -p hanzo-examples --example hello
//! ```

use hanzo_client::apis::auth_api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = hanzo_examples::config(None);

    let me = auth_api::bot_auth_me(&cfg).await?;

    println!("id      {}", me.id.unwrap_or_default());
    println!("handle  {}", me.handle.unwrap_or_default());
    println!("name    {}", me.display_name.unwrap_or_default());
    Ok(())
}

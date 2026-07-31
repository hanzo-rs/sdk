//! hello — who am I?
//!
//! The smallest complete round trip: authenticate with HANZO_API_KEY and read
//! back the identity the API resolved it to.
//!
//! Operation: GET /v1/bot/whoami (bot_whoami)
//!
//! ```bash
//! HANZO_API_KEY=sk-... cargo run -p hanzo-examples --example hello
//! ```

use hanzo_client::apis::auth_api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = hanzo_examples::config(None);

    let me = auth_api::bot_whoami(&cfg).await?;

    println!("id      {}", me.id.unwrap_or_default());
    println!("handle  {}", me.handle.unwrap_or_default());
    println!("email   {}", me.email.unwrap_or_default());
    Ok(())
}

//! store — put, get, delete.
//!
//! A full key/value round trip against the Hanzo KV service.
//!
//! Operations: PUT /v1/kv/keys/{key} (kv_setKey), GET /v1/kv/keys/{key}
//! (kv_getKey), DELETE /v1/kv/keys/{key} (kv_deleteKey).
//!
//! KV is org-scoped, so it needs X-Org-Id alongside the API key.
//!
//! ```bash
//! HANZO_API_KEY=sk-... HANZO_ORG_ID=my-org cargo run -p hanzo-examples --example store
//! ```

use hanzo_client::apis::keys_api;
use hanzo_client::models::KvSetKeyRequest;

const KEY: &str = "hanzo-sdk-example";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = hanzo_examples::config_from_env();

    let req = KvSetKeyRequest::new("hello from the Rust SDK".into());
    let set = keys_api::kv_set_key(&cfg, KEY, req, None).await?;
    println!("put     {KEY} = {:?}", set.value.unwrap_or_default());

    let got = keys_api::kv_get_key(&cfg, KEY, None).await?;
    println!("get     {KEY} = {:?}", got.value.unwrap_or_default());

    keys_api::kv_delete_key(&cfg, KEY, None).await?;
    println!("delete  {KEY}");
    Ok(())
}

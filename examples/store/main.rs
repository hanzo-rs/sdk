//! store — provision a KV store, read it back, delete it.
//!
//! Operations: POST /v1/kv (cloud_post_v1_kv), GET /v1/kv/{name}
//! (cloud_get_v1_kv_name), DELETE /v1/kv/{name} (cloud_delete_v1_kv_name).
//!
//! This is the provisioning plane, and it is the one that answers. The value
//! plane — /v1/kv/keys/{key}, kv_setKey/kv_getKey/kv_deleteKey — is authored in
//! the spec but is mounted nowhere: it replies 404 to GET and 405 to PUT and
//! DELETE at api.hanzo.ai, while /v1/kv replies 403. An example may only call
//! what routes.
//!
//! KV is org-scoped, so it needs X-Org-Id alongside the API key.
//!
//! ```bash
//! HANZO_API_KEY=sk-... HANZO_ORG_ID=my-org cargo run -p hanzo-examples --example store
//! ```

use hanzo_client::apis::kv_api;
use hanzo_client::models::CloudProvisionRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = hanzo_examples::config_from_env();

    // Names are org-unique, so a hardcoded one collides with the last run.
    let name = format!("sdk-example-{}", std::process::id());

    let mut req = CloudProvisionRequest::new();
    req.name = Some(name.clone());
    let created = kv_api::cloud_post_v1_kv(&cfg, Some(req)).await?;
    println!(
        "create   {} ({})",
        created.name.unwrap_or_default(),
        created.status.unwrap_or_default()
    );

    // Read, then delete whether or not the read succeeded, so a failure does
    // not leave the store behind for the next run to collide with.
    let read = kv_api::cloud_get_v1_kv_name(&cfg, &name).await;
    let deleted = kv_api::cloud_delete_v1_kv_name(&cfg, &name).await;

    let got = read?;
    println!(
        "read     {} kind={} host={}",
        got.name.unwrap_or_default(),
        got.kind.unwrap_or_default(),
        got.host.unwrap_or_default()
    );

    deleted?;
    println!("delete   {name}");
    Ok(())
}

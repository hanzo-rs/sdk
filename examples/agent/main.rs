//! agent — create it, run it, read it back.
//!
//! Operations: POST /v1/agents (cloud_post_v1_agents),
//! POST /v1/agents/{ref}/run (cloud_post_v1_agents_by_ref_run),
//! GET /v1/agents/{ref} (cloud_get_v1_agents_ref).
//!
//! Agents are org-scoped, so this needs X-Org-Id alongside the API key.
//!
//! Create and read are typed. Run declares only a `default` response with no
//! content schema in hanzo.yaml, so its generated function returns
//! `Result<(), _>` and the run result is not available from it.
//!
//! ```bash
//! HANZO_API_KEY=sk-... HANZO_ORG_ID=my-org cargo run -p hanzo-examples --example agent
//! ```

use hanzo_client::apis::agents_api;
use hanzo_client::models::CloudCreateAgentIn;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = hanzo_examples::config_from_env();
    let model = std::env::var("HANZO_MODEL").unwrap_or_else(|_| "zen-1".into());

    let mut create = CloudCreateAgentIn::new();
    create.name = Some("sdk-example".into());
    create.model = Some(model);
    create.instructions = Some("You answer in exactly one sentence.".into());

    let created = agents_api::cloud_post_v1_agents(&cfg, create).await?;
    let agent_ref = created.id.clone().unwrap_or_default();
    println!("created  {agent_ref} ({})", created.name.unwrap_or_default());

    agents_api::cloud_post_v1_agents_by_ref_run(&cfg, &agent_ref).await?;
    println!("ran      {agent_ref}");

    let agent = agents_api::cloud_get_v1_agents_ref(&cfg, &agent_ref).await?;
    println!(
        "read     {} runs={}",
        agent.name.unwrap_or_default(),
        agent.runs.unwrap_or_default()
    );
    Ok(())
}

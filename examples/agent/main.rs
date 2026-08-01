//! agent — create an agent, run it, read the run back.
//!
//! Operations: POST /v1/agents (post_v1_agents),
//! POST /v1/agents/{ref}/run (post_v1_agents_by_ref_run),
//! GET /v1/agents/{ref}/runs (get_v1_agents_ref_runs).
//!
//! `ref` accepts the public id (agent_...) or the org-unique name, so run and
//! read both use the name just created without waiting for an id.
//!
//! A run is asynchronous, so the last step polls the run list until the run
//! this program started reaches a terminal status. The run POST declares only a
//! `default` response with no content schema in hanzo.yaml, so its generated
//! function returns `()` and the run is identified by reading the list.
//!
//! Agents are org-scoped, so this needs X-Org-Id alongside the API key.
//!
//! ```bash
//! HANZO_API_KEY=sk-... HANZO_ORG_ID=my-org cargo run -p hanzo-examples --example agent
//! ```

use hanzo_client::apis::{agents_api, configuration::Configuration};
use hanzo_client::models::{AgentRunView, CreateAgentIn};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = hanzo_examples::config_from_env();
    let model = std::env::var("HANZO_MODEL").unwrap_or_else(|_| "zen-1".into());

    // Names are org-unique, so a hardcoded one collides with the last run.
    let name = format!("sdk-example-{}", std::process::id());

    let mut create = CreateAgentIn::new();
    create.name = Some(name.clone());
    create.model = Some(model);
    create.instructions = Some("You answer in exactly one sentence.".into());

    let created = agents_api::post_v1_agents(&cfg, create).await?;
    println!(
        "created  {} ({})",
        created.name.unwrap_or_default(),
        created.id.unwrap_or_default()
    );

    agents_api::post_v1_agents_by_ref_run(&cfg, &name).await?;
    println!("started  a run on {name}");

    let run = poll(&cfg, &name).await?;
    println!(
        "status   {} in {}ms",
        run.status.unwrap_or_default(),
        run.duration_ms.unwrap_or_default()
    );
    println!("output   {}", run.output.unwrap_or_default());
    Ok(())
}

/// Read the run list until its newest run reaches a terminal status.
async fn poll(
    cfg: &Configuration,
    agent_ref: &str,
) -> Result<AgentRunView, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(120);
    while Instant::now() < deadline {
        let list = agents_api::get_v1_agents_ref_runs(cfg, agent_ref, Some(1)).await?;
        // Runs are newest first.
        if let Some(run) = list.runs.unwrap_or_default().into_iter().next() {
            if matches!(
                run.status.as_deref(),
                Some("ok" | "error" | "failed" | "cancelled")
            ) {
                return Ok(run);
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(format!("run on {agent_ref} did not finish within 2m").into())
}

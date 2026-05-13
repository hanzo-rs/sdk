//! Advanced marketplace example with multiple agents and scenarios

use hanzo_agent_marketplace::{
    AgentMarketplace, ServiceOfferOptions, ServiceRequestOptions, ServiceType,
};
use std::collections::HashMap;

fn main() {
    println!("=== Advanced Agent Marketplace Demo ===\n");

    let marketplace = AgentMarketplace::new();

    // Scenario 1: Multiple providers competing
    println!("📋 Scenario 1: Price Competition");
    println!("--------------------------------");

    marketplace.post_offer(
        "agent_fast",
        "FastBot",
        ServiceType::Inference,
        "Fast LLM inference (CPU)",
        0.3,
        None,
    );

    marketplace.post_offer(
        "agent_premium",
        "PremiumBot",
        ServiceType::Inference,
        "Premium LLM inference (GPU)",
        0.8,
        Some(ServiceOfferOptions::new().requires_tee(true)),
    );

    marketplace.post_offer(
        "agent_budget",
        "BudgetBot",
        ServiceType::Inference,
        "Budget LLM inference",
        0.1,
        None,
    );

    // Request should match the cheapest offer (BudgetBot)
    marketplace.post_request(
        "user_1",
        "Charlie",
        ServiceType::Inference,
        "Need quick LLM inference",
        0.5,
        None,
    );

    let matches = marketplace.get_matches_for_agent("agent_budget");
    println!("✓ Matched with: BudgetBot (cheapest at 0.1 ETH)");
    println!("  Total matches: {}\n", matches.len());

    // Scenario 2: TEE requirement filtering
    println!("📋 Scenario 2: TEE Requirement");
    println!("--------------------------------");

    marketplace.post_offer(
        "agent_secure",
        "SecureBot",
        ServiceType::Compute,
        "Secure TEE compute",
        1.0,
        Some(ServiceOfferOptions::new().requires_tee(true)),
    );

    marketplace.post_offer(
        "agent_regular",
        "RegularBot",
        ServiceType::Compute,
        "Regular compute",
        0.5,
        None,
    );

    // Request requires TEE - should only match SecureBot
    marketplace.post_request(
        "user_2",
        "Diana",
        ServiceType::Compute,
        "Need confidential computing",
        2.0,
        Some(ServiceRequestOptions::new().requires_tee(true)),
    );

    let _secure_matches = marketplace.get_matches_for_agent("agent_secure");
    println!("✓ Matched with: SecureBot (has TEE)");
    println!("  Regular offers filtered out\n");

    // Scenario 3: Reputation filtering
    println!("📋 Scenario 3: Reputation Filtering");
    println!("--------------------------------");

    // Build reputation for an agent
    let _offer_id = marketplace.post_offer(
        "agent_expert",
        "ExpertBot",
        ServiceType::Coding,
        "Expert development",
        1.5,
        None,
    );

    let _request_id = marketplace.post_request(
        "user_3",
        "Eve",
        ServiceType::Coding,
        "Simple task",
        2.0,
        None,
    );

    // Complete the match to build reputation
    let expert_matches = marketplace.get_matches_for_agent("agent_expert");
    if let Some(m) = expert_matches.first() {
        let mut result = HashMap::new();
        result.insert("status".to_string(), serde_json::json!("success"));
        marketplace
            .complete_match(&m.match_id, result, None)
            .unwrap();
    }

    let expert_rep = marketplace.get_reputation("agent_expert");
    println!("✓ ExpertBot reputation after completion: {:.2}", expert_rep);

    // Now post high-reputation-required request
    marketplace.post_offer(
        "agent_expert",
        "ExpertBot",
        ServiceType::Coding,
        "Expert development",
        1.5,
        None,
    );

    marketplace.post_request(
        "user_4",
        "Frank",
        ServiceType::Coding,
        "Critical project",
        3.0,
        Some(ServiceRequestOptions::new().min_reputation(0.55)), // Requires good reputation
    );

    let _high_rep_matches = marketplace.get_matches_for_agent("agent_expert");
    println!("✓ High-reputation request matched with ExpertBot\n");

    // Scenario 4: Duration constraints
    println!("📋 Scenario 4: Duration Constraints");
    println!("--------------------------------");

    marketplace.post_offer(
        "agent_quick",
        "QuickBot",
        ServiceType::Analysis,
        "Quick analysis",
        0.5,
        Some(ServiceOfferOptions::new().max_duration_hours(2.0)),
    );

    marketplace.post_offer(
        "agent_patient",
        "PatientBot",
        ServiceType::Analysis,
        "Patient analysis",
        0.7,
        Some(ServiceOfferOptions::new().max_duration_hours(48.0)),
    );

    // Short task - both can handle
    marketplace.post_request(
        "user_5",
        "Grace",
        ServiceType::Analysis,
        "1 hour task",
        1.0,
        Some(ServiceRequestOptions::new().duration_hours(1.0)),
    );

    // Should match QuickBot (cheaper)
    println!("✓ Short task matched with QuickBot (cheaper)\n");

    // Scenario 5: Expiration handling
    println!("📋 Scenario 5: Expiration Handling");
    println!("--------------------------------");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    // Post expired offer
    marketplace.post_offer(
        "agent_late",
        "LateBot",
        ServiceType::Research,
        "Expired offer",
        0.5,
        Some(ServiceOfferOptions::new().expires_at(now - 100.0)),
    );

    let active = marketplace.get_active_offers(Some(ServiceType::Research));
    println!("✓ Expired offers filtered out: {} active", active.len());
    println!();

    // Final statistics
    println!("📊 Final Marketplace Statistics");
    println!("================================");
    let stats = marketplace.get_stats();
    println!("Active Offers: {}", stats.active_offers);
    println!("Active Requests: {}", stats.active_requests);
    println!("Total Matches: {}", stats.total_matches);
    println!("Total Volume: {} ETH", stats.total_volume_eth);
    println!("Completed Transactions: {}", stats.total_transactions);
    println!("Unique Providers: {}", stats.unique_providers);
    println!("Unique Requesters: {}", stats.unique_requesters);

    // Show all service types
    println!("\n📋 Available Service Types:");
    println!("  - Data: Data collection and processing");
    println!("  - Compute: Compute resources");
    println!("  - Inference: AI/ML inference");
    println!("  - Analysis: Data analysis");
    println!("  - Coding: Code generation");
    println!("  - Research: Research tasks");
    println!("  - Custom: Custom services");
}

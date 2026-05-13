//! Basic usage example of the agent marketplace

use hanzo_agent_marketplace::{
    AgentMarketplace, ServiceOfferOptions, ServiceRequestOptions, ServiceType,
};
use std::collections::HashMap;

fn main() {
    println!("=== Agent Marketplace Example ===\n");

    // Create marketplace
    let marketplace = AgentMarketplace::new();

    // Post a coding service offer
    println!("📝 Posting coding service offer...");
    let offer_id = marketplace.post_offer(
        "agent_0x123abc",
        "CodeBot",
        ServiceType::Coding,
        "Expert Python code generation and debugging",
        0.5, // 0.5 ETH
        Some(
            ServiceOfferOptions::new()
                .max_duration_hours(48.0)
                .min_reputation(0.3),
        ),
    );
    println!("✓ Offer posted: {}\n", offer_id);

    // Post a data analysis service offer
    println!("📝 Posting data analysis offer...");
    marketplace.post_offer(
        "agent_0x456def",
        "DataBot",
        ServiceType::Analysis,
        "Statistical analysis and ML modeling",
        0.8,
        Some(ServiceOfferOptions::new().requires_tee(true)),
    );
    println!("✓ Analysis offer posted\n");

    // Check active offers
    let offers = marketplace.get_active_offers(None);
    println!("📊 Active offers: {}", offers.len());
    for offer in &offers {
        println!(
            "  - {} by {} ({}) @ {} ETH",
            offer.service_type, offer.agent_name, offer.description, offer.price_eth
        );
    }
    println!();

    // Post a request for coding service
    println!("🔍 Posting request for coding service...");
    marketplace.post_request(
        "user_0x789ghi",
        "Alice",
        ServiceType::Coding,
        "Need Python FastAPI backend development",
        1.0, // Max 1.0 ETH
        Some(ServiceRequestOptions::new().duration_hours(24.0)),
    );
    println!("✓ Request posted\n");

    // Check if match was created automatically
    let stats = marketplace.get_stats();
    println!("📈 Marketplace Statistics:");
    println!("  Active Offers: {}", stats.active_offers);
    println!("  Active Requests: {}", stats.active_requests);
    println!("  Total Matches: {}", stats.total_matches);
    println!("  Total Transactions: {}", stats.total_transactions);
    println!();

    if stats.total_matches > 0 {
        println!("✨ Match created automatically!");

        // Get matches for the agent
        let matches = marketplace.get_matches_for_agent("agent_0x123abc");
        if let Some(m) = matches.first() {
            println!("  Match ID: {}", m.match_id);
            println!("  Provider: {}", m.offer.agent_name);
            println!("  Requester: {}", m.request.requester_name);
            println!("  Price: {} ETH", m.agreed_price_eth);
            println!("  Status: {}", m.status);
            println!();

            // Complete the match
            println!("✅ Completing match...");
            let mut result = HashMap::new();
            result.insert(
                "deliverable".to_string(),
                serde_json::json!("https://github.com/alice/fastapi-backend"),
            );
            result.insert("lines_of_code".to_string(), serde_json::json!(1250));

            marketplace
                .complete_match(&m.match_id, result, None)
                .unwrap();
            println!("✓ Match completed!\n");

            // Check reputation update
            let rep = marketplace.get_reputation("agent_0x123abc");
            println!("⭐ CodeBot reputation: {:.2}", rep);
        }
    }

    // Final statistics
    let final_stats = marketplace.get_stats();
    println!("\n📊 Final Statistics:");
    println!("  Total Volume: {} ETH", final_stats.total_volume_eth);
    println!(
        "  Completed Transactions: {}",
        final_stats.total_transactions
    );
}

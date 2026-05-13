//! Decentralized marketplace for agent services and resources.
//!
//! This crate provides a marketplace system for AI agents to discover, offer,
//! and request services. It includes:
//! - Service type enumeration
//! - Service offers and requests
//! - Automatic matching logic
//! - Reputation tracking
//! - Statistics and monitoring

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub mod error;
pub mod types;

pub use error::{MarketplaceError, Result};
pub use types::{ServiceMatch, ServiceOffer, ServiceRequest, ServiceType};

/// Get current Unix timestamp in seconds
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

/// Statistics for the marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStats {
    /// Number of active offers
    pub active_offers: usize,
    /// Number of active requests
    pub active_requests: usize,
    /// Total number of matches
    pub total_matches: usize,
    /// Total volume in ETH
    pub total_volume_eth: f64,
    /// Total completed transactions
    pub total_transactions: usize,
    /// Number of unique providers
    pub unique_providers: usize,
    /// Number of unique requesters
    pub unique_requesters: usize,
}

/// Agent marketplace for service discovery and matching
#[derive(Debug, Clone)]
pub struct AgentMarketplace {
    /// Active service offers
    offers: Arc<DashMap<String, ServiceOffer>>,
    /// Active service requests
    requests: Arc<DashMap<String, ServiceRequest>>,
    /// Matched services
    matches: Arc<DashMap<String, ServiceMatch>>,
    /// Reputation scores (agent_address -> score)
    reputation_scores: Arc<DashMap<String, f64>>,
    /// Completed transaction count (agent_address -> count)
    completed_transactions: Arc<DashMap<String, usize>>,
    /// Total volume in ETH
    total_volume_eth: Arc<std::sync::Mutex<f64>>,
    /// Total transaction count
    total_transactions: Arc<std::sync::Mutex<usize>>,
}

impl Default for AgentMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentMarketplace {
    /// Create a new marketplace instance
    pub fn new() -> Self {
        Self {
            offers: Arc::new(DashMap::new()),
            requests: Arc::new(DashMap::new()),
            matches: Arc::new(DashMap::new()),
            reputation_scores: Arc::new(DashMap::new()),
            completed_transactions: Arc::new(DashMap::new()),
            total_volume_eth: Arc::new(std::sync::Mutex::new(0.0)),
            total_transactions: Arc::new(std::sync::Mutex::new(0)),
        }
    }

    /// Post a service offer
    ///
    /// # Arguments
    /// * `agent_address` - Agent's blockchain address
    /// * `agent_name` - Agent's name
    /// * `service_type` - Type of service offered
    /// * `description` - Service description
    /// * `price_eth` - Price in ETH
    /// * `options` - Additional offer options
    ///
    /// # Returns
    /// Offer ID
    pub fn post_offer(
        &self,
        agent_address: impl Into<String>,
        agent_name: impl Into<String>,
        service_type: ServiceType,
        description: impl Into<String>,
        price_eth: f64,
        options: Option<ServiceOfferOptions>,
    ) -> String {
        let offer_id = format!("offer_{}", &Uuid::new_v4().simple().to_string()[..8]);
        let opts = options.unwrap_or_default();

        let offer = ServiceOffer {
            id: offer_id.clone(),
            agent_address: agent_address.into(),
            agent_name: agent_name.into(),
            service_type,
            description: description.into(),
            price_eth,
            min_reputation: opts.min_reputation,
            max_duration_hours: opts.max_duration_hours,
            requires_tee: opts.requires_tee,
            metadata: opts.metadata,
            created_at: now(),
            expires_at: opts.expires_at,
        };

        self.offers.insert(offer_id.clone(), offer);

        // Try to match with existing requests
        self.try_match_offers();

        offer_id
    }

    /// Post a service request
    ///
    /// # Arguments
    /// * `requester_address` - Requester's blockchain address
    /// * `requester_name` - Requester's name
    /// * `service_type` - Type of service needed
    /// * `description` - Service description
    /// * `max_price_eth` - Maximum price willing to pay
    /// * `options` - Additional request options
    ///
    /// # Returns
    /// Request ID
    pub fn post_request(
        &self,
        requester_address: impl Into<String>,
        requester_name: impl Into<String>,
        service_type: ServiceType,
        description: impl Into<String>,
        max_price_eth: f64,
        options: Option<ServiceRequestOptions>,
    ) -> String {
        let request_id = format!("request_{}", &Uuid::new_v4().simple().to_string()[..8]);
        let opts = options.unwrap_or_default();

        let request = ServiceRequest {
            id: request_id.clone(),
            requester_address: requester_address.into(),
            requester_name: requester_name.into(),
            service_type,
            description: description.into(),
            max_price_eth,
            duration_hours: opts.duration_hours,
            min_reputation: opts.min_reputation,
            requires_tee: opts.requires_tee,
            metadata: opts.metadata,
            created_at: now(),
            expires_at: opts.expires_at,
        };

        self.requests.insert(request_id.clone(), request);

        // Try to match with existing offers
        self.try_match_offers();

        request_id
    }

    /// Try to match offers with requests
    fn try_match_offers(&self) {
        // Clean expired items first
        self.clean_expired();

        // Try to match each request
        let requests: Vec<_> = self.requests.iter().map(|r| r.value().clone()).collect();

        for request in requests {
            let mut best_offer: Option<ServiceOffer> = None;
            let mut best_price = f64::INFINITY;

            // Find best matching offer
            for offer_ref in self.offers.iter() {
                let offer = offer_ref.value();

                if offer.matches_request(&request) {
                    // Check reputation requirements
                    let provider_rep = self.get_reputation(&offer.agent_address);
                    if provider_rep < request.min_reputation {
                        continue;
                    }

                    let requester_rep = self.get_reputation(&request.requester_address);
                    if requester_rep < offer.min_reputation {
                        continue;
                    }

                    // Track best price
                    if offer.price_eth < best_price {
                        best_offer = Some(offer.clone());
                        best_price = offer.price_eth;
                    }
                }
            }

            // Create match if found
            if let Some(offer) = best_offer {
                self.create_match(offer, request);
            }
        }
    }

    /// Create a match between offer and request
    fn create_match(&self, offer: ServiceOffer, request: ServiceRequest) {
        let match_id = format!("match_{}", &Uuid::new_v4().simple().to_string()[..8]);

        // Agreed price is the offer price (could implement negotiation)
        let agreed_price = offer.price_eth;

        let service_match = ServiceMatch {
            match_id: match_id.clone(),
            offer: offer.clone(),
            request: request.clone(),
            agreed_price_eth: agreed_price,
            escrow_tx: None,
            status: "pending".to_string(),
            created_at: now(),
            completed_at: None,
            result: None,
            attestation: None,
        };

        self.matches.insert(match_id, service_match);

        // Remove from active lists
        self.offers.remove(&offer.id);
        self.requests.remove(&request.id);

        // Update statistics
        let mut total_tx = self.total_transactions.lock().unwrap();
        *total_tx += 1;

        println!(
            "Match created: {} -> {} for {} ETH",
            offer.agent_name, request.requester_name, agreed_price
        );
    }

    /// Complete a match
    ///
    /// # Arguments
    /// * `match_id` - Match ID
    /// * `result` - Result of the service
    /// * `attestation` - Optional TEE attestation
    pub fn complete_match(
        &self,
        match_id: &str,
        result: HashMap<String, serde_json::Value>,
        attestation: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        let mut service_match = self
            .matches
            .get_mut(match_id)
            .ok_or_else(|| MarketplaceError::MatchNotFound(match_id.to_string()))?;

        service_match.status = "completed".to_string();
        service_match.completed_at = Some(now());
        service_match.result = Some(result);
        service_match.attestation = attestation;

        // Update reputation
        self.update_reputation(&service_match.offer.agent_address, 1.0);

        // Update statistics
        let mut total_volume = self.total_volume_eth.lock().unwrap();
        *total_volume += service_match.agreed_price_eth;

        // Track completed transactions
        self.completed_transactions
            .entry(service_match.offer.agent_address.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        Ok(())
    }

    /// Dispute a match
    ///
    /// # Arguments
    /// * `match_id` - Match ID
    /// * `reason` - Dispute reason
    pub fn dispute_match(&self, match_id: &str, reason: impl Into<String>) -> Result<()> {
        let mut service_match = self
            .matches
            .get_mut(match_id)
            .ok_or_else(|| MarketplaceError::MatchNotFound(match_id.to_string()))?;

        service_match.status = "disputed".to_string();

        let mut dispute_result = HashMap::new();
        dispute_result.insert(
            "dispute_reason".to_string(),
            serde_json::Value::String(reason.into()),
        );
        service_match.result = Some(dispute_result);

        // Negative reputation for provider
        self.update_reputation(&service_match.offer.agent_address, -0.5);

        Ok(())
    }

    /// Get agent reputation score
    ///
    /// # Arguments
    /// * `agent_address` - Agent's blockchain address
    ///
    /// # Returns
    /// Reputation score (0.0-1.0)
    pub fn get_reputation(&self, agent_address: &str) -> f64 {
        self.reputation_scores
            .get(agent_address)
            .map(|r| *r.value())
            .unwrap_or(0.5)
    }

    /// Update agent reputation
    fn update_reputation(&self, agent_address: &str, delta: f64) {
        let current = self.get_reputation(agent_address);
        let new_score = (current + delta * 0.1).clamp(0.0, 1.0); // Damped update
        self.reputation_scores
            .insert(agent_address.to_string(), new_score);
    }

    /// Remove expired offers and requests
    fn clean_expired(&self) {
        let current_time = now();

        // Clean offers
        self.offers.retain(|_, offer| {
            if let Some(expires_at) = offer.expires_at {
                current_time <= expires_at
            } else {
                true
            }
        });

        // Clean requests
        self.requests.retain(|_, request| {
            if let Some(expires_at) = request.expires_at {
                current_time <= expires_at
            } else {
                true
            }
        });
    }

    /// Get active offers, optionally filtered by type
    pub fn get_active_offers(&self, service_type: Option<ServiceType>) -> Vec<ServiceOffer> {
        self.clean_expired();

        let mut offers: Vec<ServiceOffer> = self.offers.iter().map(|r| r.value().clone()).collect();

        if let Some(st) = service_type {
            offers.retain(|o| o.service_type == st);
        }

        offers.sort_by(|a, b| a.price_eth.partial_cmp(&b.price_eth).unwrap());
        offers
    }

    /// Get active requests, optionally filtered by type
    pub fn get_active_requests(&self, service_type: Option<ServiceType>) -> Vec<ServiceRequest> {
        self.clean_expired();

        let mut requests: Vec<ServiceRequest> =
            self.requests.iter().map(|r| r.value().clone()).collect();

        if let Some(st) = service_type {
            requests.retain(|r| r.service_type == st);
        }

        requests.sort_by(|a, b| b.max_price_eth.partial_cmp(&a.max_price_eth).unwrap());
        requests
    }

    /// Get all matches involving an agent
    pub fn get_matches_for_agent(&self, agent_address: &str) -> Vec<ServiceMatch> {
        let mut matches: Vec<ServiceMatch> = self
            .matches
            .iter()
            .filter(|m| {
                m.value().offer.agent_address == agent_address
                    || m.value().request.requester_address == agent_address
            })
            .map(|r| r.value().clone())
            .collect();

        matches.sort_by(|a, b| b.created_at.partial_cmp(&a.created_at).unwrap());
        matches
    }

    /// Get marketplace statistics
    pub fn get_stats(&self) -> MarketplaceStats {
        let unique_providers = self
            .offers
            .iter()
            .map(|r| r.value().agent_address.clone())
            .collect::<std::collections::HashSet<_>>()
            .len();

        let unique_requesters = self
            .requests
            .iter()
            .map(|r| r.value().requester_address.clone())
            .collect::<std::collections::HashSet<_>>()
            .len();

        MarketplaceStats {
            active_offers: self.offers.len(),
            active_requests: self.requests.len(),
            total_matches: self.matches.len(),
            total_volume_eth: *self.total_volume_eth.lock().unwrap(),
            total_transactions: *self.total_transactions.lock().unwrap(),
            unique_providers,
            unique_requesters,
        }
    }
}

/// Options for posting a service offer
#[derive(Debug, Clone, Default)]
pub struct ServiceOfferOptions {
    pub min_reputation: f64,
    pub max_duration_hours: f64,
    pub requires_tee: bool,
    pub metadata: HashMap<String, serde_json::Value>,
    pub expires_at: Option<f64>,
}

impl ServiceOfferOptions {
    pub fn new() -> Self {
        Self {
            min_reputation: 0.0,
            max_duration_hours: 24.0,
            requires_tee: false,
            metadata: HashMap::new(),
            expires_at: None,
        }
    }

    pub fn min_reputation(mut self, value: f64) -> Self {
        self.min_reputation = value;
        self
    }

    pub fn max_duration_hours(mut self, value: f64) -> Self {
        self.max_duration_hours = value;
        self
    }

    pub fn requires_tee(mut self, value: bool) -> Self {
        self.requires_tee = value;
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn expires_at(mut self, value: f64) -> Self {
        self.expires_at = Some(value);
        self
    }
}

/// Options for posting a service request
#[derive(Debug, Clone, Default)]
pub struct ServiceRequestOptions {
    pub duration_hours: f64,
    pub min_reputation: f64,
    pub requires_tee: bool,
    pub metadata: HashMap<String, serde_json::Value>,
    pub expires_at: Option<f64>,
}

impl ServiceRequestOptions {
    pub fn new() -> Self {
        Self {
            duration_hours: 1.0,
            min_reputation: 0.0,
            requires_tee: false,
            metadata: HashMap::new(),
            expires_at: None,
        }
    }

    pub fn duration_hours(mut self, value: f64) -> Self {
        self.duration_hours = value;
        self
    }

    pub fn min_reputation(mut self, value: f64) -> Self {
        self.min_reputation = value;
        self
    }

    pub fn requires_tee(mut self, value: bool) -> Self {
        self.requires_tee = value;
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn expires_at(mut self, value: f64) -> Self {
        self.expires_at = Some(value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_creation() {
        let marketplace = AgentMarketplace::new();
        let stats = marketplace.get_stats();

        assert_eq!(stats.active_offers, 0);
        assert_eq!(stats.active_requests, 0);
        assert_eq!(stats.total_matches, 0);
    }

    #[test]
    fn test_post_offer() {
        let marketplace = AgentMarketplace::new();

        let offer_id = marketplace.post_offer(
            "agent_123",
            "CodeBot",
            ServiceType::Coding,
            "Python code generation",
            0.5,
            None,
        );

        assert!(offer_id.starts_with("offer_"));
        let offers = marketplace.get_active_offers(None);
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].agent_name, "CodeBot");
    }

    #[test]
    fn test_post_request() {
        let marketplace = AgentMarketplace::new();

        let request_id = marketplace.post_request(
            "user_456",
            "Alice",
            ServiceType::Analysis,
            "Data analysis needed",
            1.0,
            None,
        );

        assert!(request_id.starts_with("request_"));
        let requests = marketplace.get_active_requests(None);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].requester_name, "Alice");
    }

    #[test]
    fn test_automatic_matching() {
        let marketplace = AgentMarketplace::new();

        // Post offer first
        marketplace.post_offer(
            "agent_123",
            "CodeBot",
            ServiceType::Coding,
            "Python code generation",
            0.5,
            None,
        );

        // Post matching request
        marketplace.post_request(
            "user_456",
            "Alice",
            ServiceType::Coding,
            "Need Python code",
            1.0,
            None,
        );

        // Should be matched automatically
        let stats = marketplace.get_stats();
        assert_eq!(stats.active_offers, 0);
        assert_eq!(stats.active_requests, 0);
        assert_eq!(stats.total_matches, 1);
    }

    #[test]
    fn test_price_matching() {
        let marketplace = AgentMarketplace::new();

        // Offer with price 1.5 ETH
        marketplace.post_offer(
            "agent_123",
            "ExpensiveBot",
            ServiceType::Inference,
            "AI inference",
            1.5,
            None,
        );

        // Request with max price 1.0 ETH (too low)
        marketplace.post_request(
            "user_456",
            "Alice",
            ServiceType::Inference,
            "Need inference",
            1.0,
            None,
        );

        // Should NOT match due to price
        let stats = marketplace.get_stats();
        assert_eq!(stats.active_offers, 1);
        assert_eq!(stats.active_requests, 1);
        assert_eq!(stats.total_matches, 0);
    }

    #[test]
    fn test_reputation_tracking() {
        let marketplace = AgentMarketplace::new();

        // Initial reputation should be 0.5
        let rep = marketplace.get_reputation("agent_123");
        assert_eq!(rep, 0.5);

        // Update reputation
        marketplace.update_reputation("agent_123", 1.0);
        let new_rep = marketplace.get_reputation("agent_123");
        assert!(new_rep > 0.5);
        assert!(new_rep <= 1.0);
    }

    #[test]
    fn test_complete_match() {
        let marketplace = AgentMarketplace::new();

        // Create a match
        marketplace.post_offer(
            "agent_123",
            "CodeBot",
            ServiceType::Coding,
            "Python code generation",
            0.5,
            None,
        );

        marketplace.post_request(
            "user_456",
            "Alice",
            ServiceType::Coding,
            "Need Python code",
            1.0,
            None,
        );

        // Get the match ID
        let matches = marketplace.get_matches_for_agent("agent_123");
        assert_eq!(matches.len(), 1);
        let match_id = &matches[0].match_id;

        // Complete the match
        let mut result = HashMap::new();
        result.insert("code".to_string(), serde_json::json!("print('hello')"));

        marketplace.complete_match(match_id, result, None).unwrap();

        // Check updated match
        let updated_matches = marketplace.get_matches_for_agent("agent_123");
        assert_eq!(updated_matches[0].status, "completed");
        assert!(updated_matches[0].completed_at.is_some());
    }

    #[test]
    fn test_service_type_filtering() {
        let marketplace = AgentMarketplace::new();

        marketplace.post_offer(
            "agent_1",
            "CodingBot",
            ServiceType::Coding,
            "Coding service",
            0.5,
            None,
        );

        marketplace.post_offer(
            "agent_2",
            "DataBot",
            ServiceType::Data,
            "Data service",
            0.3,
            None,
        );

        // Filter by type
        let coding_offers = marketplace.get_active_offers(Some(ServiceType::Coding));
        assert_eq!(coding_offers.len(), 1);
        assert_eq!(coding_offers[0].service_type, ServiceType::Coding);

        let data_offers = marketplace.get_active_offers(Some(ServiceType::Data));
        assert_eq!(data_offers.len(), 1);
        assert_eq!(data_offers[0].service_type, ServiceType::Data);
    }

    #[test]
    fn test_expiry() {
        let marketplace = AgentMarketplace::new();

        // Create offer that expires in the past
        let expired_time = now() - 100.0;
        let options = ServiceOfferOptions::new().expires_at(expired_time);

        marketplace.post_offer(
            "agent_123",
            "CodeBot",
            ServiceType::Coding,
            "Expired offer",
            0.5,
            Some(options),
        );

        // Should be cleaned when getting active offers
        let offers = marketplace.get_active_offers(None);
        assert_eq!(offers.len(), 0);
    }

    #[test]
    fn test_tee_requirement_matching() {
        let marketplace = AgentMarketplace::new();

        // Offer without TEE
        marketplace.post_offer(
            "agent_123",
            "RegularBot",
            ServiceType::Compute,
            "Regular compute",
            0.5,
            Some(ServiceOfferOptions::new().requires_tee(false)),
        );

        // Request requiring TEE
        marketplace.post_request(
            "user_456",
            "Alice",
            ServiceType::Compute,
            "Need secure compute",
            1.0,
            Some(ServiceRequestOptions::new().requires_tee(true)),
        );

        // Should NOT match due to TEE requirement
        let stats = marketplace.get_stats();
        assert_eq!(stats.total_matches, 0);
    }
}

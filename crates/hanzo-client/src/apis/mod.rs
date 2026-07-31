use std::error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ResponseContent<T> {
    pub status: reqwest::StatusCode,
    pub content: String,
    pub entity: Option<T>,
}

#[derive(Debug)]
pub enum Error<T> {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Io(std::io::Error),
    ResponseError(ResponseContent<T>),
}

impl <T> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (module, e) = match self {
            Error::Reqwest(e) => ("reqwest", e.to_string()),
            Error::Serde(e) => ("serde", e.to_string()),
            Error::Io(e) => ("IO", e.to_string()),
            Error::ResponseError(e) => ("response", format!("status code {}", e.status)),
        };
        write!(f, "error in {}: {}", module, e)
    }
}

impl <T: fmt::Debug> error::Error for Error<T> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Error::Reqwest(e) => e,
            Error::Serde(e) => e,
            Error::Io(e) => e,
            Error::ResponseError(_) => return None,
        })
    }
}

impl <T> From<reqwest::Error> for Error<T> {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

impl <T> From<serde_json::Error> for Error<T> {
    fn from(e: serde_json::Error) -> Self {
        Error::Serde(e)
    }
}

impl <T> From<std::io::Error> for Error<T> {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub fn urlencode<T: AsRef<str>>(s: T) -> String {
    ::url::form_urlencoded::byte_serialize(s.as_ref().as_bytes()).collect()
}

pub fn parse_deep_object(prefix: &str, value: &serde_json::Value) -> Vec<(String, String)> {
    if let serde_json::Value::Object(object) = value {
        let mut params = vec![];

        for (key, value) in object {
            match value {
                serde_json::Value::Object(_) => params.append(&mut parse_deep_object(
                    &format!("{}[{}]", prefix, key),
                    value,
                )),
                serde_json::Value::Array(array) => {
                    for (i, value) in array.iter().enumerate() {
                        params.append(&mut parse_deep_object(
                            &format!("{}[{}][{}]", prefix, key, i),
                            value,
                        ));
                    }
                },
                serde_json::Value::String(s) => params.push((format!("{}[{}]", prefix, key), s.clone())),
                _ => params.push((format!("{}[{}]", prefix, key), value.to_string())),
            }
        }

        return params;
    }

    unimplemented!("Only objects are supported with style=deepObject")
}

/// Internal use only
/// A content type supported by this client.
#[allow(dead_code)]
enum ContentType {
    Json,
    Text,
    Unsupported(String)
}

impl From<&str> for ContentType {
    fn from(content_type: &str) -> Self {
        if content_type.starts_with("application") && content_type.contains("json") {
            return Self::Json;
        } else if content_type.starts_with("text/plain") {
            return Self::Text;
        } else {
            return Self::Unsupported(content_type.to_string());
        }
    }
}

pub mod ai_api;
pub mod access_tokens_api;
pub mod account_api;
pub mod activities_api;
pub mod admin_api;
pub mod ads_api;
pub mod affiliates_api;
pub mod agent_api;
pub mod agents_api;
pub mod analytics_api;
pub mod answer_api;
pub mod applications_api;
pub mod articles_api;
pub mod ask_api;
pub mod assets_api;
pub mod audit_api;
pub mod auth_api;
pub mod authentication_api;
pub mod authors_api;
pub mod authz_api;
pub mod auto_api;
pub mod automations_api;
pub mod base_api;
pub mod batches_api;
pub mod benchmark_api;
pub mod billing_api;
pub mod bindings_api;
pub mod blueprint_api;
pub mod books_api;
pub mod bot_api;
pub mod bots_api;
pub mod buckets_api;
pub mod builds_api;
pub mod cdn_api;
pub mod campaign_api;
pub mod captable_api;
pub mod cart_api;
pub mod catalog_api;
pub mod channels_api;
pub mod chat_api;
pub mod chats_api;
pub mod checkout_api;
pub mod claude_compatible_api;
pub mod cloud_api;
pub mod cloudflare_api;
pub mod clusters_api;
pub mod code_api;
pub mod collect_api;
pub mod collections_api;
pub mod commerce_api;
pub mod company_api;
pub mod completions_api;
pub mod compliance_api;
pub mod compute_api;
pub mod conflict_api;
pub mod connector_api;
pub mod connectors_api;
pub mod content_api;
pub mod counters_api;
pub mod crawl_api;
pub mod crm_api;
pub mod csrf_api;
pub mod dns_api;
pub mod dnssec_api;
pub mod dashboards_agents_api;
pub mod dashboards_vm_api;
pub mod data_api;
pub mod dataroom_api;
pub mod datastore_api;
pub mod deploy_api;
pub mod deployments_api;
pub mod destinations_api;
pub mod docdb_api;
pub mod documents_api;
pub mod domain_api;
pub mod domains_api;
pub mod download_api;
pub mod dumps_api;
pub mod edge_api;
pub mod embed_status_api;
pub mod embeddings_api;
pub mod enablement_api;
pub mod encryption_api;
pub mod engine_api;
pub mod entitlements_api;
pub mod environments_api;
pub mod errors_api;
pub mod esign_api;
pub mod evals_api;
pub mod event_api;
pub mod events_api;
pub mod exec_api;
pub mod experimental_api;
pub mod experiments_api;
pub mod export_api;
pub mod facet_search_api;
pub mod files_api;
pub mod finance_api;
pub mod flags_api;
pub mod fleet_api;
pub mod flow_api;
pub mod forms_api;
pub mod framework_api;
pub mod functions_api;
pub mod gateway_api;
pub mod geo_api;
pub mod git_api;
pub mod gpus_api;
pub mod graphs_api;
pub mod guide_api;
pub mod hash_api;
pub mod health_api;
pub mod help_api;
pub mod iam_api;
pub mod index_api;
pub mod indexers_api;
pub mod indexes_api;
pub mod ingress_api;
pub mod insights_api;
pub mod integrations_api;
pub mod intel_api;
pub mod k8s_api;
pub mod k8s_status_api;
pub mod kb_api;
pub mod keys_api;
pub mod kms_api;
pub mod kv_api;
pub mod legal_api;
pub mod licensing_api;
pub mod lifecycle_api;
pub mod links_api;
pub mod list_api;
pub mod load_balancers_api;
pub mod logs_api;
pub mod mcp_api;
pub mod mfa_api;
pub mod machines_api;
pub mod marketing_api;
pub mod marketplace_api;
pub mod markets_api;
pub mod me_api;
pub mod meet_api;
pub mod mesh_api;
pub mod messages_api;
pub mod meta_api;
pub mod metrics_api;
pub mod ml_api;
pub mod models_api;
pub mod mq_api;
pub mod multi_search_api;
pub mod namespaces_api;
pub mod network_api;
pub mod networks_api;
pub mod news_api;
pub mod nodes_api;
pub mod notify_api;
pub mod o11y_api;
pub mod objects_api;
pub mod open_ai_compatible_api;
pub mod oracles_api;
pub mod orders_api;
pub mod organizations_api;
pub mod orgs_api;
pub mod pageviews_api;
pub mod payments_api;
pub mod permissions_api;
pub mod personas_api;
pub mod pipelines_api;
pub mod plans_api;
pub mod platform_api;
pub mod plugins_api;
pub mod points_api;
pub mod policies_api;
pub mod preferences_api;
pub mod prefs_api;
pub mod pricing_api;
pub mod pricing_policy_api;
pub mod products_api;
pub mod projects_api;
pub mod prometheus_api;
pub mod promotions_api;
pub mod prompts_api;
pub mod providers_api;
pub mod pub_sub_api;
pub mod realtime_api;
pub mod records_api;
pub mod referrals_api;
pub mod registry_api;
pub mod releases_api;
pub mod remote_connections_api;
pub mod reports_api;
pub mod research_api;
pub mod resources_api;
pub mod roles_permissions_api;
pub mod routes_api;
pub mod run_api;
pub mod runner_api;
pub mod runs_api;
pub mod s3_api;
pub mod sbom_api;
pub mod scales_api;
pub mod scans_api;
pub mod scrape_api;
pub mod search_api;
pub mod search_docs_api;
pub mod secrets_api;
pub mod security_api;
pub mod sentry_api;
pub mod service_api;
pub mod sessions_api;
pub mod settings_api;
pub mod share_api;
pub mod signin_api;
pub mod signout_api;
pub mod similar_api;
pub mod sites_api;
pub mod skills_api;
pub mod snapshots_api;
pub mod social_api;
pub mod spend_api;
pub mod sql_api;
pub mod stats_api;
pub mod store_api;
pub mod stores_api;
pub mod streams_api;
pub mod subscriptions_api;
pub mod swap_api;
pub mod sync_api;
pub mod system_api;
pub mod tasks_api;
pub mod team_api;
pub mod teams_api;
pub mod templates_api;
pub mod tokens_api;
pub mod tools_api;
pub mod traces_api;
pub mod tracker_api;
pub mod train_api;
pub mod training_contribution_api;
pub mod transactions_api;
pub mod translate_api;
pub mod tree_files_api;
pub mod upload_api;
pub mod usage_api;
pub mod usages_api;
pub mod users_api;
pub mod validators_api;
pub mod vector_api;
pub mod vectors_api;
pub mod version_api;
pub mod versioning_api;
pub mod versions_api;
pub mod video_api;
pub mod videos_api;
pub mod vpcs_api;
pub mod wallets_api;
pub mod webhooks_api;
pub mod websearch_api;
pub mod websites_api;
pub mod workflows_api;
pub mod world_api;
pub mod x402_api;
pub mod zones_api;

pub mod configuration;

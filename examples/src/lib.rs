//! The six canonical Hanzo API flows live in this crate's `examples/`
//! directory — `hello`, `chat`, `money`, `store`, `agent`, `tools`. They are
//! the same six journeys in every Hanzo SDK.
//!
//! ```bash
//! HANZO_API_KEY=sk-... cargo run -p hanzo-examples --example hello
//! ```
//!
//! `config()` is the three lines every flow starts with: point at the Hanzo
//! API, carry the key. It is here so the examples show it once rather than six
//! times, and so a reader can see there is nothing else to it.

use hanzo_client::apis::configuration::Configuration;

/// A `Configuration` for the Hanzo API, authenticated from the environment.
///
/// Reads `HANZO_API_KEY`, and `HANZO_BASE_URL` to override the default
/// <https://api.hanzo.ai>. `org` sets `X-Org-Id`, which the org-scoped services
/// (kv, agents) require.
pub fn config(org: Option<String>) -> Configuration {
    let mut cfg = Configuration {
        bearer_access_token: std::env::var("HANZO_API_KEY").ok(),
        ..Default::default()
    };
    if let Ok(base) = std::env::var("HANZO_BASE_URL") {
        cfg.base_path = base;
    }
    if let Some(org) = org {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "X-Org-Id",
            org.parse().expect("HANZO_ORG_ID is not a valid header value"),
        );
        cfg.client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("build http client");
    }
    cfg
}

/// `config` with the org read from `HANZO_ORG_ID`.
pub fn config_from_env() -> Configuration {
    config(std::env::var("HANZO_ORG_ID").ok())
}

/// GET a path and return the decoded JSON body.
///
/// This exists only because 728 of the 2452 operations in `hanzo.yaml` declare
/// no 2xx content schema — many carry a bare `default:` response. The Rust
/// generator turns those into `Result<(), Error<..>>`, discarding the body, so
/// `GET /v1/billing/balance` and friends cannot return what they actually send.
/// (The Go generator hands back the raw response instead, which is why the Go
/// examples decode rather than needing this.)
///
/// It uses the same `Configuration` — same base URL, same bearer token — as
/// every generated call. When those operations regain response schemas this
/// collapses into a plain typed call and this helper goes away.
pub async fn get_json(
    cfg: &Configuration,
    path: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut req = cfg.client.get(format!("{}{path}", cfg.base_path));
    if let Some(token) = &cfg.bearer_access_token {
        req = req.bearer_auth(token);
    }
    Ok(req.send().await?.error_for_status()?.json().await?)
}

# hanzo-client

The Rust client for the [Hanzo API](https://api.hanzo.ai). Generated from
[`hanzoai/openapi`](https://github.com/hanzoai/openapi) `hanzo.yaml` — the one
document that defines every Hanzo service — so this crate and the Go,
TypeScript, and Python SDKs all describe the same product.

2452 operations across 265 API modules, one crate.

```toml
[dependencies]
hanzo-client = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Use

Every request carries a bearer token. `Configuration::default()` already points
at `https://api.hanzo.ai`; set the token and go.

```rust
use hanzo_client::apis::{auth_api, configuration::Configuration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Configuration {
        bearer_access_token: std::env::var("HANZO_API_KEY").ok(),
        ..Default::default()
    };

    let me = auth_api::bot_whoami(&cfg).await?;
    println!("{}", me.handle.unwrap_or_default());
    Ok(())
}
```

Org-scoped services (kv, agents) also need an `X-Org-Id` header — build
`cfg.client` with it as a default header.

## Flows

Six runnable journeys, the same six in every Hanzo SDK, live in this
repository's [`examples/`](../../examples):

```bash
HANZO_API_KEY=sk-... cargo run -p hanzo-examples --example hello
```

`hello`, `chat`, `money`, `store`, `agent`, `tools`.

## Do not edit

`src/` is generated. Edit the per-service spec in `hanzoai/openapi` and run
`./scripts/generate.sh` from the repository root.

## License

MIT OR Apache-2.0.

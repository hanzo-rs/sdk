//! Hanzo MCP Core - Shared traits and types for Model Context Protocol
//!
//! This crate provides the foundational abstractions used by:
//! - `hanzo-mcp-client` - Client for connecting to MCP servers
//!
//! The server side is not here. `hanzo-mcp` on crates.io is published from
//! github.com/hanzoai/mcp (its `rust/` directory) and ships the tool server
//! binary. This repo is client-side only.
//!
//! # Overview
//!
//! The Model Context Protocol (MCP) enables AI models to interact with external
//! tools and resources. This crate defines the core interfaces that both clients
//! and servers implement.
//!
//! # Configuration
//!
//! Use [`McpClientConfig`] to configure multi-server MCP client connections:
//!
//! ```rust
//! use hanzo_mcp_core::{McpClientConfig, McpServerConfig, McpServerSource};
//!
//! let config = McpClientConfig {
//!     servers: vec![
//!         McpServerConfig {
//!             name: "local-tools".to_string(),
//!             source: McpServerSource::Process {
//!                 command: "mcp-server".to_string(),
//!                 args: vec![],
//!                 work_dir: None,
//!                 env: None,
//!             },
//!             ..Default::default()
//!         },
//!     ],
//!     max_concurrent_calls: Some(5),
//!     ..Default::default()
//! };
//! ```

mod config;
mod error;
mod traits;
mod types;

pub use config::*;
pub use error::*;
pub use traits::*;
pub use types::*;

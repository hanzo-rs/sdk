//! Tool registry and execution for agents
//!
//! This module provides a unified tool registry that can integrate:
//! - Built-in tools (file operations, search, etc.)
//! - MCP server tools (via hanzo-mcp)
//! - Custom tool callbacks

use crate::traits::{AgentError, Result, ToolDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Callback type for tool execution
pub type ToolCallback = Arc<
    dyn Fn(serde_json::Value) -> futures::future::BoxFuture<'static, Result<String>> + Send + Sync,
>;

/// A registered tool with its callback
pub struct RegisteredTool {
    pub definition: ToolDefinition,
    pub callback: ToolCallback,
    pub source: ToolSource,
}

/// Source of a tool (for debugging and management)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSource {
    /// Built-in tool
    BuiltIn,
    /// Tool from an MCP server
    Mcp { server_name: String },
    /// Custom tool registered at runtime
    Custom { source: String },
}

/// Configuration for an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name (unique identifier)
    pub name: String,
    /// Connection type
    pub connection: McpConnection,
    /// Environment variables for process-based servers
    pub env: Option<HashMap<String, String>>,
    /// Timeout in seconds
    pub timeout_secs: Option<u64>,
}

/// MCP connection types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpConnection {
    /// HTTP-based connection
    #[serde(rename = "http")]
    Http { url: String },
    /// WebSocket connection
    #[serde(rename = "websocket")]
    WebSocket { url: String },
    /// Subprocess (stdio)
    #[serde(rename = "process")]
    Process {
        command: String,
        args: Option<Vec<String>>,
    },
}

/// Central registry for all tools available to agents
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, RegisteredTool>>,
    mcp_servers: RwLock<HashMap<String, McpServerConfig>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            mcp_servers: RwLock::new(HashMap::new()),
        }
    }

    /// Create a registry with default built-in tools
    pub fn with_defaults() -> Self {
        let registry = Self::new();
        // Built-in tools would be registered here
        // For now, keeping it empty as tools will come from MCP servers
        registry
    }

    /// Register a custom tool
    pub async fn register(
        &self,
        definition: ToolDefinition,
        callback: ToolCallback,
        source: ToolSource,
    ) {
        let name = definition.name.clone();
        let tool = RegisteredTool {
            definition,
            callback,
            source,
        };
        self.tools.write().await.insert(name, tool);
    }

    /// Register an MCP server and discover its tools
    pub async fn register_mcp_server(&self, config: McpServerConfig) -> Result<Vec<String>> {
        let server_name = config.name.clone();

        // Store the config
        self.mcp_servers
            .write()
            .await
            .insert(server_name.clone(), config.clone());

        // Discover and register tools from the server
        let tool_names = self.discover_mcp_tools(&config).await?;

        Ok(tool_names)
    }

    /// Discover tools from an MCP server
    async fn discover_mcp_tools(&self, config: &McpServerConfig) -> Result<Vec<String>> {
        use hanzo_mcp_client::mcp_methods;

        let tools = match &config.connection {
            McpConnection::Http { url } => mcp_methods::list_tools_via_http(url, None)
                .await
                .map_err(|e| AgentError::McpError(e.message))?,
            McpConnection::Process { command, args } => {
                let cmd_str = if let Some(args) = args {
                    format!("{} {}", command, args.join(" "))
                } else {
                    command.clone()
                };
                mcp_methods::list_tools_via_command(&cmd_str, config.env.clone())
                    .await
                    .map_err(|e| AgentError::McpError(e.message))?
            }
            McpConnection::WebSocket { url: _ } => {
                // WebSocket support would go here
                return Err(AgentError::ConfigError(
                    "WebSocket MCP not yet implemented".to_string(),
                ));
            }
        };

        let mut registered_names = Vec::new();
        let server_name = config.name.clone();

        for tool in tools {
            // Prefix tool name with server name to avoid conflicts
            let prefixed_name = format!("{}:{}", server_name, tool.name);

            let definition = ToolDefinition {
                name: prefixed_name.clone(),
                description: tool.description.unwrap_or_default().to_string(),
                parameters: serde_json::Value::Object((*tool.input_schema).clone()),
                requires_confirmation: false,
            };

            // Create callback that routes to the MCP server
            let config_clone = config.clone();
            let tool_name = tool.name.clone();

            let callback: ToolCallback = Arc::new(move |args: serde_json::Value| {
                let config = config_clone.clone();
                let name = tool_name.clone();
                Box::pin(async move {
                    let params = args.as_object().cloned().unwrap_or_default();
                    execute_mcp_tool(&config, &name, params).await
                })
            });

            self.register(
                definition,
                callback,
                ToolSource::Mcp {
                    server_name: server_name.clone(),
                },
            )
            .await;

            registered_names.push(prefixed_name);
        }

        Ok(registered_names)
    }

    /// Get a tool by name
    pub async fn get(&self, name: &str) -> Option<ToolDefinition> {
        self.tools
            .read()
            .await
            .get(name)
            .map(|t| t.definition.clone())
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, args: serde_json::Value) -> Result<String> {
        let tools = self.tools.read().await;
        let tool = tools.get(name).ok_or_else(|| AgentError::ToolError {
            tool_name: name.to_string(),
            message: "Tool not found".to_string(),
        })?;

        (tool.callback)(args).await
    }

    /// List all available tools
    pub async fn list(&self) -> Vec<ToolDefinition> {
        self.tools
            .read()
            .await
            .values()
            .map(|t| t.definition.clone())
            .collect()
    }

    /// List tools by source
    pub async fn list_by_source(&self, source_filter: &str) -> Vec<ToolDefinition> {
        self.tools
            .read()
            .await
            .values()
            .filter(|t| match &t.source {
                ToolSource::BuiltIn => source_filter == "builtin",
                ToolSource::Mcp { server_name } => server_name == source_filter,
                ToolSource::Custom { source } => source == source_filter,
            })
            .map(|t| t.definition.clone())
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute a tool on an MCP server
async fn execute_mcp_tool(
    config: &McpServerConfig,
    tool_name: &str,
    params: serde_json::Map<String, serde_json::Value>,
) -> Result<String> {
    use hanzo_mcp_client::mcp_methods;

    let result = match &config.connection {
        McpConnection::Http { url } => {
            mcp_methods::run_tool_via_http(url.clone(), tool_name.to_string(), params)
                .await
                .map_err(|e| AgentError::McpError(e.message))?
        }
        McpConnection::Process { command, args } => {
            let cmd_str = if let Some(args) = args {
                format!("{} {}", command, args.join(" "))
            } else {
                command.clone()
            };
            mcp_methods::run_tool_via_command(
                cmd_str,
                tool_name.to_string(),
                config.env.clone().unwrap_or_default(),
                params,
            )
            .await
            .map_err(|e| AgentError::McpError(e.message))?
        }
        McpConnection::WebSocket { url: _ } => {
            return Err(AgentError::ConfigError(
                "WebSocket MCP not yet implemented".to_string(),
            ));
        }
    };

    // Extract text content from the result
    let text = result
        .content
        .into_iter()
        .filter_map(|c| c.as_text().map(|t| t.text.clone()))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = ToolRegistry::new();
        let tools = registry.list().await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn test_custom_tool_registration() {
        let registry = ToolRegistry::new();

        let definition = ToolDefinition::new("test_tool", "A test tool");
        let callback: ToolCallback =
            Arc::new(|_args| Box::pin(async { Ok("test result".to_string()) }));

        registry
            .register(
                definition,
                callback,
                ToolSource::Custom {
                    source: "test".to_string(),
                },
            )
            .await;

        let tools = registry.list().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");

        let result = registry
            .execute("test_tool", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result, "test result");
    }
}

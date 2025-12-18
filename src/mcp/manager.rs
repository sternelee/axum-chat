use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;

use super::client::{
    create_mcp_client, CallToolParams, CallToolResult, McpClientError, McpClientTrait,
    McpConnectionInfo, ServerCapabilities, Tool,
};
use super::config::{McpConfig, McpServerConfig};

#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub server_name: String,
    pub tool_info: Tool,
}

pub struct McpManager {
    clients: Arc<RwLock<HashMap<String, Arc<Box<dyn McpClientTrait>>>>>,
    tools: Arc<RwLock<HashMap<String, McpTool>>>,
    config: Arc<RwLock<McpConfig>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(McpConfig::new())),
        }
    }

    pub async fn load_config(
        &self,
        config_path: &std::path::PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = McpConfig::load_from_file(config_path)?;
        *self.config.write().await = config;
        Ok(())
    }

    pub async fn save_config(
        &self,
        config_path: &std::path::PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        config.save_to_file(config_path)?;
        Ok(())
    }

    pub async fn add_server_config(&self, name: String, server_config: McpServerConfig) {
        let mut config = self.config.write().await;
        config.add_server(name, server_config);
    }

    pub async fn remove_server_config(&self, name: &str) -> Option<McpServerConfig> {
        let mut config = self.config.write().await;
        config.remove_server(name)
    }

    pub async fn initialize_servers(&self) -> Result<usize, McpManagerError> {
        let config = self.config.read().await;
        let enabled_servers = config.get_enabled_servers();

        let mut initialized_count = 0;

        for (name, server_config) in enabled_servers {
            match self.initialize_server(name.clone(), server_config).await {
                Ok(_) => {
                    initialized_count += 1;
                    println!("Successfully initialized MCP server: {}", name);
                }
                Err(e) => {
                    eprintln!("Failed to initialize MCP server {}: {}", name, e);
                }
            }
        }

        Ok(initialized_count)
    }

    pub async fn initialize_server(
        &self,
        name: String,
        server_config: &McpServerConfig,
    ) -> Result<(), McpManagerError> {
        // Remove existing client if it exists
        self.shutdown_server(&name).await.ok();

        // Create new client
        let mut client = create_mcp_client(name.clone(), server_config)
            .await
            .map_err(|e| McpManagerError::Initialization(name.clone(), e))?;

        // Initialize the client
        client
            .initialize()
            .await
            .map_err(|e| McpManagerError::Initialization(name.clone(), e))?;

        // List tools from this server
        let tools_result = client
            .list_tools()
            .await
            .map_err(|e| McpManagerError::ToolDiscovery(name.clone(), e))?;

        // Add client to manager
        {
            let mut clients = self.clients.write().await;
            clients.insert(name.clone(), Arc::new(client));
        }

        // Add tools to manager with server prefix
        {
            let mut tools = self.tools.write().await;
            for tool in tools_result.tools {
                let prefixed_name = format!("{}__{}", name, tool.name);
                let mcp_tool = McpTool {
                    name: prefixed_name.clone(),
                    description: tool
                        .description
                        .as_ref()
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "No description".to_string()),
                    server_name: name.clone(),
                    tool_info: tool,
                };
                tools.insert(prefixed_name, mcp_tool);
            }
        }

        Ok(())
    }

    pub async fn shutdown_server(&self, name: &str) -> Result<(), McpManagerError> {
        let client = {
            let mut clients = self.clients.write().await;
            clients.remove(name)
        };

        if let Some(client) = client {
            // Remove tools associated with this server
            {
                let mut tools = self.tools.write().await;
                tools.retain(|_, tool| tool.server_name != name);
            }

            // Shutdown the client
            let mut client_mut = Arc::try_unwrap(client).map_err(|_| {
                McpManagerError::Shutdown(name.to_string(), "Client is still in use".to_string())
            })?;
            client_mut
                .shutdown()
                .await
                .map_err(|e| McpManagerError::Shutdown(name.to_string(), e.to_string()))?;
        }

        Ok(())
    }

    pub async fn shutdown_all(&self) {
        let clients: Vec<String> = {
            let clients_lock = self.clients.read().await;
            clients_lock.keys().cloned().collect()
        };

        for name in clients {
            if let Err(e) = self.shutdown_server(&name).await {
                eprintln!("Error shutting down server {}: {}", name, e);
            }
        }
    }

    pub async fn get_all_tools(&self) -> Vec<McpTool> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    pub async fn get_tool(&self, name: &str) -> Option<McpTool> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpManagerError> {
        let tool = self
            .get_tool(tool_name)
            .await
            .ok_or_else(|| McpManagerError::ToolNotFound(tool_name.to_string()))?;

        let client = {
            let clients = self.clients.read().await;
            clients
                .get(&tool.server_name)
                .cloned()
                .ok_or_else(|| McpManagerError::ServerNotFound(tool.server_name.clone()))?
        };

        let server_tool_name = tool.tool_info.name.clone();
        let call_params = CallToolParams {
            name: server_tool_name,
            arguments: Some(arguments),
            timeout: timeout_secs.map(|t| t as i32),
        };

        let timeout_duration = timeout_secs.unwrap_or(300).max(1); // Minimum 1 second timeout

        let result = timeout(
            Duration::from_secs(timeout_duration as u64),
            client.call_tool(call_params),
        )
        .await
        .map_err(|_| McpManagerError::Timeout(tool_name.to_string()))?;

        result.map_err(|e| McpManagerError::ToolExecution(tool_name.to_string(), e))
    }

    pub async fn list_resources_for_server(
        &self,
        server_name: &str,
    ) -> Result<Vec<serde_json::Value>, McpManagerError> {
        let client = {
            let clients = self.clients.read().await;
            clients
                .get(server_name)
                .cloned()
                .ok_or_else(|| McpManagerError::ServerNotFound(server_name.to_string()))?
        };

        let resources_result = client
            .list_resources()
            .await
            .map_err(|e| McpManagerError::ResourceDiscovery(server_name.to_string(), e))?;

        Ok(resources_result.resources)
    }

    pub async fn read_resource(
        &self,
        server_name: &str,
        uri: &str,
    ) -> Result<super::client::ReadResourceResult, McpManagerError> {
        let client = {
            let clients = self.clients.read().await;
            clients
                .get(server_name)
                .cloned()
                .ok_or_else(|| McpManagerError::ServerNotFound(server_name.to_string()))?
        };

        let read_params = super::client::ReadResourceParams {
            uri: uri.to_string(),
        };

        client
            .read_resource(read_params)
            .await
            .map_err(|e| McpManagerError::ResourceRead(server_name.to_string(), e))
    }

    pub async fn list_prompts_for_server(
        &self,
        server_name: &str,
    ) -> Result<Vec<serde_json::Value>, McpManagerError> {
        let client = {
            let clients = self.clients.read().await;
            clients
                .get(server_name)
                .cloned()
                .ok_or_else(|| McpManagerError::ServerNotFound(server_name.to_string()))?
        };

        let prompts_result = client
            .list_prompts()
            .await
            .map_err(|e| McpManagerError::PromptDiscovery(server_name.to_string(), e))?;

        Ok(prompts_result.prompts)
    }

    pub async fn get_prompt(
        &self,
        server_name: &str,
        prompt_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<super::client::GetPromptResult, McpManagerError> {
        let client = {
            let clients = self.clients.read().await;
            clients
                .get(server_name)
                .cloned()
                .ok_or_else(|| McpManagerError::ServerNotFound(server_name.to_string()))?
        };

        client
            .get_prompt(prompt_name, arguments)
            .await
            .map_err(|e| {
                McpManagerError::PromptGet(server_name.to_string(), prompt_name.to_string(), e)
            })
    }

    pub async fn get_server_configs(&self) -> HashMap<String, McpServerConfig> {
        let config = self.config.read().await;
        config.mcp_servers.clone()
    }

    pub async fn get_connected_servers(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients.keys().cloned().collect()
    }
}

impl Drop for McpManager {
    fn drop(&mut self) {
        // Note: This is a synchronous drop, but we want to shutdown servers asynchronously
        // In a real implementation, you might want to use a more sophisticated approach
        // with a shutdown signal or explicit shutdown method
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpManagerError {
    #[error("Failed to initialize server '{0}': {1}")]
    Initialization(String, McpClientError),

    #[error("Failed to discover tools from server '{0}': {1}")]
    ToolDiscovery(String, McpClientError),

    #[error("Failed to shutdown server '{0}': {1}")]
    Shutdown(String, String),

    #[error("Tool '{0}' not found")]
    ToolNotFound(String),

    #[error("Server '{0}' not found")]
    ServerNotFound(String),

    #[error("Tool execution failed for '{0}': {1}")]
    ToolExecution(String, McpClientError),

    #[error("Timeout while executing tool '{0}'")]
    Timeout(String),

    #[error("Failed to discover resources from server '{0}': {1}")]
    ResourceDiscovery(String, McpClientError),

    #[error("Failed to read resource from server '{0}': {1}")]
    ResourceRead(String, McpClientError),

    #[error("Failed to discover prompts from server '{0}': {1}")]
    PromptDiscovery(String, McpClientError),

    #[error("Failed to get prompt '{1}' from server '{0}': {2}")]
    PromptGet(String, String, McpClientError),
}

// Global instance for easy access
static MCP_MANAGER: std::sync::LazyLock<Arc<McpManager>> =
    std::sync::LazyLock::new(|| Arc::new(McpManager::new()));

pub fn get_mcp_manager() -> Arc<McpManager> {
    MCP_MANAGER.clone()
}


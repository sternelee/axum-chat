use crate::mcp::config::{McpServiceConfig, ServiceType};
use rmcp::{transport::stdio::StdioServerTransport, Client};
use serde_json::Value;
use std::process::{Command, Child, Stdio};
use std::time::{Duration, Instant};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tracing::{info, warn, error, debug};

#[derive(Debug, Clone)]
pub struct McpService {
    pub config: McpServiceConfig,
    pub status: ServiceStatus,
    pub child: Option<std::process::Child>,
    pub client: Option<Client<StdioServerTransport>>,
    pub started_at: Option<Instant>,
    pub restart_count: u32,
    pub last_error: Option<String>,
    pub tool_registry: ToolRegistry,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Error,
    Restarting,
}

#[derive(Debug, Clone, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolInfo>,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Option<Value>,
    pub category: String,
    pub requires_approval: bool,
    pub usage_count: u64,
    pub last_used: Option<Instant>,
}

impl McpService {
    pub fn new(config: McpServiceConfig) -> Self {
        Self {
            config,
            status: ServiceStatus::Stopped,
            child: None,
            client: None,
            started_at: None,
            restart_count: 0,
            last_error: None,
            tool_registry: ToolRegistry::new(),
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.status == ServiceStatus::Running {
            return Ok(());
        }

        self.status = ServiceStatus::Starting;
        info!("Starting MCP service: {}", self.config.id);

        match self.config.r#type {
            ServiceType::Stdio => self.start_stdio().await,
            ServiceType::Sse => self.start_sse().await,
            ServiceType::WebSocket => self.start_websocket().await,
        }
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping MCP service: {}", self.config.id);

        self.status = ServiceStatus::Stopped;

        if let Some(mut child) = self.child.take() {
            match child.kill() {
                Ok(_) => {
                    info!("Successfully stopped service: {}", self.config.id);
                }
                Err(e) => {
                    warn!("Failed to kill service {}: {}", self.config.id, e);
                }
            }
        }

        self.client = None;
        self.started_at = None;

        Ok(())
    }

    pub async fn restart(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Restarting MCP service: {}", self.config.id);

        self.stop().await?;

        if self.restart_count < self.config.max_restarts {
            self.restart_count += 1;
            tokio::time::sleep(Duration::from_millis(1000)).await;
            self.start().await
        } else {
            Err(format!("Service {} exceeded max restarts ({})",
                      self.config.id, self.config.max_restarts).into())
        }
    }

    pub async fn health_check(&self) -> ServiceHealth {
        match self.status {
            ServiceStatus::Running => {
                if let Some(client) = &self.client {
                    // Try to ping the service
                    match timeout(Duration::from_secs(5), self.ping_service()).await {
                        Ok(Ok(_)) => ServiceHealth::Healthy,
                        Ok(Err(e)) => ServiceHealth::Unhealthy(e.to_string()),
                        Err(_) => ServiceHealth::Unhealthy("Health check timeout".to_string()),
                    }
                } else {
                    ServiceHealth::Unhealthy("No client connection".to_string())
                }
            }
            ServiceStatus::Starting => ServiceHealth::Starting,
            ServiceStatus::Stopped => ServiceHealth::Stopped,
            ServiceStatus::Error => ServiceHealth::Error(self.last_error.clone().unwrap_or_default()),
            ServiceStatus::Restarting => ServiceHealth::Restarting,
        }
    }

    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        if self.status != ServiceStatus::Running {
            return Err(format!("Service {} is not running", self.config.id).into());
        }

        debug!("Calling tool {} on service {}", tool_name, self.config.id);

        if let Some(client) = &mut self.client {
            let start_time = Instant::now();

            let result = timeout(
                Duration::from_millis(self.config.timeout),
                self.execute_tool_call(client, tool_name, arguments)
            ).await;

            let execution_time = start_time.elapsed();

            // Update tool usage stats
            self.tool_registry.record_usage(tool_name, execution_time);

            match result {
                Ok(Ok(value)) => Ok(value),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(format!("Tool call timeout after {:?}", execution_time).into()),
            }
        } else {
            Err("No client available".into())
        }
    }

    pub async fn list_tools(&self) -> Vec<&ToolInfo> {
        self.tool_registry.list_tools()
    }

    pub fn get_tool(&self, tool_name: &str) -> Option<&ToolInfo> {
        self.tool_registry.get_tool(tool_name)
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.started_at.map(|start| start.elapsed())
    }

    // Private methods
    async fn start_stdio(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cmd = TokioCommand::new(&self.config.command);
        cmd.args(&self.config.args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Set environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;

        let transport = StdioServerTransport::new(stdin, stdout);
        let client = Client::new("rustgpt".to_string(), transport).await?;

        self.child = Some(child.into());
        self.client = Some(client);
        self.status = ServiceStatus::Running;
        self.started_at = Some(Instant::now());

        // Load available tools
        self.load_tools().await?;

        info!("Successfully started stdio service: {}", self.config.id);
        Ok(())
    }

    async fn start_sse(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement SSE service starting
        Err("SSE service type not yet implemented".into())
    }

    async fn start_websocket(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement WebSocket service starting
        Err("WebSocket service type not yet implemented".into())
    }

    async fn load_tools(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(client) = &mut self.client {
            // Request tools list from the MCP service
            let tools_response = client.request("tools/list", None).await?;

            if let Some(tools) = tools_response.get("tools").and_then(|t| t.as_array()) {
                for tool in tools {
                    if let Some(tool_obj) = tool.as_object() {
                        let name = tool_obj.get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let description = tool_obj.get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string();

                        let parameters = tool_obj.get("inputSchema").cloned();

                        let tool_info = ToolInfo {
                            name: name.clone(),
                            description,
                            parameters,
                            category: self.determine_tool_category(&name),
                            requires_approval: self.requires_tool_approval(&name),
                            usage_count: 0,
                            last_used: None,
                        };

                        self.tool_registry.register_tool(name, tool_info);
                    }
                }
            }
        }

        Ok(())
    }

    async fn ping_service(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(client) = &self.client {
            let response = client.request("ping", None).await?;
            if response.get("result").is_some() {
                Ok(())
            } else {
                Err("Ping failed".into())
            }
        } else {
            Err("No client available".into())
        }
    }

    async fn execute_tool_call(
        &self,
        client: &mut Client<StdioServerTransport>,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let params = Some(serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        }));

        let response = client.request("tools/call", params).await?;

        match response.get("result") {
            Some(result) => Ok(result.clone()),
            None => match response.get("error") {
                Some(error) => Err(format!("Tool call error: {}", error).into()),
                None => Err("Unknown tool call error".into()),
            },
        }
    }

    fn determine_tool_category(&self, tool_name: &str) -> String {
        match tool_name {
            name if name.contains("file") => "filesystem".to_string(),
            name if name.contains("dir") || name.contains("directory") => "filesystem".to_string(),
            name if name.contains("database") || name.contains("sql") => "database".to_string(),
            name if name.contains("search") => "search".to_string(),
            name if name.contains("web") || name.contains("http") => "web".to_string(),
            name if name.contains("github") => "version_control".to_string(),
            _ => "general".to_string(),
        }
    }

    fn requires_tool_approval(&self, tool_name: &str) -> bool {
        // Check if tool requires approval based on configuration
        // This can be extended with more sophisticated logic
        match tool_name {
            name if name.contains("delete") || name.contains("remove") => true,
            name if name.contains("write") || name.contains("create") => true,
            name if name.contains("execute") || name.contains("run") => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ServiceHealth {
    Healthy,
    Unhealthy(String),
    Starting,
    Stopped,
    Error(String),
    Restarting,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register_tool(&mut self, name: String, tool: ToolInfo) {
        self.tools.insert(name, tool);
    }

    pub fn get_tool(&self, name: &str) -> Option<&ToolInfo> {
        self.tools.get(name)
    }

    pub fn list_tools(&self) -> Vec<&ToolInfo> {
        self.tools.values().collect()
    }

    pub fn record_usage(&mut self, tool_name: &str, execution_time: Duration) {
        if let Some(tool) = self.tools.get_mut(tool_name) {
            tool.usage_count += 1;
            tool.last_used = Some(Instant::now());
            debug!("Tool {} used (count: {}, execution_time: {:?})",
                   tool_name, tool.usage_count, execution_time);
        }
    }

    pub fn get_usage_stats(&self) -> HashMap<String, (u64, Option<Instant>)> {
        self.tools.iter()
            .map(|(name, tool)| (name.clone(), (tool.usage_count, tool.last_used)))
            .collect()
    }
}

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command as TokioCommand;
use tracing::{info, warn, error};

/// Practical MCP Service Manager - simplified approach without rmcp complexity
#[derive(Debug)]
pub struct PracticalMcpService {
    pub config: PracticalMcpServiceConfig,
    pub status: ServiceStatus,
    pub process: Option<tokio::process::Child>,
    pub tools: HashMap<String, ToolInfo>,
    pub started_at: Option<Instant>,
    pub restart_count: u32,
}

impl Clone for PracticalMcpService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            status: self.status.clone(),
            process: None, // Don't clone the running process
            tools: self.tools.clone(),
            started_at: self.started_at,
            restart_count: self.restart_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticalMcpServiceConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default = "HashMap::new")]
    pub env: HashMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    #[serde(default = "default_auto_restart")]
    pub auto_restart: bool,
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Error,
    Restarting,
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
    pub auto_approved: bool,
}

// Serialization helper for ToolInfo
#[derive(Serialize, Deserialize)]
struct ToolInfoSerializable {
    pub name: String,
    pub description: String,
    pub parameters: Option<Value>,
    pub category: String,
    pub requires_approval: bool,
    pub usage_count: u64,
    pub auto_approved: bool,
}

impl From<&ToolInfo> for ToolInfoSerializable {
    fn from(tool_info: &ToolInfo) -> Self {
        Self {
            name: tool_info.name.clone(),
            description: tool_info.description.clone(),
            parameters: tool_info.parameters.clone(),
            category: tool_info.category.clone(),
            requires_approval: tool_info.requires_approval,
            usage_count: tool_info.usage_count,
            auto_approved: tool_info.auto_approved,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PracticalMcpManager {
    services: HashMap<String, PracticalMcpService>,
    config_path: String,
    tool_registry: HashMap<String, RegisteredTool>,
}

#[derive(Debug, Clone)]
pub struct RegisteredTool {
    pub service_id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub requires_approval: bool,
    pub auto_approved: bool,
    pub usage_count: u64,
    pub last_used: Option<Instant>,
}

impl Serialize for RegisteredTool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("RegisteredTool", 8)?;
        state.serialize_field("service_id", &self.service_id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("description", &self.description)?;
        state.serialize_field("category", &self.category)?;
        state.serialize_field("requires_approval", &self.requires_approval)?;
        state.serialize_field("auto_approved", &self.auto_approved)?;
        state.serialize_field("usage_count", &self.usage_count)?;
        // Skip last_used as Instant doesn't serialize well
        state.serialize_field("last_used", &None::<String>)?;
        state.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServiceStatus {
    pub id: String,
    pub name: String,
    pub status: ServiceStatus,
    pub uptime: Option<u64>,
    pub restart_count: u32,
    pub tool_count: usize,
    pub last_error: Option<String>,
}

impl Default for PracticalMcpServiceConfig {
    fn default() -> Self {
        Self {
            id: "unknown".to_string(),
            name: "Unknown Service".to_string(),
            description: "No description".to_string(),
            enabled: false,
            command: "".to_string(),
            args: Vec::new(),
            env: HashMap::new(),
            timeout: default_timeout(),
            max_restarts: default_max_restarts(),
            auto_restart: default_auto_restart(),
            tools: Vec::new(),
        }
    }
}

// Helper functions for defaults
fn default_timeout() -> u64 { 30000 }
fn default_max_restarts() -> u32 { 3 }
fn default_auto_restart() -> bool { true }

impl PracticalMcpService {
    pub fn new(config: PracticalMcpServiceConfig) -> Self {
        Self {
            config,
            status: ServiceStatus::Stopped,
            process: None,
            tools: HashMap::new(),
            started_at: None,
            restart_count: 0,
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.status == ServiceStatus::Running {
            return Ok(());
        }

        self.status = ServiceStatus::Starting;
        info!("Starting MCP service: {}", self.config.id);

        // Build command
        let mut cmd = TokioCommand::new(&self.config.command);
        cmd.args(&self.config.args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Set environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // Start the process
        let child = cmd.spawn()
            .map_err(|e| format!("Failed to start MCP service {}: {}", self.config.id, e))?;

        self.process = Some(child);
        self.status = ServiceStatus::Running;
        self.started_at = Some(Instant::now());

        // Load tools (mock implementation for now)
        self.load_tools().await?;

        info!("Successfully started service: {}", self.config.id);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Stopping MCP service: {}", self.config.id);

        self.status = ServiceStatus::Stopped;

        if let Some(mut child) = self.process.take() {
            match child.kill().await {
                Ok(_) => {
                    info!("Successfully stopped service: {}", self.config.id);
                }
                Err(e) => {
                    warn!("Failed to kill service {}: {}", self.config.id, e);
                }
            }
        }

        self.started_at = None;

        Ok(())
    }

    pub async fn restart(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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

    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        if self.status != ServiceStatus::Running {
            return Err(format!("Service {} is not running", self.config.id).into());
        }

        info!("Calling tool {} on service {}", tool_name, self.config.id);

        // Update usage stats
        if let Some(tool) = self.tools.get_mut(tool_name) {
            tool.usage_count += 1;
            tool.last_used = Some(Instant::now());
        }

        // For now, return a mock response
        // TODO: Implement actual MCP protocol communication via stdin/stdout
        let mock_response = serde_json::json!({
            "result": format!("Mock execution of tool {} with args: {:?}", tool_name, arguments),
            "service_id": self.config.id,
            "tool_name": tool_name,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "status": "success"
        });

        Ok(mock_response)
    }

    pub async fn list_tools(&self) -> Vec<&ToolInfo> {
        self.tools.values().collect()
    }

    pub fn get_tool(&self, tool_name: &str) -> Option<&ToolInfo> {
        self.tools.get(tool_name)
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.started_at.map(|start| start.elapsed())
    }

    pub fn get_usage_stats(&self) -> HashMap<String, (u64, Option<Instant>)> {
        self.tools.iter()
            .map(|(name, tool)| (name.clone(), (tool.usage_count, tool.last_used)))
            .collect()
    }

    async fn load_tools(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Create mock tool information based on the tools list in config
        for tool_name in &self.config.tools {
            let tool_info = ToolInfo {
                name: tool_name.clone(),
                description: format!("Implementation of {}", tool_name),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                })),
                category: self.determine_tool_category(tool_name),
                requires_approval: self.requires_tool_approval(tool_name),
                usage_count: 0,
                last_used: None,
                auto_approved: false, // Will be set based on configuration
            };

            self.tools.insert(tool_name.clone(), tool_info);
        }

        Ok(())
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
        match tool_name {
            name if name.contains("delete") || name.contains("remove") => true,
            name if name.contains("write") || name.contains("create") => true,
            name if name.contains("execute") || name.contains("run") => true,
            _ => false,
        }
    }
}

impl PracticalMcpManager {
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            services: HashMap::new(),
            config_path: config_path.to_string(),
            tool_registry: HashMap::new(),
        })
    }

    /// Test function to verify configuration loading
    pub async fn test_config_loading(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let configs = self.load_config().await?;
        println!("Successfully loaded {} MCP service configurations", configs.len());

        for config in &configs {
            println!("  - {} ({}): enabled={}, tools={}",
                    config.id, config.name, config.enabled, config.tools.len());
        }

        Ok(configs.len())
    }

    pub async fn load_config(&self) -> Result<Vec<PracticalMcpServiceConfig>, Box<dyn std::error::Error>> {
        let config_content = tokio::fs::read_to_string(&self.config_path).await?;
        let full_config: serde_json::Value = serde_json::from_str(&config_content)?;

        // Extract services from the full config
        let services = if let Some(services) = full_config.get("services").and_then(|s| s.as_array()) {
            services.iter().enumerate().map(|(i, service)| {
                let id = service.get("id").and_then(|s| s.as_str()).unwrap_or(&format!("service_{}", i)).to_string();
                let name = service.get("name").and_then(|s| s.as_str()).unwrap_or(&id).to_string();
                let description = service.get("description").and_then(|s| s.as_str()).unwrap_or("").to_string();
                let enabled = service.get("enabled").and_then(|s| s.as_bool()).unwrap_or(false);
                let command = service.get("command").and_then(|s| s.as_str()).unwrap_or("echo").to_string();
                let args = service.get("args").and_then(|a| a.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                }).unwrap_or_else(|| vec!["mock".to_string()]);
                let tools = service.get("tools").and_then(|t| t.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                }).unwrap_or_else(|| vec!["mock_tool".to_string()]);

                PracticalMcpServiceConfig {
                    id,
                    name,
                    description,
                    enabled,
                    command,
                    args,
                    env: HashMap::new(),
                    timeout: service.get("timeout").and_then(|t| t.as_u64()).unwrap_or(30000),
                    max_restarts: service.get("max_restarts").and_then(|r| r.as_u64()).unwrap_or(3) as u32,
                    auto_restart: service.get("auto_restart").and_then(|r| r.as_bool()).unwrap_or(true),
                    tools,
                }
            }).collect()
        } else {
            Vec::new()
        };

        Ok(services)
    }

    pub async fn start_enabled_services(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let service_configs = self.load_config().await?;

        for service_config in service_configs {
            if service_config.enabled {
                info!("Starting MCP service: {}", service_config.id);

                let service_id = service_config.id.clone();
                let mut service = PracticalMcpService::new(service_config);
                match service.start().await {
                    Ok(_) => {
                        // Register tools from this service
                        self.register_tools_from_service(&service).await;
                        self.services.insert(service_id.clone(), service);
                        info!("Successfully started service: {}", service_id);
                    }
                    Err(e) => {
                        error!("Failed to start service {}: {}", service_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn call_tool(
        &mut self,
        service_id: &str,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        if let Some(service) = self.services.get_mut(service_id) {
            service.call_tool(tool_name, arguments).await
        } else {
            Err(format!("Service {} not found", service_id).into())
        }
    }

    pub async fn list_tools(&self, service_id: Option<&str>) -> Vec<(&String, &ToolInfo)> {
        let mut all_tools = Vec::new();

        if let Some(service_id) = service_id {
            if let Some(service) = self.services.get(service_id) {
                for tool in service.tools.values() {
                    all_tools.push((&service.config.id, tool));
                }
            }
        } else {
            for (id, service) in &self.services {
                for tool in service.tools.values() {
                    all_tools.push((id, tool));
                }
            }
        }

        all_tools
    }

    pub fn get_service_status(&self, service_id: &str) -> Option<McpServiceStatus> {
        self.services.get(service_id).map(|service| McpServiceStatus {
            id: service.config.id.clone(),
            name: service.config.name.clone(),
            status: service.status.clone(),
            uptime: service.uptime().map(|d| d.as_secs()),
            restart_count: service.restart_count,
            tool_count: service.tools.len(),
            last_error: None, // TODO: Track last error
        })
    }

    pub async fn list_services(&self) -> Vec<McpServiceStatus> {
        self.services.iter().map(|(id, service)| McpServiceStatus {
            id: id.clone(),
            name: service.config.name.clone(),
            status: service.status.clone(),
            uptime: service.uptime().map(|d| d.as_secs()),
            restart_count: service.restart_count,
            tool_count: service.tools.len(),
            last_error: None,
        }).collect()
    }

    pub async fn approve_tool(&mut self, service_id: &str, tool_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(service) = self.services.get_mut(service_id) {
            if let Some(tool) = service.tools.get_mut(tool_name) {
                tool.auto_approved = true;
                info!("Approved auto-approval for tool {}::{}", service_id, tool_name);
                return Ok(());
            }
        }
        Err(format!("Tool {}::{} not found", service_id, tool_name).into())
    }

    pub async fn revoke_tool_approval(&mut self, service_id: &str, tool_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(service) = self.services.get_mut(service_id) {
            if let Some(tool) = service.tools.get_mut(tool_name) {
                tool.auto_approved = false;
                info!("Revoked auto-approval for tool {}::{}", service_id, tool_name);
                return Ok(());
            }
        }
        Err(format!("Tool {}::{} not found", service_id, tool_name).into())
    }

    pub async fn get_rustgpt_tools(&self) -> Vec<RegisteredTool> {
        let mut tools = Vec::new();

        for (service_id, service) in &self.services {
            for tool in service.tools.values() {
                tools.push(RegisteredTool {
                    service_id: service_id.clone(),
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    category: tool.category.clone(),
                    requires_approval: tool.requires_approval && !tool.auto_approved,
                    auto_approved: tool.auto_approved,
                    usage_count: tool.usage_count,
                    last_used: tool.last_used,
                });
            }
        }

        tools
    }

    // Private methods
    async fn register_tools_from_service(&mut self, service: &PracticalMcpService) {
        for tool in service.tools.values() {
            let registered_tool = RegisteredTool {
                service_id: service.config.id.clone(),
                name: tool.name.clone(),
                description: tool.description.clone(),
                category: tool.category.clone(),
                requires_approval: tool.requires_approval && !tool.auto_approved,
                auto_approved: tool.auto_approved,
                usage_count: tool.usage_count,
                last_used: tool.last_used,
            };
            self.tool_registry.insert(
                format!("{}::{}", service.config.id, tool.name),
                registered_tool
            );
        }
    }
}
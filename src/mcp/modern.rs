use rmcp::{
    model::{CallToolRequestParam, ListToolsRequest},
    transport::{TokioChildProcess, ConfigureCommandExt},
};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{info, warn, error, debug};

/// Modern MCP Service using rmcp 0.9 API
#[derive(Debug)]
pub struct ModernMcpService {
    pub config: ModernMcpServiceConfig,
    pub service: Option<rmcp::client::Client<TokioChildProcess>>,
    pub status: ServiceStatus,
    pub tools: HashMap<String, ToolInfo>,
    pub started_at: Option<Instant>,
    pub restart_count: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModernMcpServiceConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub timeout: Duration,
    pub max_restarts: u32,
    pub auto_restart: bool,
}

#[derive(Debug, Clone, PartialEq)]
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
}

#[derive(Debug)]
pub struct ModernMcpManager {
    services: HashMap<String, ModernMcpService>,
    config_path: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ModernMcpConfig {
    pub services: Vec<ModernMcpServiceConfig>,
    pub global_settings: ModernGlobalSettings,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ModernGlobalSettings {
    pub max_concurrent_services: usize,
    pub default_timeout: u64,
    pub auto_start_enabled_services: bool,
    pub health_check_interval: u64,
}

impl ModernMcpService {
    pub fn new(config: ModernMcpServiceConfig) -> Self {
        Self {
            config,
            service: None,
            status: ServiceStatus::Stopped,
            tools: HashMap::new(),
            started_at: None,
            restart_count: 0,
            last_error: None,
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.status == ServiceStatus::Running {
            return Ok(());
        }

        self.status = ServiceStatus::Starting;
        info!("Starting MCP service: {}", self.config.id);

        // Build command with arguments
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args);

        // Set environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // Configure command using rmcp's ConfigureCommandExt
        cmd.configure(|cmd| {
            // The command is already configured with args and env
        });

        // Create transport
        let transport = TokioChildProcess::new(cmd)?;

        // Create client service
        let service = ()
            .serve(transport)
            .await
            .map_err(|e| format!("Failed to start MCP service {}: {}", self.config.id, e))?;

        // Get server information
        let server_info = service.peer_info();
        info!("Connected to MCP server {}: {:?}", self.config.id, server_info);

        // Load available tools
        self.load_tools(&service).await?;

        self.service = Some(service);
        self.status = ServiceStatus::Running;
        self.started_at = Some(Instant::now());

        info!("Successfully started service: {}", self.config.id);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping MCP service: {}", self.config.id);

        self.status = ServiceStatus::Stopped;

        if let Some(service) = self.service.take() {
            // Gracefully close the connection
            if let Err(e) = service.cancel().await {
                warn!("Error stopping service {}: {}", self.config.id, e);
            }
        }

        self.started_at = None;

        Ok(())
    }

    pub async fn restart(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Restarting MCP service: {}", self.config.id);

        if self.restart_count >= self.config.max_restarts {
            return Err(format!("Service {} exceeded max restarts ({})",
                              self.config.id, self.config.max_restarts).into());
        }

        self.stop().await?;
        self.restart_count += 1;

        // Wait a moment before restarting
        tokio::time::sleep(Duration::from_millis(1000)).await;

        self.start().await
    }

    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        if self.status != ServiceStatus::Running {
            return Err(format!("Service {} is not running", self.config.id).into());
        }

        if let Some(service) = &mut self.service {
            let start_time = Instant::now();

            info!("Calling tool {} on service {}", tool_name, self.config.id);

            // Prepare the tool call parameters
            let tool_param = CallToolRequestParam {
                name: tool_name.to_string(),
                arguments,
            };

            // Call the tool with timeout
            let result = tokio::time::timeout(
                self.config.timeout,
                service.call_tool(tool_param)
            ).await;

            let execution_time = start_time.elapsed();

            // Update usage stats
            self.update_tool_usage(tool_name, execution_time);

            match result {
                Ok(Ok(response)) => {
                    debug!("Tool {} executed successfully in {:?}", tool_name, execution_time);
                    Ok(response.content)
                }
                Ok(Err(e)) => {
                    error!("Tool call failed: {}", e);
                    Err(e.into())
                }
                Err(_) => {
                    error!("Tool call timeout after {:?}", execution_time);
                    Err(format!("Tool call timeout after {:?}", execution_time).into())
                }
            }
        } else {
            Err("No service available".into())
        }
    }

    pub async fn list_tools(&mut self) -> Result<Vec<&ToolInfo>, Box<dyn std::error::Error + Send + Sync>> {
        if self.status != ServiceStatus::Running {
            return Ok(Vec::new());
        }

        if let Some(service) = &mut self.service {
            let tools_response = service.list_tools(ListToolsRequestParam::default()).await?;

            // Clear existing tools and reload
            self.tools.clear();

            if let Some(tools) = tools_response.tools {
                for tool in tools {
                    let tool_info = ToolInfo {
                        name: tool.name.clone(),
                        description: tool.description.unwrap_or_default(),
                        parameters: tool.input_schema,
                        category: self.determine_tool_category(&tool.name),
                        requires_approval: self.requires_tool_approval(&tool.name),
                        usage_count: self.tools.get(&tool.name).map(|t| t.usage_count).unwrap_or(0),
                        last_used: self.tools.get(&tool.name).and_then(|t| t.last_used),
                    };
                    self.tools.insert(tool.name.clone(), tool_info);
                }
            }

            Ok(self.tools.values().collect())
        } else {
            Ok(Vec::new())
        }
    }

    pub fn get_tool(&self, tool_name: &str) -> Option<&ToolInfo> {
        self.tools.get(tool_name)
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.started_at.map(|start| start.elapsed())
    }

    pub fn get_tool_usage_stats(&self) -> HashMap<String, (u64, Option<Instant>)> {
        self.tools.iter()
            .map(|(name, tool)| (name.clone(), (tool.usage_count, tool.last_used)))
            .collect()
    }

    // Private methods
    async fn load_tools(&mut self, service: &rmcp::client::Client<TokioChildProcess>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tools_response = service.list_tools(ListToolsRequestParam::default()).await?;

        if let Some(tools) = tools_response.tools {
            for tool in tools {
                let tool_info = ToolInfo {
                    name: tool.name.clone(),
                    description: tool.description.unwrap_or_default(),
                    parameters: tool.input_schema,
                    category: self.determine_tool_category(&tool.name),
                    requires_approval: self.requires_tool_approval(&tool.name),
                    usage_count: 0,
                    last_used: None,
                };
                self.tools.insert(tool.name.clone(), tool_info);
            }
        }

        info!("Loaded {} tools for service {}", self.tools.len(), self.config.id);
        Ok(())
    }

    fn update_tool_usage(&mut self, tool_name: &str, execution_time: Duration) {
        if let Some(tool) = self.tools.get_mut(tool_name) {
            tool.usage_count += 1;
            tool.last_used = Some(Instant::now());
            debug!("Tool {} used (count: {}, execution_time: {:?})",
                   tool_name, tool.usage_count, execution_time);
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
        match tool_name {
            name if name.contains("delete") || name.contains("remove") => true,
            name if name.contains("write") || name.contains("create") => true,
            name if name.contains("execute") || name.contains("run") => true,
            _ => false,
        }
    }
}

impl ModernMcpManager {
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            services: HashMap::new(),
            config_path: config_path.to_string(),
        })
    }

    pub async fn load_config(&self) -> Result<ModernMcpConfig, Box<dyn std::error::Error>> {
        let config_content = tokio::fs::read_to_string(&self.config_path).await?;
        let full_config: serde_json::Value = serde_json::from_str(&config_content)?;

        // Extract services from the full config
        let services = if let Some(services) = full_config.get("services").and_then(|s| s.as_array()) {
            services.iter().map(|service| {
                let id = service.get("id").and_then(|s| s.as_str()).unwrap_or("unknown").to_string();
                let name = service.get("name").and_then(|s| s.as_str()).unwrap_or(&id).to_string();
                let description = service.get("description").and_then(|s| s.as_str()).unwrap_or("").to_string();
                let enabled = service.get("enabled").and_then(|s| s.as_bool()).unwrap_or(false);
                let command = service.get("command").and_then(|s| s.as_str()).unwrap_or("").to_string();
                let args = service.get("args").and_then(|a| a.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                }).unwrap_or_default();
                let timeout_ms = service.get("timeout").and_then(|t| t.as_u64()).unwrap_or(30000);
                let max_restarts = service.get("max_restarts").and_then(|r| r.as_u32()).unwrap_or(3);
                let auto_restart = service.get("auto_restart").and_then(|r| r.as_bool()).unwrap_or(true);

                // Extract environment variables
                let env = if let Some(env_map) = service.get("env").and_then(|e| e.as_object()) {
                    env_map.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                } else {
                    HashMap::new()
                };

                ModernMcpServiceConfig {
                    id,
                    name,
                    description,
                    enabled,
                    command,
                    args,
                    env,
                    timeout: Duration::from_millis(timeout_ms),
                    max_restarts,
                    auto_restart,
                }
            }).collect()
        } else {
            Vec::new()
        };

        let global_settings = full_config.get("global_settings").map(|gs| {
            ModernGlobalSettings {
                max_concurrent_services: gs.get("max_concurrent_services").and_then(|s| s.as_usize()).unwrap_or(10),
                default_timeout: gs.get("default_timeout").and_then(|s| s.as_u64()).unwrap_or(30000),
                auto_start_enabled_services: gs.get("auto_start_enabled_services").and_then(|s| s.as_bool()).unwrap_or(true),
                health_check_interval: gs.get("health_check_interval").and_then(|s| s.as_u64()).unwrap_or(60000),
            }
        }).unwrap_or_else(|| ModernGlobalSettings {
            max_concurrent_services: 10,
            default_timeout: 30000,
            auto_start_enabled_services: true,
            health_check_interval: 60000,
        });

        Ok(ModernMcpConfig {
            services,
            global_settings,
        })
    }

    pub async fn start_enabled_services(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.load_config().await?;

        for service_config in config.services {
            if service_config.enabled {
                info!("Starting MCP service: {}", service_config.id);

                let mut service = ModernMcpService::new(service_config);
                match service.start().await {
                    Ok(_) => {
                        self.services.insert(service.config.id.clone(), service);
                        info!("Successfully started service: {}", service.config.id);
                    }
                    Err(e) => {
                        error!("Failed to start service {}: {}", service.config.id, e);
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

    pub async fn list_tools(&mut self, service_id: Option<&str>) -> Result<Vec<(&String, &ToolInfo)>, Box<dyn std::error::Error>> {
        if let Some(service_id) = service_id {
            if let Some(service) = self.services.get_mut(service_id) {
                let tools = service.list_tools().await?;
                Ok(tools.into_iter().map(|tool| (&service.config.id, tool)).collect())
            } else {
                Ok(Vec::new())
            }
        } else {
            let mut all_tools = Vec::new();
            for (id, service) in &mut self.services {
                let tools = service.list_tools().await?;
                for tool in tools {
                    all_tools.push((id, tool));
                }
            }
            Ok(all_tools)
        }
    }

    pub async fn get_service(&mut self, service_id: &str) -> Option<&mut ModernMcpService> {
        self.services.get_mut(service_id)
    }

    pub async fn list_services(&self) -> Vec<&ModernMcpService> {
        self.services.values().collect()
    }

    pub async fn get_usage_stats(&self) -> HashMap<String, HashMap<String, (u64, Option<Instant>)>> {
        let mut all_stats = HashMap::new();
        for (id, service) in &self.services {
            all_stats.insert(id.clone(), service.get_tool_usage_stats());
        }
        all_stats
    }
}
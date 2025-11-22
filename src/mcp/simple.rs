use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::{Command, Child, Stdio};
use std::time::{Duration, Instant};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tracing::{info, warn, error, debug};

/// A simplified MCP service manager without rmcp dependency
#[derive(Debug, Clone)]
pub struct SimpleMcpService {
    pub config: SimpleMcpServiceConfig,
    pub status: ServiceStatus,
    pub process: Option<Child>,
    pub tools: HashMap<String, ToolInfo>,
    pub started_at: Option<Instant>,
    pub restart_count: u32,
}

#[derive(Debug, Clone)]
pub struct SimpleMcpServiceConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub timeout: Duration,
    pub max_restarts: u32,
    pub tools: Vec<String>,
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

#[derive(Debug, Clone)]
pub struct SimpleMcpManager {
    services: HashMap<String, SimpleMcpService>,
    config_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleMcpConfig {
    pub services: Vec<SimpleMcpServiceConfig>,
    pub global_settings: SimpleGlobalSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleGlobalSettings {
    pub max_concurrent_services: usize,
    pub default_timeout: u64,
    pub auto_start_enabled_services: bool,
}

impl SimpleMcpManager {
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            services: HashMap::new(),
            config_path: config_path.to_string(),
        })
    }

    pub async fn load_config(&self) -> Result<SimpleMcpConfig, Box<dyn std::error::Error>> {
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
                let tools = service.get("tools").and_then(|t| t.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                }).unwrap_or_default();

                SimpleMcpServiceConfig {
                    id,
                    name,
                    description,
                    enabled,
                    command,
                    args,
                    env: HashMap::new(),
                    timeout: Duration::from_millis(service.get("timeout").and_then(|t| t.as_u64()).unwrap_or(30000)),
                    max_restarts: service.get("max_restarts").and_then(|r| r.as_u32()).unwrap_or(3),
                    tools,
                }
            }).collect()
        } else {
            Vec::new()
        };

        let global_settings = full_config.get("global_settings").map(|gs| {
            SimpleGlobalSettings {
                max_concurrent_services: gs.get("max_concurrent_services").and_then(|s| s.as_usize()).unwrap_or(10),
                default_timeout: gs.get("default_timeout").and_then(|s| s.as_u64()).unwrap_or(30000),
                auto_start_enabled_services: gs.get("auto_start_enabled_services").and_then(|s| s.as_bool()).unwrap_or(true),
            }
        }).unwrap_or_else(|| SimpleGlobalSettings {
            max_concurrent_services: 10,
            default_timeout: 30000,
            auto_start_enabled_services: true,
        });

        Ok(SimpleMcpConfig {
            services,
            global_settings,
        })
    }

    pub async fn start_enabled_services(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.load_config().await?;

        for service_config in config.services {
            if service_config.enabled {
                info!("Starting MCP service: {}", service_config.id);

                let mut service = SimpleMcpService::new(service_config);
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
            if service.status != ServiceStatus::Running {
                return Err(format!("Service {} is not running", service_id).into());
            }

            info!("Calling tool {}::{}", service_id, tool_name);

            // Update usage stats
            if let Some(tool) = service.tools.get_mut(tool_name) {
                tool.usage_count += 1;
                tool.last_used = Some(Instant::now());
            }

            // For now, return a mock response
            // TODO: Implement actual MCP protocol communication
            let mock_response = serde_json::json!({
                "result": format!("Mock execution of tool {} with args: {:?}", tool_name, arguments),
                "service_id": service_id,
                "tool_name": tool_name,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            Ok(mock_response)
        } else {
            Err(format!("Service {} not found", service_id).into())
        }
    }

    pub async fn list_tools(&self, service_id: Option<&str>) -> Vec<(String, String, ToolInfo)> {
        let mut all_tools = Vec::new();

        if let Some(service_id) = service_id {
            if let Some(service) = self.services.get(service_id) {
                for tool in service.tools.values() {
                    all_tools.push((service_id.to_string(), tool.name.clone(), tool.clone()));
                }
            }
        } else {
            for (sid, service) in &self.services {
                for tool in service.tools.values() {
                    all_tools.push((sid.clone(), tool.name.clone(), tool.clone()));
                }
            }
        }

        all_tools
    }

    pub async fn get_service_status(&self, service_id: &str) -> Option<&SimpleMcpService> {
        self.services.get(service_id)
    }

    pub async fn list_services(&self) -> Vec<&SimpleMcpService> {
        self.services.values().collect()
    }
}

impl SimpleMcpService {
    pub fn new(config: SimpleMcpServiceConfig) -> Self {
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

        // For now, just simulate starting the service
        // TODO: Implement actual process management
        self.status = ServiceStatus::Running;
        self.started_at = Some(Instant::now());

        // Load available tools (mock implementation)
        self.load_mock_tools().await?;

        info!("Successfully started service: {}", self.config.id);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Stopping MCP service: {}", self.config.id);

        self.status = ServiceStatus::Stopped;
        self.process = None;
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

    pub fn uptime(&self) -> Option<Duration> {
        self.started_at.map(|start| start.elapsed())
    }

    async fn load_mock_tools(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Create mock tool information based on the tools list in config
        for tool_name in &self.config.tools {
            let tool_info = ToolInfo {
                name: tool_name.clone(),
                description: format!("Mock implementation of {}", tool_name),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                })),
                category: self.determine_tool_category(tool_name),
                requires_approval: self.requires_tool_approval(tool_name),
                usage_count: 0,
                last_used: None,
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
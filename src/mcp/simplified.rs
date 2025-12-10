use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    process::Command as TokioCommand,
    sync::Mutex,
    time::{sleep, timeout},
};
use tracing::{info, warn, error, debug};

use crate::mcp::{
    models::{CancellationToken, McpExecutionResult, McpSettings, ToolCallRequest},
    practical::{PracticalMcpServiceConfig, ServiceStatus},
};

/// Simplified Enhanced MCP Service with process management and timeout support
#[derive(Debug)]
pub struct SimplifiedMcpService {
    pub config: PracticalMcpServiceConfig,
    pub status: ServiceStatus,
    pub process: Option<tokio::process::Child>,
    pub started_at: Option<Instant>,
    pub restart_count: u32,
    pub cancellation_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub settings: McpSettings,
    pub tools: HashMap<String, ToolInfo>,
}

impl Clone for SimplifiedMcpService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            status: self.status.clone(),
            process: None, // Don't clone the running process
            started_at: self.started_at,
            restart_count: self.restart_count,
            cancellation_tokens: self.cancellation_tokens.clone(),
            settings: self.settings.clone(),
            tools: self.tools.clone(),
        }
    }
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

impl SimplifiedMcpService {
    pub fn new(config: PracticalMcpServiceConfig, settings: McpSettings) -> Self {
        Self {
            config,
            status: ServiceStatus::Stopped,
            process: None,
            started_at: None,
            restart_count: 0,
            cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            settings,
            tools: HashMap::new(),
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.status == ServiceStatus::Running {
            return Ok(());
        }

        self.status = ServiceStatus::Starting;
        info!("Starting simplified MCP service: {}", self.config.id);

        // Build command
        let mut cmd = TokioCommand::new(&self.config.command);
        cmd.args(&self.config.args);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.kill_on_drop(true);

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

        info!("Successfully started simplified service: {}", self.config.id);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Stopping simplified MCP service: {}", self.config.id);

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

    pub async fn call_tool(
        &mut self,
        request: ToolCallRequest,
    ) -> Result<McpExecutionResult, Box<dyn std::error::Error>> {
        if self.status != ServiceStatus::Running {
            return Err(format!("Service {} is not running", self.config.id).into());
        }

        let start_time = Instant::now();
        let cancellation_token = CancellationToken::new();
        let token_id = cancellation_token.id.clone();

        // Store cancellation token
        {
            let tokens = self.cancellation_tokens.lock().await;
            let mut tokens_guard = tokens;
            tokens_guard.insert(token_id.clone(), cancellation_token.clone());
        }

        info!("Calling tool {} on service {}", request.tool_name, self.config.id);

        // Update usage stats
        if let Some(tool) = self.tools.get_mut(&request.tool_name) {
            tool.usage_count += 1;
            tool.last_used = Some(Instant::now());
        }

        // For now, simulate tool execution with timeout and cancellation support
        let execution_delay = Duration::from_millis(100 + (request.tool_name.len() as u64 * 10));

        let result = tokio::select! {
            _ = sleep(execution_delay) => {
                // Check if cancelled before proceeding
                if cancellation_token.is_cancelled() {
                    Ok(McpExecutionResult {
                        success: false,
                        result: None,
                        error: Some("Tool call was cancelled".to_string()),
                        server_id: self.config.id.clone(),
                        tool_name: request.tool_name,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    })
                } else {
                    // Simulate successful tool execution
                    Ok(McpExecutionResult {
                        success: true,
                        result: Some(serde_json::json!({
                            "message": format!("Mock execution of tool {} with args: {:?}", request.tool_name, request.arguments),
                            "tool_name": request.tool_name,
                            "server_id": self.config.id,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                        error: None,
                        server_id: self.config.id.clone(),
                        tool_name: request.tool_name,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    })
                }
            }
            _ = async {
                while !cancellation_token.is_cancelled() {
                    sleep(Duration::from_millis(50)).await;
                }
            } => {
                Ok(McpExecutionResult {
                    success: false,
                    result: None,
                    error: Some(format!("Tool call '{}' was cancelled", request.tool_name)),
                    server_id: self.config.id.clone(),
                    tool_name: request.tool_name,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                })
            }
        };

        // Clean up cancellation token
        {
            let tokens = self.cancellation_tokens.lock().await;
            let mut tokens_guard = tokens;
            tokens_guard.remove(&token_id);
        }

        result
    }

    pub async fn cancel_tool_call(&self, token_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let tokens = self.cancellation_tokens.lock().await;
        if let Some(token) = tokens.get(token_id) {
            token.cancel();
            info!("Cancelled tool call with token: {}", token_id);
            Ok(())
        } else {
            Err(format!("Cancellation token {} not found", token_id).into())
        }
    }

    pub async fn health_check(&self) -> bool {
        if self.status != ServiceStatus::Running {
            return false;
        }

        if let Some(ref child) = self.process {
            // Try to check if the process is still running
            match child.id() {
                Some(_pid) => true, // Process is still running
                None => false,
            }
        } else {
            false
        }
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
                auto_approved: false,
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
}

/// Simplified Enhanced MCP Manager
#[derive(Debug, Clone)]
pub struct SimplifiedMcpManager {
    services: HashMap<String, SimplifiedMcpService>,
    config_path: String,
    settings: McpSettings,
}

impl SimplifiedMcpManager {
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            services: HashMap::new(),
            config_path: config_path.to_string(),
            settings: McpSettings::default(),
        })
    }

    /// Test function to verify configuration loading
    pub async fn test_config_loading(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let configs = self.load_config().await?;
        info!("Successfully loaded {} MCP service configurations", configs.len());

        for config in &configs {
            info!("  - {} ({}): enabled={}, tools={}",
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
                let env = service.get("env").and_then(|e| e.as_object()).map(|obj| {
                    obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect()
                }).unwrap_or_default();
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
                    env,
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
                info!("Starting simplified MCP service: {}", service_config.id);

                let service_id = service_config.id.clone();
                let mut service = SimplifiedMcpService::new(service_config, self.settings.clone());

                match service.start().await {
                    Ok(_) => {
                        self.services.insert(service_id.clone(), service);
                        info!("Successfully started simplified service: {}", service_id);
                    }
                    Err(e) => {
                        error!("Failed to start simplified service {}: {}", service_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn call_tool(
        &mut self,
        request: ToolCallRequest,
    ) -> Result<McpExecutionResult, Box<dyn std::error::Error>> {
        let service_id = request.server_id.as_ref().unwrap_or(&"default".to_string()).clone();

        if let Some(service) = self.services.get_mut(&service_id) {
            service.call_tool(request).await
        } else {
            Err(format!("Service {} not found", service_id).into())
        }
    }

    pub async fn cancel_tool_call(
        &self,
        service_id: &str,
        token_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(service) = self.services.get(service_id) {
            service.cancel_tool_call(token_id).await
        } else {
            Err(format!("Service {} not found", service_id).into())
        }
    }

    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();

        for (id, service) in &self.services {
            results.insert(id.clone(), service.health_check().await);
        }

        results
    }

    pub async fn list_all_tools(&self) -> Vec<(String, &ToolInfo)> {
        let mut all_tools = Vec::new();

        for (id, service) in &self.services {
            for tool in service.list_tools().await {
                all_tools.push((id.clone(), tool));
            }
        }

        all_tools
    }

    pub async fn get_rustgpt_tools(&self) -> Vec<crate::mcp::practical::RegisteredTool> {
        let mut tools = Vec::new();

        for (service_id, service) in &self.services {
            for tool in service.list_tools().await {
                tools.push(crate::mcp::practical::RegisteredTool {
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
}
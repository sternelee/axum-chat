use rmcp::{
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
    transport::{
        streamable_http_client::StreamableHttpClientTransportConfig,
        StreamableHttpClientTransport, TokioChildProcess,
    },
    ServiceExt,
    RoleClient,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{Mutex, oneshot},
    time::{sleep, timeout},
};
use tracing::{info, warn, error, debug};

use crate::mcp::{
    constants::*,
    models::{CancellationToken, McpExecutionResult, McpServerConfig, McpSettings, ToolCallRequest},
    practical::{PracticalMcpService, PracticalMcpServiceConfig, ServiceStatus},
};

/// Enhanced MCP Service with real rmcp integration
#[derive(Debug)]
pub struct EnhancedMcpService {
    pub config: PracticalMcpServiceConfig,
    pub status: ServiceStatus,
    pub rmcp_service: Option<rmcp::service::RunningService<RoleClient, TokioChildProcess>>,
    pub started_at: Option<Instant>,
    pub restart_count: u32,
    pub cancellation_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub settings: McpSettings,
}

impl EnhancedMcpService {
    pub fn new(config: PracticalMcpServiceConfig, settings: McpSettings) -> Self {
        Self {
            config,
            status: ServiceStatus::Stopped,
            rmcp_service: None,
            started_at: None,
            restart_count: 0,
            cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            settings,
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.status == ServiceStatus::Running {
            return Ok(());
        }

        self.status = ServiceStatus::Starting;
        info!("Starting enhanced MCP service: {}", self.config.id);

        // Extract server configuration
        let server_config = self.extract_server_config()?;

        if let Some(transport_type) = &server_config.transport_type {
            if transport_type == "http" {
                if let Some(url) = &server_config.url {
                    return self.start_http_transport(url, &server_config).await;
                }
            }
        }

        // Default to child process transport
        self.start_child_process_transport(&server_config).await
    }

    async fn start_child_process_transport(
        &mut self,
        config: &McpServerConfig
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = tokio::process::Command::new(&config.command);

        // Add arguments
        config.args.iter().for_each(|arg| {
            if let Some(arg_str) = arg.as_str() {
                cmd.arg(arg_str);
            }
        });

        // Set environment variables
        config.envs.iter().for_each(|(key, value)| {
            if let Some(value_str) = value.as_str() {
                cmd.env(key, value_str);
            }
        });

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        // Create child process transport
        let (process, _stderr) = TokioChildProcess::builder(cmd)
            .spawn()
            .map_err(|e| format!("Failed to spawn MCP process: {}", e))?;

        // Create client info
        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "RustGPT MCP Client".to_string(),
                version: "1.0.0".to_string(),
                title: None,
                website_url: None,
                icons: None,
            },
        };

        // Start the service
        let service = client_info
            .serve(process)
            .await
            .map_err(|e| format!("Failed to start MCP service: {}", e))?;

        info!("Successfully connected to MCP service: {}", self.config.id);

        self.rmcp_service = Some(service);
        self.status = ServiceStatus::Running;
        self.started_at = Some(Instant::now());

        Ok(())
    }

    async fn start_http_transport(
        &mut self,
        url: &str,
        _config: &McpServerConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // For now, skip HTTP transport as it's complex
        // We'll focus on child process transport first
        warn!("HTTP transport not yet implemented for URL: {}", url);
        Err(format!("HTTP transport not yet implemented: {}", url).into())
    }

  
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Stopping enhanced MCP service: {}", self.config.id);

        self.status = ServiceStatus::Stopped;

        if let Some(service) = self.rmcp_service.take() {
            match service.cancel().await {
                Ok(_) => {
                    info!("Successfully stopped MCP service: {}", self.config.id);
                }
                Err(e) => {
                    warn!("Failed to gracefully stop MCP service {}: {}", self.config.id, e);
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
            tokens.insert(token_id.clone(), cancellation_token.clone());
        }

        let service = self.rmcp_service.as_ref()
            .ok_or("MCP service not available")?;

        let arguments_map = request.arguments
            .and_then(|args| args.as_object().cloned())
            .unwrap_or_default();

        let tool_call = service.call_tool(CallToolRequestParam {
            name: request.tool_name.clone().into(),
            arguments: Some(arguments_map),
        });

        // Execute with timeout and cancellation support
        let timeout_duration = Duration::from_millis(request.timeout_ms.unwrap_or(
            self.settings.tool_call_timeout_seconds * 1000
        ));

        let result = tokio::select! {
            result = timeout(timeout_duration, tool_call) => {
                match result {
                    Ok(call_result) => call_result.map_err(|e| e.to_string()),
                    Err(_) => Err(format!("Tool call '{}' timed out after {}ms", request.tool_name, timeout_duration.as_millis())),
                }
            }
            _ = async {
                while !cancellation_token.is_cancelled() {
                    sleep(Duration::from_millis(100)).await;
                }
            } => {
                Err(format!("Tool call '{}' was cancelled", request.tool_name))
            }
        };

        // Clean up cancellation token
        {
            let tokens = self.cancellation_tokens.lock().await;
            tokens.remove(&token_id);
        }

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(call_result) => {
                let success = call_result.is_error != Some(true);
                let error = if call_result.is_error == Some(true) {
                    call_result.content.first()
                        .and_then(|c| c.as_text())
                        .map(|t| t.text.clone())
                } else {
                    None
                };

                Ok(McpExecutionResult {
                    success,
                    result: None, // Skip serialization for now to avoid type issues
                    error,
                    server_id: self.config.id.clone(),
                    tool_name: request.tool_name,
                    execution_time_ms,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                })
            }
            Err(e) => Ok(McpExecutionResult {
                success: false,
                result: None,
                error: Some(e.to_string()),
                server_id: self.config.id.clone(),
                tool_name: request.tool_name,
                execution_time_ms,
                timestamp: chrono::Utc::now().to_rfc3339(),
            }),
        }
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

        if let Some(service) = &self.rmcp_service {
            match timeout(Duration::from_secs(2), service.list_all_tools()).await {
                Ok(Ok(_tools)) => true,
                Ok(Err(_e)) => false,
                Err(_timeout) => false,
            }
        } else {
            false
        }
    }

    fn extract_server_config(&self) -> Result<McpServerConfig, Box<dyn std::error::Error>> {
        Ok(McpServerConfig {
            transport_type: None, // Could be added to config later
            url: None, // Could be added to config later
            command: self.config.command.clone(),
            args: self.config.args.iter().map(|arg| serde_json::Value::String(arg.clone())).collect(),
            envs: self.config.env.iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect(),
            timeout: Some(Duration::from_millis(self.config.timeout)),
            headers: serde_json::Map::new(),
        })
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.started_at.map(|start| start.elapsed())
    }
}

/// Enhanced MCP Manager with real protocol support
#[derive(Debug)]
pub struct EnhancedMcpManager {
    services: HashMap<String, EnhancedMcpService>,
    config_path: String,
    settings: McpSettings,
}

impl EnhancedMcpManager {
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            services: HashMap::new(),
            config_path: config_path.to_string(),
            settings: McpSettings::default(),
        })
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
                info!("Starting enhanced MCP service: {}", service_config.id);

                let service_id = service_config.id.clone();
                let mut service = EnhancedMcpService::new(service_config, self.settings.clone());

                match service.start().await {
                    Ok(_) => {
                        self.services.insert(service_id.clone(), service);
                        info!("Successfully started enhanced service: {}", service_id);
                    }
                    Err(e) => {
                        error!("Failed to start enhanced service {}: {}", service_id, e);
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
}
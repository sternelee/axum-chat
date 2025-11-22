use crate::mcp::config::{McpConfig, RustGptIntegration};
use crate::mcp::service::{McpService, ServiceStatus, ServiceHealth};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use tokio::time::{interval, timeout};
use tracing::{info, warn, error, debug};

#[derive(Clone)]
pub struct McpManager {
    services: Arc<RwLock<HashMap<String, McpService>>>,
    config: Arc<RwLock<McpConfig>>,
    audit_logger: Arc<Mutex<Option<AuditLogger>>>,
    health_checker: Arc<Mutex<Option<HealthChecker>>>,
}

pub struct AuditLogger {
    log_file: String,
    enabled: bool,
}

pub struct HealthChecker {
    interval: Duration,
    running: bool,
}

impl McpManager {
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = crate::mcp::config::load_config(config_path)?;

        // Initialize audit logger if enabled
        let audit_logger = if config.security.audit_logging.enabled {
            Some(AuditLogger::new(&config.security.audit_logging.log_file))
        } else {
            None
        };

        let manager = Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(config)),
            audit_logger: Arc::new(Mutex::new(audit_logger)),
            health_checker: Arc::new(Mutex::new(None)),
        };

        Ok(manager)
    }

    pub async fn start_enabled_services(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let mut services = self.services.write().await;

        for service_config in &config.services {
            if service_config.enabled {
                info!("Starting enabled service: {}", service_config.id);

                let mut service = McpService::new(service_config.clone());

                match service.start().await {
                    Ok(_) => {
                        services.insert(service_config.id.clone(), service);

                        // Log service start
                        if let Ok(logger) = self.audit_logger.try_lock() {
                            if let Some(ref logger) = *logger {
                                logger.log_service_start(&service_config.id).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to start service {}: {}", service_config.id, e);
                    }
                }
            }
        }

        drop(services);
        drop(config);

        // Start health checker
        self.start_health_checker().await?;

        Ok(())
    }

    pub async fn start_service(&self, service_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().await;

        let service_config = config.services.iter()
            .find(|s| s.id == service_id)
            .ok_or(format!("Service {} not found", service_id))?
            .clone();

        drop(config);

        let mut services = self.services.write().await;
        let mut service = McpService::new(service_config);

        service.start().await?;
        services.insert(service_id.to_string(), service);

        // Log service start
        if let Ok(logger) = self.audit_logger.try_lock() {
            if let Some(ref logger) = *logger {
                logger.log_service_start(service_id).await;
            }
        }

        Ok(())
    }

    pub async fn stop_service(&self, service_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut services = self.services.write().await;

        if let Some(mut service) = services.remove(service_id) {
            service.stop().await?;
            info!("Stopped service: {}", service_id);
        }

        Ok(())
    }

    pub async fn restart_service(&self, service_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.stop_service(service_id).await?;
        self.start_service(service_id).await?;
        Ok(())
    }

    pub async fn call_tool(
        &self,
        service_id: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut services = self.services.write().await;

        let service = services.get_mut(service_id)
            .ok_or(format!("Service {} not found or not running", service_id))?;

        // Check rate limits
        self.check_rate_limit(service_id).await?;

        // Log tool call
        if let Ok(logger) = self.audit_logger.try_lock() {
            if let Some(ref logger) = *logger {
                logger.log_tool_call(service_id, tool_name, &arguments).await;
            }
        }

        let start_time = Instant::now();
        let result = service.call_tool(tool_name, arguments).await;
        let execution_time = start_time.elapsed();

        if let Err(ref e) = result {
            warn!("Tool call failed: {}::{} - {}", service_id, tool_name, e);
        }

        Ok(result?)
    }

    pub async fn list_tools(&self, service_id: Option<&str>) -> Vec<ToolInfo> {
        let services = self.services.read().await;

        if let Some(service_id) = service_id {
            if let Some(service) = services.get(service_id) {
                service.list_tools().await.into_iter().map(|t| ToolInfo {
                    service_id: service_id.to_string(),
                    name: t.name.clone(),
                    description: t.description.clone(),
                    category: t.category.clone(),
                    requires_approval: t.requires_approval,
                    usage_count: t.usage_count,
                    last_used: t.last_used,
                }).collect()
            } else {
                Vec::new()
            }
        } else {
            let mut all_tools = Vec::new();
            for (sid, service) in services.iter() {
                let tools = service.list_tools().await;
                for tool in tools {
                    all_tools.push(ToolInfo {
                        service_id: sid.clone(),
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        category: tool.category.clone(),
                        requires_approval: tool.requires_approval,
                        usage_count: tool.usage_count,
                        last_used: tool.last_used,
                    });
                }
            }
            all_tools
        }
    }

    pub async fn get_service_status(&self, service_id: &str) -> Option<ServiceStatusInfo> {
        let services = self.services.read().await;

        services.get(service_id).map(|service| ServiceStatusInfo {
            id: service_id.to_string(),
            name: service.config.name.clone(),
            status: service.status.clone(),
            health: ServiceHealth::Healthy, // TODO: Implement health check
            uptime: service.uptime(),
            restart_count: service.restart_count,
            last_error: service.last_error.clone(),
            tool_count: service.tool_registry.list_tools().len(),
        })
    }

    pub async fn list_services(&self) -> Vec<ServiceStatusInfo> {
        let services = self.services.read().await;
        let mut status_list = Vec::new();

        for (id, service) in services.iter() {
            status_list.push(ServiceStatusInfo {
                id: id.clone(),
                name: service.config.name.clone(),
                status: service.status.clone(),
                health: ServiceHealth::Healthy, // TODO: Implement health check
                uptime: service.uptime(),
                restart_count: service.restart_count,
                last_error: service.last_error.clone(),
                tool_count: service.tool_registry.list_tools().len(),
            });
        }

        status_list
    }

    pub async fn get_usage_stats(&self) -> UsageStats {
        let services = self.services.read().await;
        let mut total_tool_calls = 0;
        let mut service_stats = HashMap::new();

        for (id, service) in services.iter() {
            let usage = service.tool_registry.get_usage_stats();
            let service_total: u64 = usage.values().map(|(count, _)| *count).sum();

            total_tool_calls += service_total;
            service_stats.insert(id.clone(), ServiceUsageStats {
                tool_calls: service_total,
                tools: usage,
            });
        }

        UsageStats {
            total_tool_calls,
            service_stats,
        }
    }

    pub async fn update_config(&self, new_config: McpConfig) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = self.config.write().await;
        *config = new_config;
        Ok(())
    }

    pub async fn reload_config(&self, config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let new_config = crate::mcp::config::load_config(config_path)?;
        self.update_config(new_config).await?;
        info!("Reloaded MCP configuration from {}", config_path);
        Ok(())
    }

    // RustGPT specific methods
    pub async fn get_rustgpt_tools(&self) -> Vec<RustGptTool> {
        let config = self.config.read().await;
        let tools = self.list_tools(None).await;

        tools.into_iter().filter_map(|tool| {
            // Check if tool is auto-approved for RustGPT
            let auto_approved = config.integrations
                .get("rustgpt")
                .and_then(|integration| {
                    serde_json::from_value::<RustGptIntegration>(integration.clone()).ok()
                })
                .map(|integration| integration.auto_approve_tools.contains(&tool.name))
                .unwrap_or(false);

            Some(RustGptTool {
                id: format!("{}::{}", tool.service_id, tool.name),
                service_id: tool.service_id,
                name: tool.name,
                description: tool.description,
                category: tool.category,
                requires_approval: tool.requires_approval && !auto_approved,
                auto_approved,
                usage_count: tool.usage_count,
                last_used: tool.last_used,
            })
        }).collect()
    }

    // Private methods
    async fn start_health_checker(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let interval = Duration::from_millis(config.global_settings.health_check_interval);
        drop(config);

        let services = self.services.clone();
        let audit_logger = self.audit_logger.clone();

        tokio::spawn(async move {
            let mut interval = interval(interval);
            loop {
                interval.tick().await;

                let services_snapshot = {
                    let services_read = services.read().await;
                    services_read.clone()
                };

                for (id, service) in services_snapshot {
                    let health = service.health_check().await;

                    match health {
                        ServiceHealth::Unhealthy(reason) => {
                            warn!("Service {} is unhealthy: {}", id, reason);

                            if let Ok(logger) = audit_logger.try_lock() {
                                if let Some(ref logger) = *logger {
                                    logger.log_service_error(&id, &reason).await;
                                }
                            }

                            // Try to restart the service if auto-restart is enabled
                            // TODO: Implement restart logic
                        }
                        _ => {}
                    }
                }
            }
        });

        Ok(())
    }

    async fn check_rate_limit(&self, service_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().await;

        // Check global rate limit
        let global_limit = config.security.rate_limiting.global_requests_per_minute;
        // TODO: Implement actual rate limiting logic

        // Check per-service rate limit
        if let Some(&service_limit) = config.security.rate_limiting.per_service_limits.get(service_id) {
            // TODO: Implement per-service rate limiting
        }

        Ok(())
    }
}

impl AuditLogger {
    fn new(log_file: &str) -> Self {
        Self {
            log_file: log_file.to_string(),
            enabled: true,
        }
    }

    async fn log_service_start(&self, service_id: &str) {
        if !self.enabled {
            return;
        }

        let timestamp = chrono::Utc::now().to_rfc3339();
        let log_entry = format!(
            "{} [START] Service: {}\n",
            timestamp, service_id
        );

        if let Err(e) = self.write_log(&log_entry).await {
            error!("Failed to write audit log: {}", e);
        }
    }

    async fn log_tool_call(
        &self,
        service_id: &str,
        tool_name: &str,
        arguments: &Option<serde_json::Value>,
    ) {
        if !self.enabled {
            return;
        }

        let timestamp = chrono::Utc::now().to_rfc3339();
        let args_json = arguments.as_ref()
            .map(|a| serde_json::to_string(a).unwrap_or_default())
            .unwrap_or_else(|| "null".to_string());

        let log_entry = format!(
            "{} [TOOL_CALL] Service: {}, Tool: {}, Arguments: {}\n",
            timestamp, service_id, tool_name, args_json
        );

        if let Err(e) = self.write_log(&log_entry).await {
            error!("Failed to write audit log: {}", e);
        }
    }

    async fn log_service_error(&self, service_id: &str, error: &str) {
        if !self.enabled {
            return;
        }

        let timestamp = chrono::Utc::now().to_rfc3339();
        let log_entry = format!(
            "{} [ERROR] Service: {}, Error: {}\n",
            timestamp, service_id, error
        );

        if let Err(e) = self.write_log(&log_entry).await {
            error!("Failed to write audit log: {}", e);
        }
    }

    async fn write_log(&self, entry: &str) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .await?;

        file.write_all(entry.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }
}

// Public data structures
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub service_id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub requires_approval: bool,
    pub usage_count: u64,
    pub last_used: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct ServiceStatusInfo {
    pub id: String,
    pub name: String,
    pub status: ServiceStatus,
    pub health: ServiceHealth,
    pub uptime: Option<Duration>,
    pub restart_count: u32,
    pub last_error: Option<String>,
    pub tool_count: usize,
}

#[derive(Debug, Clone)]
pub struct UsageStats {
    pub total_tool_calls: u64,
    pub service_stats: HashMap<String, ServiceUsageStats>,
}

#[derive(Debug, Clone)]
pub struct ServiceUsageStats {
    pub tool_calls: u64,
    pub tools: HashMap<String, (u64, Option<Instant>)>,
}

#[derive(Debug, Clone)]
pub struct RustGptTool {
    pub id: String,
    pub service_id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub requires_approval: bool,
    pub auto_approved: bool,
    pub usage_count: u64,
    pub last_used: Option<Instant>,
}
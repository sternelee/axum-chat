use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub services: Vec<McpServiceConfig>,
    pub global_settings: GlobalSettings,
    pub security: SecuritySettings,
    pub integrations: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServiceConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub r#type: ServiceType,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default = "HashMap::new")]
    pub env: HashMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_auto_restart")]
    pub auto_restart: bool,
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    pub permissions: ServicePermissions,
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceType {
    Stdio,
    Sse,
    WebSocket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePermissions {
    #[serde(flatten, default)]
    pub custom: HashMap<String, serde_json::Value>,
}

// Filesystem permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemPermissions {
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub allowed_extensions: Vec<String>,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: String,
    #[serde(default)]
    pub read_only: bool,
}

// Database permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabasePermissions {
    pub allowed_databases: Vec<String>,
    pub allowed_operations: Vec<String>,
    #[serde(default = "default_max_rows")]
    pub max_rows_per_query: u32,
    #[serde(default = "default_query_timeout")]
    pub query_timeout: u32,
}

// Web permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebPermissions {
    pub allowed_domains: Vec<String>,
    #[serde(default = "default_max_pages")]
    pub max_pages_per_session: u32,
    #[serde(default = "default_page_timeout")]
    pub page_timeout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSettings {
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_services: usize,
    #[serde(default = "default_timeout")]
    pub default_timeout: u64,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_auto_start")]
    pub auto_start_enabled_services: bool,
    #[serde(default = "default_health_interval")]
    pub health_check_interval: u64,
    pub service_discovery: Option<ServiceDiscovery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscovery {
    #[serde(default)]
    pub auto_discover: bool,
    pub discovery_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    #[serde(default = "default_require_approval")]
    pub require_approval_for_new_tools: bool,
    #[serde(default)]
    pub allowed_tool_categories: Vec<String>,
    #[serde(default)]
    pub blocked_tools: Vec<String>,
    pub rate_limiting: RateLimiting,
    pub audit_logging: AuditLogging,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiting {
    #[serde(default = "default_global_rate_limit")]
    pub global_requests_per_minute: u32,
    #[serde(default = "HashMap::new")]
    pub per_service_limits: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogging {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_log_file")]
    pub log_file: String,
    #[serde(default = "default_log_tool_calls")]
    pub log_tool_calls: bool,
    #[serde(default = "default_log_service_starts")]
    pub log_service_starts: bool,
}

// RustGPT integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustGptIntegration {
    #[serde(default)]
    pub auto_approve_tools: Vec<String>,
    #[serde(default = "default_tool_timeout_override")]
    pub tool_timeout_override: u64,
    #[serde(default = "HashMap::new")]
    pub custom_tools: HashMap<String, CustomTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTool {
    pub service: String,
    pub tool: String,
    pub description: String,
    pub allowed_extensions: Option<Vec<String>>,
    pub preprocessing: Option<HashMap<String, serde_json::Value>>,
    pub output_path: Option<String>,
    pub file_format: Option<String>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            services: Vec::new(),
            global_settings: GlobalSettings::default(),
            security: SecuritySettings::default(),
            integrations: HashMap::new(),
        }
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            max_concurrent_services: default_max_concurrent(),
            default_timeout: default_timeout(),
            log_level: default_log_level(),
            auto_start_enabled_services: default_auto_start(),
            health_check_interval: default_health_interval(),
            service_discovery: None,
        }
    }
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            require_approval_for_new_tools: default_require_approval(),
            allowed_tool_categories: vec![
                "filesystem".to_string(),
                "database".to_string(),
                "web".to_string(),
                "search".to_string(),
            ],
            blocked_tools: Vec::new(),
            rate_limiting: RateLimiting::default(),
            audit_logging: AuditLogging::default(),
        }
    }
}

impl Default for RateLimiting {
    fn default() -> Self {
        Self {
            global_requests_per_minute: default_global_rate_limit(),
            per_service_limits: HashMap::new(),
        }
    }
}

impl Default for AuditLogging {
    fn default() -> Self {
        Self {
            enabled: true,
            log_file: default_log_file(),
            log_tool_calls: default_log_tool_calls(),
            log_service_starts: default_log_service_starts(),
        }
    }
}

// Helper functions for defaults
fn default_timeout() -> u64 { 30000 }
fn default_auto_restart() -> bool { true }
fn default_max_restarts() -> u32 { 3 }
fn default_max_concurrent() -> usize { 10 }
fn default_log_level() -> String { "info".to_string() }
fn default_auto_start() -> bool { true }
fn default_health_interval() -> u64 { 60000 }
fn default_require_approval() -> bool { true }
fn default_global_rate_limit() -> u32 { 100 }
fn default_log_file() -> String { "./logs/mcp-audit.log".to_string() }
fn default_log_tool_calls() -> bool { true }
fn default_log_service_starts() -> bool { true }
fn default_max_file_size() -> String { "10MB".to_string() }
fn default_max_rows() -> u32 { 1000 }
fn default_query_timeout() -> u32 { 30 }
fn default_max_pages() -> u32 { 5 }
fn default_page_timeout() -> u32 { 30 }
fn default_tool_timeout_override() -> u64 { 45000 }

// Utility functions
pub fn load_config(config_path: &str) -> Result<McpConfig, Box<dyn std::error::Error>> {
    let config_content = std::fs::read_to_string(config_path)?;
    let config: McpConfig = serde_json::from_str(&config_content)?;

    // Environment variable substitution
    let config = substitute_env_vars(config)?;

    Ok(config)
}

pub fn substitute_env_vars(mut config: McpConfig) -> Result<McpConfig, Box<dyn std::error::Error>> {
    for service in &mut config.services {
        for (_, value) in service.env.iter_mut() {
            if value.starts_with("${") && value.ends_with('}') {
                let env_var = &value[2..value.len()-1];
                if let Ok(env_value) = std::env::var(env_var) {
                    *value = env_value;
                }
            }
        }
    }
    Ok(config)
}

pub fn save_config(config: &McpConfig, config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_content = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path, config_content)?;
    Ok(())
}
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Configuration parameters extracted from MCP server config
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub transport_type: Option<String>,
    pub url: Option<String>,
    pub command: String,
    pub args: Vec<Value>,
    pub envs: serde_json::Map<String, Value>,
    pub timeout: Option<Duration>,
    pub headers: serde_json::Map<String, Value>,
}

/// Runtime MCP settings that can be adjusted via UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSettings {
    #[serde(default = "default_tool_call_timeout_seconds")]
    pub tool_call_timeout_seconds: u64,
    #[serde(default = "default_base_restart_delay_ms")]
    pub base_restart_delay_ms: u64,
    #[serde(default = "default_max_restart_delay_ms")]
    pub max_restart_delay_ms: u64,
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            tool_call_timeout_seconds: crate::mcp::constants::DEFAULT_MCP_TOOL_CALL_TIMEOUT_SECS,
            base_restart_delay_ms: crate::mcp::constants::DEFAULT_MCP_BASE_RESTART_DELAY_MS,
            max_restart_delay_ms: crate::mcp::constants::DEFAULT_MCP_MAX_RESTART_DELAY_MS,
            backoff_multiplier: crate::mcp::constants::DEFAULT_MCP_BACKOFF_MULTIPLIER,
        }
    }
}

impl McpSettings {
    /// Returns the tool call timeout duration, enforcing a minimum of 1 second to avoid zero-duration timeouts.
    pub fn tool_call_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.tool_call_timeout_seconds.max(1))
    }
}

/// Tool with server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolWithServer {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    pub server: String,
}

/// MCP server execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpExecutionResult {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub server_id: String,
    pub tool_name: String,
    pub execution_time_ms: u64,
    pub timestamp: String,
}

/// Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub tool_name: String,
    pub server_id: Option<String>,
    pub arguments: Option<Value>,
    pub timeout_ms: Option<u64>,
}

/// Tool call cancellation token
#[derive(Debug, Clone)]
pub struct CancellationToken {
    pub id: String,
    pub created_at: std::time::Instant,
    pub is_cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id,
            created_at: std::time::Instant::now(),
            is_cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// Helper functions for defaults
fn default_tool_call_timeout_seconds() -> u64 {
    crate::mcp::constants::DEFAULT_MCP_TOOL_CALL_TIMEOUT_SECS
}

fn default_base_restart_delay_ms() -> u64 {
    crate::mcp::constants::DEFAULT_MCP_BASE_RESTART_DELAY_MS
}

fn default_max_restart_delay_ms() -> u64 {
    crate::mcp::constants::DEFAULT_MCP_MAX_RESTART_DELAY_MS
}

fn default_backoff_multiplier() -> f64 {
    crate::mcp::constants::DEFAULT_MCP_BACKOFF_MULTIPLIER
}
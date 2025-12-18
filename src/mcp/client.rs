use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex as TokioMutex;
use tokio::time::timeout;

use super::config::{McpServerConfig, TransportType};

#[derive(Debug, Clone)]
pub struct McpConnectionInfo {
    pub name: String,
    pub transport_type: TransportType,
    pub server_info: Option<Value>,
}

// Internal state for the MCP client
struct RmcpClientState {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    request_id: i64,
}

// MCP client using direct JSON-RPC communication with interior mutability
#[derive(Clone)]
pub struct RmcpClient {
    name: String,
    state: Arc<TokioMutex<RmcpClientState>>,
}

// Result types compatible with our interface
#[derive(Debug, Clone)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: Option<Value>,
    pub timeout: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct CallToolResult {
    pub content: Vec<McpContent>,
    pub is_error: Option<bool>,
    pub structured_content: Option<Value>,
    pub meta: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct McpContent {
    pub r#type: String,
    pub text: Option<String>,
    pub data: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Debug, Clone)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReadResourceParams {
    pub uri: String,
}

#[derive(Debug, Clone)]
pub struct ListResourcesResult {
    pub resources: Vec<Value>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReadResourceResult {
    pub contents: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct ListPromptsResult {
    pub prompts: Vec<Value>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GetPromptResult {
    pub description: Option<String>,
    pub messages: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    pub tools: Option<Value>,
    pub resources: Option<Value>,
    pub prompts: Option<Value>,
}

#[async_trait]
pub trait McpClientTrait: Send + Sync {
    async fn initialize(&mut self) -> Result<(), McpClientError>;
    async fn list_tools(&self) -> Result<ListToolsResult, McpClientError>;
    async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, McpClientError>;
    async fn list_resources(&self) -> Result<ListResourcesResult, McpClientError>;
    async fn read_resource(
        &self,
        params: ReadResourceParams,
    ) -> Result<ReadResourceResult, McpClientError>;
    async fn list_prompts(&self) -> Result<ListPromptsResult, McpClientError>;
    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<GetPromptResult, McpClientError>;
    fn get_connection_info(&self) -> McpConnectionInfo;
    async fn shutdown(&mut self) -> Result<(), McpClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum McpClientError {
    #[error("Initialization error: {0}")]
    Initialization(String),
    #[error("Tool execution error: {0}")]
    ToolExecution(String),
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Timeout error")]
    Timeout,
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("MCP protocol error: {0}")]
    Protocol(String),
    #[error("Process error: {0}")]
    Process(String),
}

impl RmcpClient {
    pub async fn new(name: String, config: &McpServerConfig) -> Result<Self, McpClientError> {
        if config.command.is_none() {
            return Err(McpClientError::Configuration(
                "Command is required for stdio transport".to_string(),
            ));
        }

        let command = config.command.as_ref().unwrap().clone();
        let args = config.args.as_ref().unwrap_or(&vec![]).clone();
        let env = config.env.as_ref().unwrap_or(&HashMap::new()).clone();

        let mut cmd = Command::new(&command);
        cmd.args(&args)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::inherit());

        // Set environment variables
        for (key, value) in &env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()
            .map_err(|e| McpClientError::Process(format!("Failed to spawn process: {}", e)))?;

        let stdin = child.stdin.take().ok_or_else(|| {
            McpClientError::Process("Failed to get stdin handle".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            McpClientError::Process("Failed to get stdout handle".to_string())
        })?;

        let state = RmcpClientState {
            child,
            stdin,
            stdout,
            request_id: 0,
        };

        let client = Self {
            name,
            state: Arc::new(TokioMutex::new(state)),
        };

        // Initialize the MCP connection
        client.initialize_connection().await?;

        Ok(client)
    }

    async fn initialize_connection(&self) -> Result<(), McpClientError> {
        let mut state = self.state.lock().await;

        // Send initialize request
        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id(&mut state),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "clientInfo": {
                    "name": "axum-chat",
                    "version": "0.1.0"
                }
            }
        });

        let response = self.send_request(&mut state, init_request).await
            .map_err(|e| McpClientError::Initialization(format!("Initialize failed: {}", e)))?;

        // Send initialized notification
        let initialized_notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        self.send_notification(&mut state, initialized_notification).await
            .map_err(|e| McpClientError::Initialization(format!("Initialized notification failed: {}", e)))?;

        println!("MCP client '{}' initialized successfully", self.name);
        Ok(())
    }

    fn next_request_id(&self, state: &mut RmcpClientState) -> i64 {
        state.request_id += 1;
        state.request_id
    }

    async fn send_request(&self, state: &mut RmcpClientState, request: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let request_str = serde_json::to_string(&request)?;
        let timeout_secs = 30;

        // Send request
        state.stdin.write_all(request_str.as_bytes()).await?;
        state.stdin.write_all(b"\n").await?;
        state.stdin.flush().await?;

        // Read response
        let mut buffer = String::new();
        let mut bytes_read = 0;
        let max_bytes = 100_000; // Prevent infinite reading

        loop {
            let mut temp_buffer = [0; 1024];
            let n = timeout(Duration::from_secs(timeout_secs), state.stdout.read(&mut temp_buffer)).await??;

            if n == 0 {
                break; // EOF
            }

            bytes_read += n;
            if bytes_read > max_bytes {
                return Err("Response too large".into());
            }

            let chunk = String::from_utf8_lossy(&temp_buffer[..n]);
            buffer.push_str(&chunk);

            // Try to parse complete JSON response
            if let Ok(response) = self.extract_json_response(&buffer) {
                return Ok(response);
            }
        }

        Err("No complete response received".into())
    }

    async fn send_notification(&self, state: &mut RmcpClientState, notification: Value) -> Result<(), Box<dyn std::error::Error>> {
        let notification_str = serde_json::to_string(&notification)?;
        state.stdin.write_all(notification_str.as_bytes()).await?;
        state.stdin.write_all(b"\n").await?;
        state.stdin.flush().await?;
        Ok(())
    }

    fn extract_json_response(&self, buffer: &str) -> Result<Value, serde_json::Error> {
        let trimmed = buffer.trim();
        if trimmed.is_empty() {
            return serde_json::from_str("{}"); // Empty response
        }

        // Try to find the complete JSON object
        let mut brace_count = 0;
        let mut json_end = 0;

        for (i, char) in trimmed.chars().enumerate() {
            match char {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        json_end = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }

        if brace_count == 0 && json_end > 0 {
            let json_str = &trimmed[..json_end];
            serde_json::from_str(json_str)
        } else {
            serde_json::from_str(trimmed) // Fallback
        }
    }

}

#[async_trait]
impl McpClientTrait for RmcpClient {
    async fn initialize(&mut self) -> Result<(), McpClientError> {
        println!("Initializing MCP client: {}", self.name);
        // rmcp client is already initialized during construction
        Ok(())
    }

    async fn list_tools(&self) -> Result<ListToolsResult, McpClientError> {
        let mut state = self.state.lock().await;

        // Send tools/list request
        let list_tools_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id(&mut state),
            "method": "tools/list"
        });

        let response = self.send_request(&mut state, list_tools_request).await
            .map_err(|e| McpClientError::ToolExecution(format!("Failed to list tools: {}", e)))?;

        // Extract tools from response
        if let Some(result) = response.get("result") {
            if let Some(tools_array) = result.get("tools").and_then(|t| t.as_array()) {
                let converted_tools: Vec<Tool> = tools_array
                    .iter()
                    .filter_map(|tool| {
                        let name = tool.get("name")?.as_str()?;
                        let description = tool.get("description").and_then(|d| d.as_str()).map(String::from);
                        let input_schema = tool.get("inputSchema").cloned().unwrap_or_default();

                        Some(Tool {
                            name: name.to_string(),
                            description,
                            input_schema,
                        })
                    })
                    .collect();

                let next_cursor = result.get("nextCursor").and_then(|c| c.as_str()).map(String::from);

                return Ok(ListToolsResult {
                    tools: converted_tools,
                    next_cursor,
                });
            }
        }

        // If we get here, something went wrong with the response format
        Ok(ListToolsResult {
            tools: vec![],
            next_cursor: None,
        })
    }

    async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, McpClientError> {
        let mut state = self.state.lock().await;

        // Send tools/call request
        let call_tool_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id(&mut state),
            "method": "tools/call",
            "params": {
                "name": params.name,
                "arguments": params.arguments
            }
        });

        let response = self.send_request(&mut state, call_tool_request).await
            .map_err(|e| McpClientError::ToolExecution(format!("Failed to call tool: {}", e)))?;

        // Extract result from response
        if let Some(result) = response.get("result") {
            let content = if let Some(content_array) = result.get("content").and_then(|c| c.as_array()) {
                content_array
                    .iter()
                    .filter_map(|c| {
                        let r#type = c.get("type")?.as_str()?.to_string();
                        let text = c.get("text").and_then(|t| t.as_str()).map(String::from);
                        let data = c.get("data").and_then(|d| d.as_str()).map(String::from);
                        let mime_type = c.get("mimeType").and_then(|m| m.as_str()).map(String::from);

                        Some(McpContent {
                            r#type,
                            text,
                            data,
                            mime_type,
                        })
                    })
                    .collect()
            } else {
                vec![]
            };

            let is_error = result.get("isError").and_then(|e| e.as_bool());
            let meta = result.get("meta").cloned();

            return Ok(CallToolResult {
                content,
                is_error,
                structured_content: None,
                meta,
            });
        }

        // If we get here, something went wrong with the response format
        Err(McpClientError::ToolExecution("Invalid tool call response format".to_string()))
    }

    async fn list_resources(&self) -> Result<ListResourcesResult, McpClientError> {
        // For now, return empty resources - you can implement this later
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        _params: ReadResourceParams,
    ) -> Result<ReadResourceResult, McpClientError> {
        // For now, return empty contents - you can implement this later
        Ok(ReadResourceResult {
            contents: vec![],
        })
    }

    async fn list_prompts(&self) -> Result<ListPromptsResult, McpClientError> {
        // For now, return empty prompts - you can implement this later
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        _name: &str,
        _arguments: Option<Value>,
    ) -> Result<GetPromptResult, McpClientError> {
        // For now, return empty result - you can implement this later
        Ok(GetPromptResult {
            description: None,
            messages: vec![],
        })
    }

    fn get_connection_info(&self) -> McpConnectionInfo {
        McpConnectionInfo {
            name: self.name.clone(),
            transport_type: TransportType::Stdio,
            server_info: None,
        }
    }

    async fn shutdown(&mut self) -> Result<(), McpClientError> {
        let mut state = self.state.lock().await;

        // Send shutdown notification if possible
        let shutdown_notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/shutdown"
        });

        if let Err(e) = self.send_notification(&mut state, shutdown_notification).await {
            eprintln!("Warning: Failed to send shutdown notification: {}", e);
        }

        // Kill the process
        if let Err(e) = state.child.kill().await {
            eprintln!("Warning: Failed to kill MCP process: {}", e);
        }

        Ok(())
    }
}

// Factory function to create appropriate client
pub async fn create_mcp_client(
    name: String,
    config: &McpServerConfig,
) -> Result<Box<dyn McpClientTrait>, McpClientError> {
    let transport_type = config.transport.as_ref().unwrap_or(&TransportType::Stdio);

    match transport_type {
        TransportType::Stdio => {
            let client = RmcpClient::new(name, config).await?;
            Ok(Box::new(client))
        }
        TransportType::Sse => Err(McpClientError::Configuration(
            "SSE transport not yet implemented".to_string(),
        )),
        TransportType::Http => Err(McpClientError::Configuration(
            "HTTP transport not yet implemented".to_string(),
        )),
    }
}


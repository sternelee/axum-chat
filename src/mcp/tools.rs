use serde_json::Value;
use std::collections::HashMap;

use super::manager::{get_mcp_manager, McpManagerError};
use crate::ai::stream::GenerationEvent;

#[derive(Debug, Clone)]
pub struct McpToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct McpContent {
    pub r#type: String,
    pub text: Option<String>,
    pub data: Option<String>,
    pub mime_type: Option<String>,
}

pub async fn execute_mcp_tool(tool_call: &McpToolCall) -> Result<McpToolResult, McpManagerError> {
    let manager = get_mcp_manager();
    let call_result = manager
        .call_tool(&tool_call.name, tool_call.arguments.clone(), None)
        .await?;

    Ok(McpToolResult {
        content: call_result
            .content
            .into_iter()
            .map(|c| McpContent {
                r#type: c.r#type,
                text: c.text,
                data: c.data,
                mime_type: c.mime_type,
            })
            .collect(),
        is_error: call_result.is_error.unwrap_or(false),
    })
}

pub async fn get_available_tools() -> Result<Vec<crate::data::model::ToolInfo>, McpManagerError> {
    let manager = get_mcp_manager();
    let mcp_tools = manager.get_all_tools().await;

    let mut tools = Vec::new();

    for mcp_tool in mcp_tools {
        let tool_info = crate::data::model::ToolInfo {
            name: mcp_tool.name.clone(),
            description: mcp_tool.description.clone(),
            parameters: convert_tool_schema(&mcp_tool.tool_info.input_schema),
        };
        tools.push(tool_info);
    }

    Ok(tools)
}

fn convert_tool_schema(schema: &Value) -> Option<Value> {
    // Convert tool schema to OpenAI Function calling format
    if let Value::Object(obj) = schema {
        let mut openai_schema = serde_json::Map::new();

        // Set type to object
        openai_schema.insert("type".to_string(), Value::String("object".to_string()));

        // Extract properties
        if let Some(properties) = obj.get("properties") {
            openai_schema.insert("properties".to_string(), properties.clone());
        }

        // Extract required fields
        if let Some(required) = obj.get("required") {
            openai_schema.insert("required".to_string(), required.clone());
        }

        Some(Value::Object(openai_schema))
    } else {
        None
    }
}

pub async fn format_tool_call_for_openai(tool_call: &McpToolCall) -> crate::data::model::ToolCall {
    crate::data::model::ToolCall {
        id: tool_call.id.clone(),
        r#type: "function".to_string(),
        function: crate::data::model::FunctionCall {
            name: tool_call.name.clone(),
            arguments: serde_json::to_string(&tool_call.arguments)
                .unwrap_or_else(|_| "{}".to_string()),
        },
    }
}

pub async fn format_tool_result_for_openai(
    result: &McpToolResult,
) -> Option<crate::data::model::ToolResult> {
    let content_text = result
        .content
        .iter()
        .filter_map(|c| c.text.clone())
        .collect::<Vec<_>>()
        .join("\n");

    if content_text.is_empty() {
        return None;
    }

    Some(crate::data::model::ToolResult {
        tool_call_id: "mcp_tool".to_string(), // This should be set from context
        output: content_text,
    })
}

pub fn parse_tool_call_from_ai(tool_call: &crate::data::model::ToolCall) -> Option<McpToolCall> {
    // Check if this is an MCP tool (prefixed with server name)
    if tool_call.function.name.contains("__") {
        let arguments = match serde_json::from_str(&tool_call.function.arguments) {
            Ok(args) => args,
            Err(_) => return None,
        };

        Some(McpToolCall {
            id: tool_call.id.clone(),
            name: tool_call.function.name.clone(),
            arguments,
        })
    } else {
        None
    }
}

// Tool execution with streaming support
use tokio::sync::mpsc;

pub async fn execute_mcp_tool_streaming(
    tool_call: &McpToolCall,
    mut sender: mpsc::Sender<Result<GenerationEvent, axum::Error>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Send tool call start event
    let openai_tool_call = format_tool_call_for_openai(tool_call).await;
    if sender
        .send(Ok(GenerationEvent::ToolCall(openai_tool_call)))
        .await
        .is_err()
    {
        return Ok(()); // Channel closed
    }

    // Execute the tool
    match execute_mcp_tool(tool_call).await {
        Ok(result) => {
            // Send tool result as text
            if let Some(openai_result) = format_tool_result_for_openai(&result).await {
                let result_text = format!("Tool Result: {}", openai_result.output);
                if sender
                    .send(Ok(GenerationEvent::Text(result_text)))
                    .await
                    .is_err()
                {
                    return Ok(()); // Channel closed
                }
            }
        }
        Err(e) => {
            // Send error as text
            let error_text = format!("Tool Execution Error: {}", e);
            if sender
                .send(Ok(GenerationEvent::Text(error_text)))
                .await
                .is_err()
            {
                return Ok(()); // Channel closed
            }
        }
    }

    Ok(())
}

// Security and permission utilities
pub fn validate_tool_call(tool_name: &str, arguments: &Value) -> Result<(), SecurityError> {
    // Basic security checks
    if tool_name.contains("filesystem__") {
        validate_filesystem_tool_arguments(arguments)?;
    } else if tool_name.contains("shell__") || tool_name.contains("exec__") {
        return Err(SecurityError::DangerousOperation(
            "Shell execution tools are blocked".to_string(),
        ));
    }

    Ok(())
}

fn validate_filesystem_tool_arguments(arguments: &Value) -> Result<(), SecurityError> {
    if let Value::Object(obj) = arguments {
        if let Some(path) = obj.get("path") {
            if let Value::String(path_str) = path {
                // Basic path traversal protection
                if path_str.contains("..") {
                    return Err(SecurityError::PathTraversal(path_str.clone()));
                }

                // Check for dangerous paths
                let dangerous_paths = [
                    "/etc",
                    "/bin",
                    "/usr/bin",
                    "/sbin",
                    "/usr/sbin",
                    "C:\\Windows",
                    "C:\\Program Files",
                    "C:\\Program Files (x86)",
                ];

                for dangerous in &dangerous_paths {
                    if path_str.starts_with(dangerous) {
                        return Err(SecurityError::DangerousPath(path_str.clone()));
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Dangerous operation: {0}")]
    DangerousOperation(String),

    #[error("Path traversal attempt: {0}")]
    PathTraversal(String),

    #[error("Access to dangerous path: {0}")]
    DangerousPath(String),

    #[error("Tool not authorized: {0}")]
    NotAuthorized(String),
}

// Built-in tools that don't require MCP servers
pub mod builtin {
    use serde_json::Value;

    #[derive(Debug, Clone)]
    pub struct BuiltinTool {
        pub name: String,
        pub description: String,
        pub parameters: Option<Value>,
    }

    pub fn get_builtin_tools() -> Vec<BuiltinTool> {
        vec![
            BuiltinTool {
                name: "get_time".to_string(),
                description: "Get the current time".to_string(),
                parameters: None,
            },
            BuiltinTool {
                name: "echo".to_string(),
                description: "Echo back the provided text".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "The text to echo back"
                        }
                    },
                    "required": ["text"]
                })),
            },
        ]
    }

    pub async fn execute_builtin_tool(name: &str, arguments: Value) -> Result<Value, String> {
        match name {
            "get_time" => {
                let now = chrono::Utc::now();
                Ok(serde_json::json!({
                    "time": now.to_rfc3339(),
                    "unix_timestamp": now.timestamp()
                }))
            }
            "echo" => {
                if let Some(text) = arguments.get("text").and_then(|v| v.as_str()) {
                    Ok(serde_json::json!({
                        "echoed_text": text
                    }))
                } else {
                    Err("Missing 'text' parameter".to_string())
                }
            }
            _ => Err(format!("Unknown builtin tool: {}", name)),
        }
    }
}


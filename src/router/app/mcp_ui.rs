use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    Form,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tera::Context;
use tracing::{info, error, warn, debug};

use crate::AppState;
use crate::mcp::practical::RegisteredTool;

/// MCP UI resource request parameters
#[derive(Debug, Deserialize)]
pub struct UiResourceRequest {
    pub service_id: String,
    pub tool_name: Option<String>,
    pub action: Option<String>,
}

/// MCP UI action form data
#[derive(Debug, Deserialize)]
pub struct McpUiActionForm {
    pub service_id: String,
    pub tool_name: String,
    pub action_type: String,
    pub parameters: Value,
}

/// Tool execution request
#[derive(Debug, Deserialize)]
pub struct ToolExecutionRequest {
    pub service_id: String,
    pub tool_name: String,
    pub arguments: HashMap<String, Value>,
    pub timeout_ms: Option<u64>,
}

/// Render MCP UI preview page
pub async fn mcp_ui_page(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Html<String>, (StatusCode, String)> {
    let service_id = params.get("service_id").cloned().unwrap_or_default();
    let tool_name = params.get("tool_name").cloned();

    let mut context = Context::new();
    context.insert("service_id", &service_id);
    context.insert("tool_name", &tool_name);

    // Fetch available services and tools for the UI
    let (services, tools) = {
        let mcp_manager_option = {
            let guard = state.mcp_manager.lock().unwrap();
            guard.as_ref().cloned()
        };

        match mcp_manager_option {
            Some(manager) => {
                let services = match manager.load_config().await {
                    Ok(configs) => {
                        configs.into_iter().map(|config| json!({
                            "id": config.id,
                            "name": config.name,
                            "description": config.description,
                            "enabled": config.enabled,
                            "command": config.command
                        })).collect::<Vec<Value>>()
                    }
                    Err(e) => {
                        error!("Failed to load MCP services: {}", e);
                        Vec::new()
                    }
                };

                let tools = manager.get_rustgpt_tools().await;
                (services, tools)
            }
            None => {
                warn!("MCP manager not initialized");
                (Vec::new(), Vec::new())
            }
        }
    };

    context.insert("services", &services);
    context.insert("tools", &tools);

    let mcp_ui_view = state.tera.render("views/mcp_ui.html", &context).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    // Wrap in main layout
    let mut main_context = Context::new();
    main_context.insert("view", &mcp_ui_view);
    main_context.insert("with_footer", &true);

    let rendered = state.tera.render("views/main.html", &main_context).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(Html(rendered))
}

/// Get MCP UI resource details
pub async fn get_ui_resource(
    State(state): State<Arc<AppState>>,
    Path((service_id, tool_name)): Path<(String, String)>,
) -> impl IntoResponse {
    let mcp_manager_option = {
        let guard = state.mcp_manager.lock().unwrap();
        guard.as_ref().cloned()
    };

    match mcp_manager_option {
        Some(manager) => {
            // Find the specific tool
            let tools = manager.get_rustgpt_tools().await;
            if let Some(tool) = tools.iter().find(|t| {
                t.name == tool_name &&
                t.service_id == service_id
            }) {
                let ui_resource = create_ui_resource_from_tool(tool, &service_id);
                (StatusCode::OK, Json(ui_resource)).into_response()
            } else {
                (StatusCode::NOT_FOUND, Json(json!({
                    "error": "Tool not found",
                    "service_id": service_id,
                    "tool_name": tool_name
                }))).into_response()
            }
        }
        None => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": "MCP manager not initialized"
            }))).into_response()
        }
    }
}

/// Execute MCP tool with UI feedback
pub async fn execute_tool_with_ui(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ToolExecutionRequest>,
) -> impl IntoResponse {
    let mcp_manager_option = {
        let guard = state.mcp_manager.lock().unwrap();
        guard.as_ref().cloned()
    };

    match mcp_manager_option {
        Some(manager) => {
            info!("Executing tool {} from service {}", request.tool_name, request.service_id);

            // Create execution context
            let execution_id = uuid::Uuid::new_v4().to_string();
            let timeout = request.timeout_ms.unwrap_or(30000); // 30 seconds default

            // For now, simulate tool execution (would need actual MCP integration)
            debug!("Tool execution started with ID: {}", execution_id);

            // Return execution response with UI resource
            let response = json!({
                "execution_id": execution_id,
                "status": "started",
                "service_id": request.service_id,
                "tool_name": request.tool_name,
                "arguments": request.arguments,
                "timeout_ms": timeout,
                "ui_resource": {
                    "uri": format!("ui://tool-execution/{}", execution_id),
                    "content": {
                        "type": "remoteDom",
                        "html": format!(
                            r#"<div class="mcp-tool-execution">
                                <h3>Executing: {}</h3>
                                <div class="execution-status" data-execution-id="{}">
                                    <div class="spinner"></div>
                                    <p>Running tool...</p>
                                </div>
                                <div class="execution-result" style="display: none;">
                                    <h4>Result:</h4>
                                    <pre class="result-content"></pre>
                                </div>
                            </div>"#,
                            request.tool_name, execution_id
                        )
                    },
                    "encoding": "utf-8"
                }
            });

            (StatusCode::OK, Json(response)).into_response()
        }
        None => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": "MCP manager not initialized"
            }))).into_response()
        }
    }
}

/// Handle MCP UI action submissions
pub async fn handle_ui_action(
    State(state): State<Arc<AppState>>,
    Form(form): Form<McpUiActionForm>,
) -> impl IntoResponse {
    info!("Received UI action: {} for tool {} from service {}",
          form.action_type, form.tool_name, form.service_id);

    // Process different types of UI actions
    match form.action_type.as_str() {
        "execute" => {
            // Handle tool execution
            let response = json!({
                "status": "success",
                "message": "Tool execution started",
                "action_id": uuid::Uuid::new_v4().to_string()
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        "preview" => {
            // Handle tool preview
            let response = json!({
                "status": "success",
                "message": "Tool preview generated",
                "preview_url": format!("/mcp/ui/preview/{}/{}", form.service_id, form.tool_name)
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        "cancel" => {
            // Handle tool cancellation
            let response = json!({
                "status": "success",
                "message": "Tool execution cancelled"
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        _ => {
            (StatusCode::BAD_REQUEST, Json(json!({
                "error": "Unknown action type",
                "action_type": form.action_type
            }))).into_response()
        }
    }
}

/// Get execution status with live updates
pub async fn get_execution_status(
    State(_state): State<Arc<AppState>>,
    Path(execution_id): Path<String>,
) -> impl IntoResponse {
    // For now, simulate execution status
    // In a real implementation, this would query the actual execution state
    let status = json!({
        "execution_id": execution_id,
        "status": "completed",
        "started_at": chrono::Utc::now().to_rfc3339(),
        "completed_at": chrono::Utc::now().to_rfc3339(),
        "result": {
            "type": "success",
            "data": "Tool execution completed successfully"
        }
    });

    (StatusCode::OK, Json(status)).into_response()
}

/// Create UI resource from tool definition
fn create_ui_resource_from_tool(tool: &RegisteredTool, service_id: &str) -> Value {
    json!({
        "uri": format!("ui://tool/{}/{}", service_id, tool.name),
        "mimeType": "text/html",
        "content": {
            "type": "remoteDom",
            "html": format!(
                r#"<div class="mcp-tool-interface">
                    <div class="tool-header">
                        <h3>{}</h3>
                        <p class="tool-description">{}</p>
                    </div>
                    <div class="tool-parameters">
                        <h4>Parameters:</h4>
                        <div class="parameter-form">
                            {}
                        </div>
                    </div>
                    <div class="tool-actions">
                        <button class="btn btn-primary execute-tool" data-service-id="{}" data-tool-name="{}">
                            Execute Tool
                        </button>
                        <button class="btn btn-secondary preview-tool" data-service-id="{}" data-tool-name="{}">
                            Preview
                        </button>
                    </div>
                    <div class="tool-output" style="display: none;">
                        <h4>Output:</h4>
                        <pre class="output-content"></pre>
                    </div>
                </div>"#,
                tool.name,
                tool.description,
                generate_parameter_form(&None), // No parameters available for now
                service_id,
                tool.name,
                service_id,
                tool.name
            )
        },
        "encoding": "utf-8"
    })
}

/// Generate HTML form for tool parameters
fn generate_parameter_form(parameters: &Option<Value>) -> String {
    if let Some(parameters) = parameters {
        if let Some(props) = parameters.get("properties").and_then(|p| p.as_object()) {
            let mut form_html = String::new();

            for (param_name, param_schema) in props {
                let param_type = param_schema.get("type").and_then(|t| t.as_str()).unwrap_or("string");
                let description = param_schema.get("description").and_then(|d| d.as_str()).unwrap_or("");
                let required = parameters.get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.contains(&Value::String(param_name.clone())))
                    .unwrap_or(false);

                form_html.push_str(&format!(
                    r#"<div class="parameter-field">
                        <label for="param-{}">{}</label>
                        <input type="{}" id="param-{}" name="{}" {} placeholder="{}" />
                        <small class="param-description">{}</small>
                    </div>"#,
                    param_name,
                    param_name,
                    match param_type {
                        "number" | "integer" => "number",
                        "boolean" => "checkbox",
                        _ => "text"
                    },
                    param_name,
                    param_name,
                    if required { "required" } else { "" },
                    description,
                    if required { "Required" } else { "Optional" }
                ));
            }

            form_html
        } else {
            "<p>No parameters required</p>".to_string()
        }
    } else {
        "<p>No parameters required</p>".to_string()
    }
}

/// MCP UI WebSocket endpoint for real-time updates (placeholder)
pub async fn mcp_ui_websocket(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Implement WebSocket for real-time tool execution updates
    (StatusCode::NOT_IMPLEMENTED, Json(json!({
        "error": "WebSocket endpoint not yet implemented"
    }))).into_response()
}
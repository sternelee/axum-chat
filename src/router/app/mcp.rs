use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tera::Context;
use tracing::{info, error, warn};

use crate::{AppState, mcp::practical::*, User};

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub timeout: u64,
    pub max_restarts: u32,
    pub auto_restart: bool,
    pub env: Value,
    pub tools: Vec<String>,
    pub permissions: Value,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct ServiceResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub timeout: u64,
    pub max_restarts: u32,
    pub auto_restart: bool,
    pub env: Value,
    pub tools: Vec<String>,
    pub permissions: Value,
    pub enabled: bool,
    pub status: Option<String>,
    pub r#type: String,
}

impl From<PracticalMcpServiceConfig> for ServiceResponse {
    fn from(config: PracticalMcpServiceConfig) -> Self {
        Self {
            id: config.id.clone(),
            name: config.name.clone(),
            description: Some(config.description),
            command: config.command,
            args: config.args,
            timeout: config.timeout,
            max_restarts: config.max_restarts,
            auto_restart: config.auto_restart,
            env: json!(config.env),
            tools: config.tools.clone(),
            permissions: json!({}), // Empty permissions for now
            enabled: config.enabled,
            status: None, // Would need to query the manager for real status
            r#type: "stdio".to_string(),
        }
    }
}

/// Render MCP configuration page
pub async fn mcp_config_page(
    State(state): State<Arc<crate::AppState>>,
    Extension(current_user): Extension<Option<User>>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut context = Context::new();
    context.insert("current_user", &current_user);

    // Render the MCP configuration page
    let mcp_view = state.tera.render("views/mcp_config.html", &context).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    // Wrap in main layout
    let mut main_context = Context::new();
    main_context.insert("view", &mcp_view);
    main_context.insert("current_user", &current_user);
    main_context.insert("with_footer", &true);

    let rendered = state.tera.render("views/main.html", &main_context).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;
    Ok(Html(rendered))
}

/// Get all MCP services
#[axum::debug_handler]
pub async fn get_services(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mcp_manager_option = {
        let guard = state.mcp_manager.lock().unwrap();
        guard.as_ref().cloned()
    };

    match mcp_manager_option {
        Some(manager) => {
            match manager.load_config().await {
                Ok(configs) => {
                    let services: Vec<ServiceResponse> = configs.into_iter()
                        .map(ServiceResponse::from)
                        .collect();
                    Json(services).into_response()
                }
                Err(e) => {
                    error!("Failed to load MCP services: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                        "error": "Failed to load MCP services"
                    }))).into_response()
                }
            }
        }
        None => {
            warn!("MCP manager not initialized");
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": "MCP manager not initialized"
            }))).into_response()
        }
    }
}

/// Create new MCP service
pub async fn create_service(
    State(_state): State<Arc<AppState>>,
    Json(_service_request): Json<ServiceRequest>,
) -> impl IntoResponse {
    // For now, return success but note that this would require updating the config file
    info!("Request to create MCP service");

    // TODO: Implement actual service creation by updating mcp.json
    (StatusCode::CREATED, Json(json!({
        "message": "Service creation requested",
        "id": "new-service"
    }))).into_response()
}

/// Update MCP service
pub async fn update_service(
    State(_state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
    Json(_service_request): Json<ServiceRequest>,
) -> impl IntoResponse {
    info!("Request to update MCP service: {}", service_id);

    // TODO: Implement actual service update by updating mcp.json
    (StatusCode::OK, Json(json!({
        "message": "Service update requested",
        "id": service_id
    }))).into_response()
}

/// Delete MCP service
pub async fn delete_service(
    State(_state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
) -> impl IntoResponse {
    info!("Request to delete MCP service: {}", service_id);

    // TODO: Implement actual service deletion by updating mcp.json
    (StatusCode::OK, Json(json!({
        "message": "Service deletion requested",
        "id": service_id
    }))).into_response()
}

/// Start MCP service
pub async fn start_service(
    State(_state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
) -> impl IntoResponse {
    // TODO: Implement actual service start logic
    info!("Request to start MCP service: {}", service_id);
    (StatusCode::OK, Json(json!({
        "message": "Service start requested",
        "id": service_id
    }))).into_response()
}

/// Stop MCP service
pub async fn stop_service(
    State(_state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
) -> impl IntoResponse {
    // TODO: Implement actual service stop logic
    info!("Request to stop MCP service: {}", service_id);
    (StatusCode::OK, Json(json!({
        "message": "Service stop requested",
        "id": service_id
    }))).into_response()
}

/// Get MCP service status
pub async fn get_service_status(
    State(_state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
) -> impl IntoResponse {
    // TODO: Implement actual status query
    (StatusCode::OK, Json(json!({
        "id": service_id,
        "status": "unknown",
        "uptime": null,
        "restart_count": 0,
        "last_error": null
    }))).into_response()
}

/// Get available tools from all MCP services
#[axum::debug_handler]
pub async fn get_tools(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mcp_manager_option = {
        let guard = state.mcp_manager.lock().unwrap();
        guard.as_ref().cloned()
    };

    match mcp_manager_option {
        Some(manager) => {
            let tools = manager.get_rustgpt_tools().await;
            (StatusCode::OK, Json(json!({
                "tools": tools
            }))).into_response()
        }
        None => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": "MCP manager not initialized"
            }))).into_response()
        }
    }
}
use axum::{
    extract::{Extension, Path as AxumPath, State},
    http::StatusCode,
    response::{Html, Json},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tera::Context;

use crate::data::model::{
    AgentWithProvider, CreateAgentRequest, UpdateAgentRequest,
};
use crate::{User, middleware::internal_error};

/// Render enhanced agents configuration page
pub async fn agents_list(
    State(state): State<Arc<crate::AppState>>,
    Extension(current_user): Extension<Option<User>>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut context = Context::new();
    context.insert("current_user", &current_user);

    // Render the agents configuration page
    let agents_view = state.tera.render("views/agents_config.html", &context).map_err(internal_error)?;

    // Wrap in main layout
    let mut main_context = Context::new();
    main_context.insert("view", &agents_view);
    main_context.insert("current_user", &current_user);
    main_context.insert("with_footer", &true);

    let rendered = state.tera.render("views/main.html", &main_context).map_err(internal_error)?;
    Ok(Html(rendered))
}

pub async fn api_agents_list(
    State(state): State<Arc<crate::AppState>>,
    Extension(current_user): Extension<Option<crate::User>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Get user ID from authentication
    let user_id = match current_user {
        Some(user) => user.id,
        None => return Err((StatusCode::UNAUTHORIZED, "Authentication required".to_string())),
    };

    let agents = state.chat_repo.get_agents_by_user(user_id).await.map_err(internal_error)?;
    let providers = state.chat_repo.get_all_providers().await.map_err(internal_error)?;

    let mut agents_with_providers = Vec::new();
    for agent in agents {
        if let Ok(Some(agent_with_provider)) = state.chat_repo.get_agent_with_provider(agent.id).await {
            agents_with_providers.push(agent_with_provider);
        }
    }

    let response = json!({
        "agents": agents_with_providers,
        "providers": providers
    });

    Ok(Json(response))
}

pub async fn api_get_agent(
    AxumPath(id): AxumPath<i64>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<AgentWithProvider>, (StatusCode, String)> {
    let agent = state.chat_repo.get_agent_with_provider(id).await.map_err(internal_error)?;
    match agent {
        Some(a) => Ok(Json(a)),
        None => Err((StatusCode::NOT_FOUND, "Agent not found".to_string())),
    }
}

pub async fn api_create_agent(
    State(state): State<Arc<crate::AppState>>,
    Extension(current_user): Extension<Option<crate::User>>,
    Json(request): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, String)> {
    // Get user ID from authentication
    let user_id = match current_user {
        Some(user) => user.id,
        None => return Err((StatusCode::UNAUTHORIZED, "Authentication required".to_string())),
    };

    eprintln!("=== AGENT CREATION DEBUG ===");
    eprintln!("User ID: {}", user_id);
    eprintln!("Agent Name: {}", request.name);
    eprintln!("Provider ID: {}", request.provider_id);
    eprintln!("Model Name: {}", request.model_name);
    eprintln!("=============================");

    // First check if the provider exists
    match state.chat_repo.get_provider_by_id(request.provider_id).await {
        Ok(Some(provider)) => {
            eprintln!("Provider found: {} ({})", provider.name, provider.id);
        }
        Ok(None) => {
            eprintln!("ERROR: Provider with ID {} does not exist!", request.provider_id);
            return Err((StatusCode::BAD_REQUEST, format!("Provider with ID {} does not exist", request.provider_id)));
        }
        Err(e) => {
            eprintln!("ERROR: Failed to check provider existence: {}", e);
            return Err(internal_error(e));
        }
    }

    match state.chat_repo.create_agent(user_id, request).await {
        Ok(id) => {
            eprintln!("Agent created successfully with ID: {}", id);
            Ok((
                StatusCode::CREATED,
                Json(json!({ "message": "Agent created successfully", "id": id })),
            ))
        },
        Err(e) => {
            eprintln!("ERROR: Failed to create agent: {}", e);
            Err(internal_error(e))
        },
    }
}

pub async fn api_update_agent(
    AxumPath(id): AxumPath<i64>,
    State(state): State<Arc<crate::AppState>>,
    Json(request): Json<UpdateAgentRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match state.chat_repo.update_agent(id, request).await {
        Ok(rows) if rows > 0 => Ok(Json(json!({ "message": "Agent updated successfully" }))),
        Ok(_) => Err((StatusCode::NOT_FOUND, "Agent not found".to_string())),
        Err(e) => Err(internal_error(e)),
    }
}

pub async fn api_delete_agent(
    AxumPath(id): AxumPath<i64>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match state.chat_repo.delete_agent(id).await {
        Ok(rows) if rows > 0 => Ok(Json(json!({ "message": "Agent deleted successfully" }))),
        Ok(_) => Err((StatusCode::NOT_FOUND, "Agent not found".to_string())),
        Err(e) => Err(internal_error(e)),
    }
}



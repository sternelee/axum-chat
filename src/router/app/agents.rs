use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::{Html, Json},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_cookies::Cookies;

use crate::data::model::{
    AgentWithProvider, CreateAgentRequest, ProviderModel, UpdateAgentRequest,
};
use crate::middleware::internal_error;

pub async fn agents_list(
    State(state): State<Arc<crate::AppState>>,
    _cookies: Cookies,
) -> Result<Html<String>, (StatusCode, String)> {
    let repo = &state.chat_repo;

    // TODO: Get user ID from authentication
    let user_id = 1; // Placeholder

    let agents = match repo.get_agents_by_user(user_id).await {
        Ok(a) => a,
        Err(e) => return Err(internal_error(e)),
    };

    let _providers = match repo.get_all_providers().await {
        Ok(p) => p,
        Err(e) => return Err(internal_error(e)),
    };

    let mut html = String::from(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Agents Management - RustGPT</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <link href="https://cdn.jsdelivr.net/npm/daisyui@4.4.19/dist/full.min.css" rel="stylesheet" type="text/css" />
</head>
<body class="bg-base-200">
    <div class="container mx-auto px-4 py-8">
        <div class="mb-6">
            <h1 class="text-3xl font-bold mb-2">Agents Management</h1>
            <p class="text-gray-600">Create and manage AI agents with different capabilities</p>
        </div>

        <div class="mb-6">
            <button onclick="showCreateModal()" class="btn btn-primary">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                </svg>
                Create Agent
            </button>
        </div>

        <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
"#);

    for agent in agents {
        let status_class = if agent.is_active { "success" } else { "error" };
        let status_text = if agent.is_active { "Active" } else { "Inactive" };
        let public_badge = if agent.public { r#"<span class="badge badge-info ml-2">Public</span>"# } else { "" };

        html.push_str(&format!(r#"
            <div class="card bg-base-100 shadow-xl">
                <div class="card-body">
                    <div class="flex justify-between items-start mb-2">
                        <h2 class="card-title text-lg">
                            <span class="text-2xl">{}</span>
                            {}
                        </h2>
                        <div class="badge badge-{}">{}</div>
                    </div>

                    <p class="text-gray-600 mb-4">{}</p>

                    <div class="text-sm space-y-1 mb-4">
                        <div><strong>Model:</strong> {}</div>
                        <div><strong>Provider:</strong> ID {}</div>
                        <div><strong>Category:</strong> {}</div>
                        <div class="flex flex-wrap gap-1 mt-2">
"#,
            agent.icon,
            public_badge,
            status_class,
            status_text,
            agent.description.unwrap_or_default(),
            agent.model_name,
            agent.provider_id,
            agent.category
        ));

        if agent.stream {
            html.push_str(r#"<span class="badge badge-outline badge-sm">Streaming</span>"#);
        }
        if agent.chat {
            html.push_str(r#"<span class="badge badge-outline badge-sm">Chat</span>"#);
        }
        if agent.image {
            html.push_str(r#"<span class="badge badge-outline badge-sm">Vision</span>"#);
        }
        if agent.tool {
            html.push_str(r#"<span class="badge badge-outline badge-sm">Tools</span>"#);
        }
        if agent.file {
            html.push_str(r#"<span class="badge badge-outline badge-sm">Files</span>"#);
        }

        html.push_str(&format!(r#"
                        </div>
                    </div>

                    <div class="card-actions justify-end">
                        <button onclick="editAgent({})" class="btn btn-sm btn-outline">Edit</button>
                        <button onclick="deleteAgent({})" class="btn btn-sm btn-error btn-outline">Delete</button>
                    </div>
                </div>
            </div>
"#, agent.id, agent.id));
    }

    html.push_str(r#"
        </div>
    </div>
"#);

    Ok(Html(html))
}

pub async fn api_agents_list(
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<Vec<AgentWithProvider>>, (StatusCode, String)> {
    // TODO: Get user ID from authentication
    let user_id = 1; // Placeholder
    let agents = state.chat_repo.get_agents_by_user(user_id).await.map_err(internal_error)?;

    let mut agents_with_providers = Vec::new();
    for agent in agents {
        if let Ok(Some(agent_with_provider)) = state.chat_repo.get_agent_with_provider(agent.id).await {
            agents_with_providers.push(agent_with_provider);
        }
    }

    Ok(Json(agents_with_providers))
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
    Json(request): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, String)> {
    // TODO: Get user ID from authentication
    let user_id = 1; // Placeholder

    match state.chat_repo.create_agent(user_id, request).await {
        Ok(id) => Ok((
            StatusCode::CREATED,
            Json(json!({ "message": "Agent created successfully", "id": id })),
        )),
        Err(e) => Err(internal_error(e)),
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

pub async fn api_provider_models(
    AxumPath(provider_id): AxumPath<i64>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<Vec<ProviderModel>>, (StatusCode, String)> {
    let models = state.chat_repo.get_models_by_provider(provider_id).await.map_err(internal_error)?;
    Ok(Json(models))
}
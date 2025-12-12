use axum::{
    routing::{get, post, put},
    Router,
};

use std::sync::Arc;

use crate::AppState;

mod home;
use home::app;
mod chat;
use chat::{
    approve_all_tools, approve_tool, chat, chat_add_message, chat_by_id, chat_generate,
    create_chat_with_agent, create_chat_with_agent_form, create_chat_with_provider,
    create_chat_with_provider_form, delete_chat, new_chat, reject_tool,
};
mod auth;
use auth::{form_signup, login, login_form, logout, signup};
mod blog;
use blog::{blog, blog_by_slug};
mod settings;
use settings::{settings, settings_openai_api_key, settings_syntax_theme, theme_preview};
mod error;
use error::error;
mod providers;
use providers::{
    api_create_provider, api_delete_provider, api_get_provider, api_providers_list,
    api_update_provider, api_get_provider_models, providers_list,
};
mod agents;
use agents::{
    agents_list, api_agents_list, api_create_agent, api_delete_agent, api_get_agent,
    api_update_agent,
};
mod mcp;
use mcp::{
    create_service, delete_service, get_service_status, get_services, get_tools, mcp_config_page,
    start_service, stop_service, update_service,
};
mod mcp_ui;
use mcp_ui::{
    execute_tool_with_ui, get_execution_status, get_ui_resource, handle_ui_action, mcp_ui_page,
};

use crate::middleware::auth;

pub fn app_router(state: Arc<AppState>) -> Router {
    // Page routes (no /api/ prefix)
    let chat_router = Router::new()
        .route("/", get(chat).post(new_chat))
        .route("/{id}", get(chat_by_id).delete(delete_chat))
        .route("/agent/{id}", get(create_chat_with_agent_form).post(create_chat_with_agent))
        .route("/provider/{id}", get(create_chat_with_provider_form).post(create_chat_with_provider))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(auth));

    let settings_router = Router::new()
        .route("/", get(settings))
        .route("/", post(settings_openai_api_key))
        .route("/theme", post(settings_syntax_theme))
        .route("/theme/preview", post(theme_preview))
        .layer(axum::middleware::from_fn(auth));

    let providers_router = Router::new()
        .route("/", get(providers_list))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(auth));

    let agents_router = Router::new()
        .route("/", get(agents_list))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(auth));

    let mcp_router = Router::new()
        .route("/", get(mcp_config_page))
        .route("/ui", get(mcp_ui_page))
        .route(
            "/ui/resource/{service_id}/{tool_name}",
            get(get_ui_resource),
        )
        .route("/ui/execute", post(execute_tool_with_ui))
        .route("/ui/action", post(handle_ui_action))
        .route("/ui/execution/{execution_id}", get(get_execution_status))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(auth));

    // API routes (all with /api/ prefix)
    let api_router = Router::new()
        // Chat API endpoints
        .route("/chat/{id}/message/add", post(chat_add_message))
        .route("/chat/{id}/generate", get(chat_generate))
        // Providers API endpoints
        .route("/providers", get(api_providers_list).post(api_create_provider))
        .route(
            "/providers/{id}",
            get(api_get_provider)
                .put(api_update_provider)
                .delete(api_delete_provider),
        )
        // Agents API endpoints
        .route("/agents", get(api_agents_list).post(api_create_agent))
        .route(
            "/agents/{id}",
            get(api_get_agent)
                .put(api_update_agent)
                .delete(api_delete_agent),
        )
        // MCP API endpoints
        .route("/mcp/services", get(get_services).post(create_service))
        .route("/mcp/services/{id}", put(update_service).delete(delete_service))
        .route("/mcp/services/{id}/start", post(start_service))
        .route("/mcp/services/{id}/stop", post(stop_service))
        .route("/mcp/services/{id}/status", get(get_service_status))
        .route("/mcp/tools", get(get_tools))
        // Tool approval endpoints
        .route("/approve-tool", post(approve_tool))
        .route("/reject-tool", post(reject_tool))
        .route("/approve-all-tools", post(approve_all_tools))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(auth));

    // Public API endpoints (no auth required)
    let public_api_router = Router::new()
        .route("/providers/{id}/models", get(api_get_provider_models))
        .with_state(state.clone());

    Router::new()
        // Static page routes
        .route("/", get(app))
        .route("/error", get(error))
        .route("/login", get(login).post(login_form))
        .route("/signup", get(signup).post(form_signup))
        .route("/logout", get(logout))
        .route("/blog", get(blog))
        .route("/blog/{slug}", get(blog_by_slug))
        // Page routers
        .nest("/chat", chat_router)
        .nest("/settings", settings_router)
        .nest("/providers", providers_router)
        .nest("/agents", agents_router)
        .nest("/mcp", mcp_router)
        // API routers
        .nest("/api", api_router)
        .nest("/api", public_api_router)
        .with_state(state.clone())
}

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use std::sync::Arc;

use crate::{middleware::valid_openai_api_key, AppState};

mod home;
use home::app;
mod chat;
use chat::{chat, chat_add_message, chat_by_id, chat_generate, delete_chat, new_chat, approve_tool, reject_tool, approve_all_tools};
mod auth;
use auth::{form_signup, login, login_form, logout, signup};
mod blog;
use blog::{blog, blog_by_slug};
mod settings;
use settings::{settings, settings_openai_api_key, change_password};
mod error;
use error::error;
mod providers;
use providers::{
    api_create_provider, api_delete_provider, api_get_provider, api_providers_list, api_update_provider,
    providers_list,
};
mod agents;
use agents::{
    agents_list, api_agents_list, api_create_agent, api_delete_agent, api_get_agent, api_update_agent,
    api_provider_models,
};
mod mcp;
use mcp::{
    mcp_config_page, get_services, create_service, update_service, delete_service, start_service,
    stop_service, get_service_status, get_tools,
};

use crate::middleware::auth;

pub fn app_router(state: Arc<AppState>) -> Router {
    let chat_router = Router::new()
        .route("/", get(chat).post(new_chat))
        .route("/{id}", get(chat_by_id).delete(delete_chat))
        .route("/{id}/message/add", post(chat_add_message))
        .route("/{id}/generate", get(chat_generate))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(valid_openai_api_key))
        .layer(axum::middleware::from_fn(auth));

    let settings_router = Router::new()
        .route("/", get(settings).post(settings_openai_api_key))
        .route("/password", post(change_password))
        .layer(axum::middleware::from_fn(auth));

    let providers_router = Router::new()
        .route("/", get(providers_list))
        .route("/api", get(api_providers_list).post(api_create_provider))
        .route("/api/{id}", get(api_get_provider).put(api_update_provider).delete(api_delete_provider))
        .route("/api/{id}/models", get(api_provider_models))
        .with_state(state.clone());

    let agents_router = Router::new()
        .route("/", get(agents_list))
        .route("/api", get(api_agents_list).post(api_create_agent))
        .route("/api/{id}", get(api_get_agent).put(api_update_agent).delete(api_delete_agent))
        .with_state(state.clone());

    let mcp_router = Router::new()
        .route("/", get(mcp_config_page))
        .route("/api/services", get(get_services))
        .route("/api/services", post(create_service))
        .route("/api/services/{id}", put(update_service))
        .route("/api/services/{id}", delete(delete_service))
        .route("/api/services/{id}/start", post(start_service))
        .route("/api/services/{id}/stop", post(stop_service))
        .route("/api/services/{id}/status", get(get_service_status))
        .route("/api/tools", get(get_tools))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(auth));

    let api_router = Router::new()
        .route("/approve-tool", post(approve_tool))
        .route("/reject-tool", post(reject_tool))
        .route("/approve-all-tools", post(approve_all_tools))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(auth));

    Router::new()
        .route("/", get(app))
        .route("/error", get(error))
        .route("/login", get(login).post(login_form))
        .route("/signup", get(signup).post(form_signup))
        .route("/logout", get(logout))
        .route("/blog", get(blog))
        .route("/blog/{slug}", get(blog_by_slug))
        .nest("/chat", chat_router)
        .nest("/settings", settings_router)
        .nest("/providers", providers_router)
        .nest("/agents", agents_router)
        .nest("/mcp", mcp_router)
        .nest("/api", api_router)
        .with_state(state.clone())
}

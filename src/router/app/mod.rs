use axum::{
    routing::{get, post},
    Router,
};

use std::sync::Arc;

use crate::AppState;

mod home;
use home::app;
mod chat;
use chat::{
    chat, chat_add_message, chat_by_id, chat_generate, confirm_tool_call, delete_chat, new_chat,
    reject_tool_call,
};
mod auth;
use auth::{form_signup, login, login_form, logout, signup};
mod settings;
use settings::{
    delete_mcp_server, mcp_settings, restart_mcp_server, settings, settings_openai_api_key,
    update_mcp_settings,
};
mod a2ui;
use a2ui::{a2ui_example, generate_a2ui};
mod error;
use error::error;

use crate::middleware::auth;

pub fn app_router(state: Arc<AppState>) -> Router {
    let chat_router = Router::new()
        .route("/", get(chat).post(new_chat))
        .route("/{id}", get(chat_by_id).delete(delete_chat))
        .route("/{id}/message/add", post(chat_add_message))
        .route("/{id}/generate", get(chat_generate))
        .route(
            "/{id}/tool-confirm/{confirmation_id}",
            post(confirm_tool_call),
        )
        .route(
            "/{id}/tool-reject/{confirmation_id}",
            post(reject_tool_call),
        )
        .with_state(state.clone())
        .layer(axum::middleware::from_fn(auth));

    let settings_router = Router::new()
        .route("/", get(settings).post(settings_openai_api_key))
        .route("/mcp", get(mcp_settings))
        .route("/mcp/update", post(update_mcp_settings))
        .route("/mcp/delete", post(delete_mcp_server))
        .route("/mcp/restart", post(restart_mcp_server))
        .layer(axum::middleware::from_fn(auth));

    Router::new()
        .route("/", get(app))
        .route("/error", get(error))
        .route("/login", get(login).post(login_form))
        .route("/signup", get(signup).post(form_signup))
        .route("/logout", get(logout))
        .route("/demo", get(demo))
        .route("/demo-file-voice", get(demo_file_voice))
        .route("/demo-multi-turn", get(demo_multi_turn))
        .route("/demo-loading", get(demo_loading))
        .route("/api/a2ui", post(generate_a2ui))
        .route("/api/a2ui/example", get(a2ui_example))
        .nest("/chat", chat_router)
        .nest("/settings", settings_router)
        .with_state(state.clone())
}

async fn demo(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Html<String> {
    let rendered = state
        .tera
        .render("views/demo_extended_features.html", &tera::Context::new())
        .unwrap_or_else(|e| format!("Error rendering demo page: {}", e));
    axum::response::Html(rendered)
}

async fn demo_file_voice(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Html<String> {
    let rendered = state
        .tera
        .render("views/demo_file_voice.html", &tera::Context::new())
        .unwrap_or_else(|e| format!("Error rendering demo page: {}", e));
    axum::response::Html(rendered)
}

async fn demo_multi_turn(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Html<String> {
    let rendered = state
        .tera
        .render("views/demo_multi_turn.html", &tera::Context::new())
        .unwrap_or_else(|e| format!("Error rendering demo page: {}", e));
    axum::response::Html(rendered)
}

async fn demo_loading(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Html<String> {
    let rendered = state
        .tera
        .render("views/demo_loading.html", &tera::Context::new())
        .unwrap_or_else(|e| format!("Error rendering demo page: {}", e));
    axum::response::Html(rendered)
}

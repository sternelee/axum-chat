use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{Html, Redirect, Json},
    Form,
};

use serde::{Deserialize, Serialize};
use tera::Context;

use std::sync::Arc;
use std::collections::HashMap;

use crate::{AppState, User};
use crate::mcp::{get_mcp_manager, McpServerConfig};

#[derive(Deserialize, Debug)]
pub struct AISettings {
    api_key: String,
    base_url: Option<String>,
    model: Option<String>,
    system_prompt: Option<String>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_tokens: Option<i64>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct McpServerSettings {
    pub name: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub disabled: Option<bool>,
    pub timeout: Option<u64>,
    pub description: Option<String>,
    pub transport: Option<String>,
    pub url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
pub struct McpSettingsResponse {
    pub servers: HashMap<String, McpServerSettings>,
    pub connected_servers: Vec<String>,
    pub available_tools: Vec<String>,
}

#[axum::debug_handler]
pub async fn settings_openai_api_key(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
    Form(ai_settings): Form<AISettings>,
) -> Result<Redirect, StatusCode> {
    let id = current_user.unwrap().id;

    // Default values for optional fields
    let base_url = ai_settings
        .base_url
        .as_deref()
        .unwrap_or("https://api.siliconflow.cn/v1");
    let model = ai_settings
        .model
        .as_deref()
        .unwrap_or("Qwen/Qwen2.5-7B-Instruct");
    let system_prompt = ai_settings
        .system_prompt
        .as_deref()
        .unwrap_or("You are a helpful assistant.");
    let temperature = ai_settings.temperature.unwrap_or(0.7);
    let top_p = ai_settings.top_p.unwrap_or(1.0);
    let max_tokens = ai_settings.max_tokens.unwrap_or(2000);

    sqlx::query(r#"
        INSERT INTO settings (user_id, openai_api_key, base_url, model, system_prompt, temperature, top_p, max_tokens)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT (user_id) DO UPDATE SET
            openai_api_key = excluded.openai_api_key,
            base_url = excluded.base_url,
            model = excluded.model,
            system_prompt = excluded.system_prompt,
            temperature = excluded.temperature,
            top_p = excluded.top_p,
            max_tokens = excluded.max_tokens
    "#)
        .bind(id)
        .bind(&ai_settings.api_key)
        .bind(base_url)
        .bind(model)
        .bind(system_prompt)
        .bind(temperature)
        .bind(top_p)
        .bind(max_tokens)
        .execute(&*state.pool)
        .await
        .unwrap();

    Ok(Redirect::to("/settings"))
}

#[axum::debug_handler]
pub async fn mcp_settings(
    Extension(current_user): Extension<Option<User>>,
) -> Result<Json<McpSettingsResponse>, StatusCode> {
    let _user = current_user.as_ref().unwrap();

    let mcp_manager = get_mcp_manager();

    // Get server configurations
    let server_configs = mcp_manager.get_server_configs().await;
    let mut servers = HashMap::new();

    for (name, config) in server_configs {
        let server_settings = McpServerSettings {
            name: name.clone(),
            command: config.command,
            args: config.args,
            env: config.env,
            disabled: config.disabled,
            timeout: config.timeout,
            description: config.description,
            transport: config.transport.map(|t| format!("{:?}", t).to_lowercase()),
            url: config.url,
            headers: config.headers,
        };
        servers.insert(name, server_settings);
    }

    // Get connected servers
    let connected_servers = mcp_manager.get_connected_servers().await;

    // Get available tools
    let tools = mcp_manager.get_all_tools().await;
    let available_tools = tools.into_iter().map(|tool| tool.name).collect();

    Ok(Json(McpSettingsResponse {
        servers,
        connected_servers,
        available_tools,
    }))
}

#[axum::debug_handler]
pub async fn update_mcp_settings(
    Form(settings): Form<McpServerSettings>,
) -> Result<Redirect, StatusCode> {
    let mcp_manager = get_mcp_manager();

    // Convert settings to McpServerConfig
    let transport = match settings.transport.as_deref() {
        Some("stdio") => Some(crate::mcp::TransportType::Stdio),
        Some("sse") => Some(crate::mcp::TransportType::Sse),
        Some("http") => Some(crate::mcp::TransportType::Http),
        _ => None,
    };

    let server_config = McpServerConfig {
        command: settings.command,
        args: settings.args,
        env: settings.env,
        disabled: settings.disabled,
        timeout: settings.timeout,
        description: settings.description,
        transport,
        url: settings.url,
        headers: settings.headers,
    };

    // Add/update server configuration
    mcp_manager.add_server_config(settings.name.clone(), server_config).await;

    // Save configuration to file
    let mcp_config_path = std::path::PathBuf::from("mcp.json");
    if let Err(e) = mcp_manager.save_config(&mcp_config_path).await {
        eprintln!("Failed to save MCP configuration: {}", e);
    }

    Ok(Redirect::to("/settings"))
}

#[axum::debug_handler]
pub async fn delete_mcp_server(
    Form(settings): Form<McpServerSettings>,
) -> Result<Redirect, StatusCode> {
    let mcp_manager = get_mcp_manager();

    // Remove server configuration
    mcp_manager.remove_server_config(&settings.name).await;

    // Shutdown the server if it's running
    if let Err(e) = mcp_manager.shutdown_server(&settings.name).await {
        eprintln!("Error shutting down server {}: {}", settings.name, e);
    }

    // Save configuration to file
    let mcp_config_path = std::path::PathBuf::from("mcp.json");
    if let Err(e) = mcp_manager.save_config(&mcp_config_path).await {
        eprintln!("Failed to save MCP configuration: {}", e);
    }

    Ok(Redirect::to("/settings"))
}

#[axum::debug_handler]
pub async fn restart_mcp_server(
    Form(settings): Form<McpServerSettings>,
) -> Result<Redirect, StatusCode> {
    let mcp_manager = get_mcp_manager();

    // Get server configuration
    let server_configs = mcp_manager.get_server_configs().await;
    if let Some(server_config) = server_configs.get(&settings.name) {
        // Shutdown the server if it's running
        mcp_manager.shutdown_server(&settings.name).await.ok();

        // Restart the server
        if let Err(e) = mcp_manager.initialize_server(settings.name.clone(), server_config).await {
            eprintln!("Failed to restart MCP server {}: {}", settings.name, e);
        }
    }

    Ok(Redirect::to("/settings"))
}

#[axum::debug_handler]
pub async fn settings(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
) -> Result<Html<String>, StatusCode> {
    let user = current_user.as_ref().unwrap();

    let mut context = Context::new();
    context.insert("openai_api_key", &user.openai_api_key);
    context.insert("base_url", &user.base_url);
    context.insert("model", &user.model);
    context.insert("system_prompt", &user.system_prompt);
    context.insert("temperature", &user.temperature);
    context.insert("top_p", &user.top_p);
    context.insert("max_tokens", &user.max_tokens);

    let settings = state.tera.render("views/settings.html", &context).unwrap();

    let mut context = Context::new();
    context.insert("view", &settings);
    context.insert("current_user", &current_user);
    context.insert("with_footer", &true);
    let rendered = state.tera.render("views/main.html", &context).unwrap();

    Ok(Html(rendered))
}

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{Html, Redirect},
    Form,
};

use serde::Deserialize;
use tera::Context;

use std::sync::Arc;

use crate::{AppState, User};

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

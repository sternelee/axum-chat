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
pub struct OpenAiAPIKey {
    api_key: String,
}

#[derive(Deserialize, Debug)]
pub struct SyntaxThemeSettings {
    syntax_theme: String,
    code_line_numbers: Option<String>, // HTML checkbox sends "on" or nothing
    code_wrap_lines: Option<String>,   // HTML checkbox sends "on" or nothing
    enhanced_markdown: Option<String>, // HTML checkbox sends "on" or nothing
}

#[derive(Deserialize, Debug)]
pub struct ThemePreviewRequest {
    theme: String,
    sample_code: Option<String>,
}

#[axum::debug_handler]
pub async fn settings_openai_api_key(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
    Form(set_openai_api_key): Form<OpenAiAPIKey>,
) -> Result<Redirect, StatusCode> {
    let id = current_user.unwrap().id;

      let _ = state.db.execute(
        "INSERT INTO settings (user_id, openai_api_key) VALUES (?, ?) ON CONFLICT (user_id) DO UPDATE SET openai_api_key = ?",
        vec![
            serde_json::Value::Number(id.into()),
            serde_json::Value::String(set_openai_api_key.api_key.clone()),
            serde_json::Value::String(set_openai_api_key.api_key),
        ],
    ).await
    .unwrap();

    Ok(Redirect::to("/settings"))
}

#[axum::debug_handler]
pub async fn settings_syntax_theme(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
    Form(theme_settings): Form<SyntaxThemeSettings>,
) -> Result<Redirect, StatusCode> {
    let user = current_user.unwrap();
    let id = user.id;

    // Convert checkbox strings to booleans
    let code_line_numbers = theme_settings.code_line_numbers.is_some();
    let code_wrap_lines = theme_settings.code_wrap_lines.is_some();
    let enhanced_markdown = theme_settings.enhanced_markdown.is_some();

    let _ = state.db.execute(
        "INSERT INTO settings (user_id, syntax_theme, code_line_numbers, code_wrap_lines, enhanced_markdown)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (user_id) DO UPDATE SET
         syntax_theme = excluded.syntax_theme,
         code_line_numbers = excluded.code_line_numbers,
         code_wrap_lines = excluded.code_wrap_lines,
         enhanced_markdown = excluded.enhanced_markdown",
        vec![
            serde_json::Value::Number(id.into()),
            serde_json::Value::String(theme_settings.syntax_theme),
            serde_json::Value::Bool(code_line_numbers),
            serde_json::Value::Bool(code_wrap_lines),
            serde_json::Value::Bool(enhanced_markdown),
        ],
    ).await
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
    context.insert("syntax_theme", &user.syntax_theme);
    context.insert("code_line_numbers", &user.code_line_numbers);
    context.insert("code_wrap_lines", &user.code_wrap_lines);
    context.insert("enhanced_markdown", &user.enhanced_markdown);

    // Get available themes from syntax highlighter
    let themes = vec![
        ("base16-ocean.dark", "Ocean Dark"),
        ("base16-eighties.dark", "Eighties Dark"),
        ("base16-mocha.dark", "Mocha Dark"),
        ("base16-tomorrow.dark", "Tomorrow Dark"),
        ("Solarized (dark)", "Solarized Dark"),
        ("base16-ocean.light", "Ocean Light"),
        ("base16-tomorrow.light", "Tomorrow Light"),
        ("InspiredGitHub", "GitHub Light"),
        ("Material", "Material"),
    ];
    context.insert("available_themes", &themes);

    let settings = state.tera.render("views/settings.html", &context).unwrap();

    let mut context = Context::new();
    context.insert("view", &settings);
    context.insert("current_user", &current_user);
    context.insert("with_footer", &true);
    let rendered = state.tera.render("views/main.html", &context).unwrap();

    Ok(Html(rendered))
}

#[axum::debug_handler]
pub async fn theme_preview(
    State(state): State<Arc<AppState>>,
    Form(preview_request): Form<ThemePreviewRequest>,
) -> Result<Html<String>, StatusCode> {
    let sample_code = preview_request.sample_code.unwrap_or_else(|| {
        r#"fn main() {
    let greeting = "Hello, World!";
    println!("{}", greeting);

    let numbers = vec![1, 2, 3, 4, 5];
    for num in numbers {
        println!("Number: {}", num);
    }
}"#.to_string()
    });

    // Use the enhanced markdown renderer with the requested theme
    let highlighted_code = crate::utils::highlight_code_with_theme(
        &sample_code,
        "rust",
        &preview_request.theme
    ).unwrap_or_else(|_| format!("Failed to highlight code with theme: {}", preview_request.theme));

    let mut context = Context::new();
    context.insert("highlighted_code", &highlighted_code);
    context.insert("theme_name", &preview_request.theme);
    context.insert("sample_code", &sample_code);

    let preview = state.tera.render("partials/theme_preview.html", &context).unwrap();
    Ok(Html(preview))
}

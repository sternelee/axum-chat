use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form, Json,
};

use serde::Deserialize;
use tera::Context;
use tower_cookies::{Cookie, Cookies};

use std::sync::Arc;

use crate::{AppState, User};

#[derive(Deserialize)]
pub struct LoginQuery {
    error: Option<String>,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>
) -> Html<String> {
    let mut context = Context::new();
    context.insert("name", "World");

    // Pass error parameter to template if present
    if let Some(error) = query.error {
        context.insert("error", &error);
    }

    let home = state.tera.render("views/login.html", &context).unwrap();

    let mut context = Context::new();
    context.insert("view", &home);
    let rendered = state.tera.render("views/main.html", &context).unwrap();

    Html(rendered)
}

#[derive(Debug)]
pub enum LogInError {
    InvalidCredentials,
    DatabaseError(String),
}

impl IntoResponse for LogInError {
    fn into_response(self) -> Response {
        match self {
            LogInError::InvalidCredentials => {
                // Redirect to login page with error message instead of JSON response
                let error_url = "/login?error=invalid_credentials";
                let redirect = axum::response::Redirect::to(error_url);
                let mut response = redirect.into_response();
                // Add HTMX redirect header if the request is from HTMX
                response.headers_mut().insert("HX-Redirect", error_url.parse().unwrap());
                response
            }
            LogInError::DatabaseError(message) => {
                let error_url = &format!("/login?error=database_error");
                let redirect = axum::response::Redirect::to(error_url);
                let mut response = redirect.into_response();
                response.headers_mut().insert("HX-Redirect", error_url.parse().unwrap());
                response
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct LogIn {
    email: String,
    password: String,
}

#[axum::debug_handler]
pub async fn login_form(
    cookies: Cookies,
    state: State<Arc<AppState>>,
    Form(log_in): Form<LogIn>,
) -> Result<Redirect, LogInError> {
    // Verify password using libsql
    let result = state.db.query(
        "SELECT users.id, users.email, users.password, users.created_at,
                settings.openai_api_key,
                COALESCE(settings.syntax_theme, 'base16-ocean.dark') as syntax_theme,
                COALESCE(settings.code_line_numbers, 1) as code_line_numbers,
                COALESCE(settings.code_wrap_lines, 0) as code_wrap_lines,
                COALESCE(settings.enhanced_markdown, 1) as enhanced_markdown
         FROM users LEFT JOIN settings ON settings.user_id=users.id WHERE users.email = ?",
        vec![serde_json::Value::String(log_in.email)],
    ).await
    .map_err(|e| LogInError::DatabaseError(e))?;

    if result.rows.is_empty() {
        return Err(LogInError::InvalidCredentials);
    }

    let row = &result.rows[0];
    let user = User {
        id: row["id"].as_i64().unwrap_or(0),
        email: row["email"].as_str().unwrap_or("").to_string(),
        password: row["password"].as_str().unwrap_or("").to_string(),
        created_at: row["created_at"].as_str().unwrap_or("").to_string(),
        openai_api_key: row["openai_api_key"].as_str().map(|s| s.to_string()),
        syntax_theme: row["syntax_theme"].as_str().unwrap_or("base16-ocean.dark").to_string(),
        code_line_numbers: row["code_line_numbers"].as_bool().unwrap_or(true),
        code_wrap_lines: row["code_wrap_lines"].as_bool().unwrap_or(false),
        enhanced_markdown: row["enhanced_markdown"].as_bool().unwrap_or(true),
    };

    if user.password != log_in.password {
        return Err(LogInError::InvalidCredentials);
    }

    let cookie = Cookie::build(("rust-gpt-session", user.id.to_string()))
        .path("/")
        .http_only(true)
        .build();
    cookies.add(cookie);

    Ok(Redirect::to("/"))
}

pub async fn signup(State(state): State<Arc<AppState>>) -> Html<String> {
    // TODO: Hash password
    let mut context = Context::new();
    context.insert("name", "World");
    let home = state.tera.render("views/signup.html", &context).unwrap();

    let mut context = Context::new();
    context.insert("view", &home);
    let rendered = state.tera.render("views/main.html", &context).unwrap();

    Html(rendered)
}

#[derive(Debug)]
pub enum SignUpError {
    PasswordMismatch,
    DatabaseError(String),
}

impl IntoResponse for SignUpError {
    fn into_response(self) -> Response {
        match self {
            SignUpError::PasswordMismatch => {
                (StatusCode::BAD_REQUEST, Json("Passwords do not match.")).into_response()
            }
            SignUpError::DatabaseError(message) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(message)).into_response()
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct SignUp {
    email: String,
    password: String,
    password_confirmation: String,
}

#[axum::debug_handler]
pub async fn form_signup(
    state: State<Arc<AppState>>,
    Form(sign_up): Form<SignUp>,
) -> Result<Redirect, SignUpError> {
    if sign_up.password != sign_up.password_confirmation {
        return Err(SignUpError::PasswordMismatch);
    }

    // insert into db using libsql
    let result = state.db.execute(
        "INSERT INTO users (email, password) VALUES (?, ?)",
        vec![
            serde_json::Value::String(sign_up.email),
            serde_json::Value::String(sign_up.password),
        ],
    ).await
    .map_err(|e| SignUpError::DatabaseError(e))?;

    if result.rows_affected > 0 {
        Ok(Redirect::to("/login"))
    } else {
        Err(SignUpError::DatabaseError("Failed to insert user".to_string()))
    }
}

#[axum::debug_handler]
pub async fn logout(cookies: Cookies) -> Result<Redirect, StatusCode> {
    let mut cookie = Cookie::build(("rust-gpt-session", ""))
        .path("/")
        .http_only(true)
        .build();
    cookie.make_removal();

    cookies.add(cookie);

    Ok(Redirect::to("/"))
}

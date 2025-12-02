use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form, Json,
};

use serde::Deserialize;
use tera::Context;
use tower_cookies::{Cookie, Cookies};

use std::sync::Arc;

use crate::{AppState, User};
use crate::utils::{hash_password, verify_password, PasswordError};

pub async fn login(State(state): State<Arc<AppState>>) -> Html<String> {
    let mut context = Context::new();
    context.insert("name", "World");
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
            LogInError::InvalidCredentials => (
                StatusCode::BAD_REQUEST,
                Json("Invalid Username or Password"),
            )
                .into_response(),
            LogInError::DatabaseError(message) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(message)).into_response()
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct LogIn {
    email: String,
    password: String,
}

impl LogIn {
    fn validate(&self) -> Result<(), &'static str> {
        if self.email.is_empty() {
            return Err("Email cannot be empty");
        }
        
        if self.password.is_empty() {
            return Err("Password cannot be empty");
        }
        
        if !self.email.contains('@') {
            return Err("Invalid email format");
        }
        
        Ok(())
    }
}

#[axum::debug_handler]
pub async fn login_form(
    cookies: Cookies,
    state: State<Arc<AppState>>,
    Form(log_in): Form<LogIn>,
) -> Result<Redirect, LogInError> {
    log_in.validate().map_err(|_| LogInError::InvalidCredentials)?;
    
    let user = sqlx::query_as!(
        User,
        "SELECT users.*, settings.openai_api_key FROM users LEFT JOIN settings ON settings.user_id=users.id WHERE users.email = $1",
        log_in.email,
    ).fetch_one(&*state.pool).await
    .map_err(|_| LogInError::InvalidCredentials)?;

    match verify_password(&log_in.password, &user.password) {
        Ok(true) => {
            let cookie = Cookie::build(("rust-gpt-session", user.id.to_string()))
                .path("/")
                .http_only(true)
                .build();
            cookies.add(cookie);
            Ok(Redirect::to("/"))
        }
        Ok(false) => Err(LogInError::InvalidCredentials),
        Err(_) => Err(LogInError::InvalidCredentials),
    }
}

pub async fn signup(State(state): State<Arc<AppState>>) -> Html<String> {
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

impl SignUp {
    fn validate(&self) -> Result<(), &'static str> {
        if self.email.is_empty() {
            return Err("Email cannot be empty");
        }
        
        if !self.email.contains('@') {
            return Err("Invalid email format");
        }
        
        if self.email.len() > 255 {
            return Err("Email is too long");
        }
        
        if self.password.len() < 8 {
            return Err("Password must be at least 8 characters long");
        }
        
        if self.password.len() > 255 {
            return Err("Password is too long");
        }
        
        Ok(())
    }
}

#[axum::debug_handler]
pub async fn form_signup(
    state: State<Arc<AppState>>,
    Form(sign_up): Form<SignUp>,
) -> Result<Redirect, SignUpError> {
    sign_up.validate().map_err(|_| SignUpError::DatabaseError("Validation failed".to_string()))?;
    
    if sign_up.password != sign_up.password_confirmation {
        return Err(SignUpError::PasswordMismatch);
    }

    let hashed_password = hash_password(&sign_up.password)
        .map_err(|_| SignUpError::DatabaseError("Failed to hash password".to_string()))?;

    match sqlx::query!(
        "INSERT INTO users (email, password) VALUES ($1, $2) RETURNING id",
        sign_up.email,
        hashed_password
    )
    .fetch_one(&*state.pool)
    .await
    {
        Ok(_) => Ok(Redirect::to("/login")),
        Err(_e) => {
            Err(SignUpError::DatabaseError("Email already registered".to_string()))
        }
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

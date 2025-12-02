use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
    Json,
};

use serde::{Deserialize, Serialize};
use tera::Context;

use std::sync::Arc;

use crate::{AppState, User};
use crate::utils::{hash_password, verify_password};

#[derive(Deserialize, Debug)]
pub struct OpenAiAPIKey {
    api_key: String,
}

#[derive(Deserialize, Debug)]
pub struct ChangePassword {
    current_password: String,
    new_password: String,
    confirm_password: String,
}

#[derive(Serialize)]
pub struct ApiKeyResponse {
    message: String,
    success: bool,
}

#[derive(Debug)]
pub enum SettingsError {
    InvalidApiKey,
    DatabaseError,
    Unauthorized,
    InvalidPassword,
    PasswordMismatch,
    WeakPassword,
}

impl IntoResponse for SettingsError {
    fn into_response(self) -> Response {
        match self {
            SettingsError::InvalidApiKey => {
                (StatusCode::BAD_REQUEST, Json("Invalid API key format")).into_response()
            }
            SettingsError::DatabaseError => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json("Failed to save settings")).into_response()
            }
            SettingsError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, Json("Not authenticated")).into_response()
            }
            SettingsError::InvalidPassword => {
                (StatusCode::BAD_REQUEST, Json("Current password is incorrect")).into_response()
            }
            SettingsError::PasswordMismatch => {
                (StatusCode::BAD_REQUEST, Json("Passwords do not match")).into_response()
            }
            SettingsError::WeakPassword => {
                (StatusCode::BAD_REQUEST, Json("Password must be at least 8 characters long")).into_response()
            }
        }
    }
}

#[axum::debug_handler]
pub async fn settings_openai_api_key(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
    Form(set_openai_api_key): Form<OpenAiAPIKey>,
) -> Result<Redirect, SettingsError> {
    let user = current_user.ok_or(SettingsError::Unauthorized)?;

    if set_openai_api_key.api_key.is_empty() {
        return Err(SettingsError::InvalidApiKey);
    }

    if set_openai_api_key.api_key.len() < 20 {
        return Err(SettingsError::InvalidApiKey);
    }

    sqlx::query("INSERT INTO settings (user_id, openai_api_key) VALUES (?, ?) ON CONFLICT (user_id) DO UPDATE SET openai_api_key = ?")
        .bind(user.id)
        .bind(&set_openai_api_key.api_key)
        .bind(&set_openai_api_key.api_key)
        .execute(&*state.pool)
        .await
        .map_err(|_| SettingsError::DatabaseError)?;

    Ok(Redirect::to("/settings"))
}

#[axum::debug_handler]
pub async fn change_password(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
    Form(change_password): Form<ChangePassword>,
) -> Result<Redirect, SettingsError> {
    let user = current_user.ok_or(SettingsError::Unauthorized)?;

    if change_password.new_password.len() < 8 {
        return Err(SettingsError::WeakPassword);
    }

    if change_password.new_password != change_password.confirm_password {
        return Err(SettingsError::PasswordMismatch);
    }

    verify_password(&change_password.current_password, &user.password)
        .ok()
        .and_then(|verified| if verified { Some(()) } else { None })
        .ok_or(SettingsError::InvalidPassword)?;

    let hashed_password = hash_password(&change_password.new_password)
        .map_err(|_| SettingsError::DatabaseError)?;

    sqlx::query("UPDATE users SET password = ? WHERE id = ?")
        .bind(&hashed_password)
        .bind(user.id)
        .execute(&*state.pool)
        .await
        .map_err(|_| SettingsError::DatabaseError)?;

    Ok(Redirect::to("/settings"))
}

#[axum::debug_handler]
pub async fn settings(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
) -> Result<Html<String>, StatusCode> {
    let key = current_user.as_ref().unwrap().openai_api_key.as_ref();

    let mut context = Context::new();
    context.insert("openai_api_key", &key);

    let settings = state.tera.render("views/settings.html", &context).unwrap();

    let mut context = Context::new();
    context.insert("view", &settings);
    context.insert("current_user", &current_user);
    context.insert("with_footer", &true);
    let rendered = state.tera.render("views/main.html", &context).unwrap();

    Ok(Html(rendered))
}

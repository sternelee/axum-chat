use axum::{
    extract::{Extension, Path as AxumPath, State},
    http::StatusCode,
    response::{Html, Json},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tera::Context;

use crate::data::model::{
    CreateProviderRequest, Provider, UpdateProviderRequest,
};
use crate::{User, middleware::internal_error};

/// Render enhanced providers configuration page
pub async fn providers_list(
    State(state): State<Arc<crate::AppState>>,
    Extension(current_user): Extension<Option<User>>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut context = Context::new();
    context.insert("current_user", &current_user);

    // Render the providers configuration page
    let providers_view = state.tera.render("views/providers_config.html", &context).map_err(internal_error)?;

    // Wrap in main layout
    let mut main_context = Context::new();
    main_context.insert("view", &providers_view);
    main_context.insert("current_user", &current_user);
    main_context.insert("with_footer", &true);

    let rendered = state.tera.render("views/main.html", &main_context).map_err(internal_error)?;
    Ok(Html(rendered))
}

pub async fn api_providers_list(
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<Vec<Provider>>, (StatusCode, String)> {
    let providers = state.chat_repo.get_all_providers().await.map_err(internal_error)?;
    Ok(Json(providers))
}

pub async fn api_get_provider(
    AxumPath(id): AxumPath<i64>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<Provider>, (StatusCode, String)> {
    let provider = state.chat_repo.get_provider_by_id(id).await.map_err(internal_error)?;
    match provider {
        Some(p) => Ok(Json(p)),
        None => Err((StatusCode::NOT_FOUND, "Provider not found".to_string())),
    }
}

pub async fn api_create_provider(
    State(state): State<Arc<crate::AppState>>,
    Json(request): Json<CreateProviderRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, String)> {
    match state.chat_repo.create_provider(request).await {
        Ok(id) => Ok((
            StatusCode::CREATED,
            Json(json!({ "message": "Provider created successfully", "id": id })),
        )),
        Err(e) => Err(internal_error(e)),
    }
}

pub async fn api_update_provider(
    AxumPath(id): AxumPath<i64>,
    State(state): State<Arc<crate::AppState>>,
    Json(request): Json<UpdateProviderRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match state.chat_repo.update_provider(id, request).await {
        Ok(rows) if rows > 0 => Ok(Json(json!({ "message": "Provider updated successfully" }))),
        Ok(_) => Err((StatusCode::NOT_FOUND, "Provider not found".to_string())),
        Err(e) => Err(internal_error(e)),
    }
}

pub async fn api_delete_provider(
    AxumPath(id): AxumPath<i64>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    match state.chat_repo.delete_provider(id).await {
        Ok(rows) if rows > 0 => Ok(Json(json!({ "message": "Provider deleted successfully" }))),
        Ok(_) => Err((StatusCode::NOT_FOUND, "Provider not found".to_string())),
        Err(e) => Err(internal_error(e)),
    }
}
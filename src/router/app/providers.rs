use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::{Html, Json},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_cookies::Cookies;

use crate::data::model::{
    CreateProviderRequest, Provider, ProviderType, UpdateProviderRequest,
};
use crate::middleware::internal_error;

pub async fn providers_list(
    State(state): State<Arc<crate::AppState>>,
    _cookies: Cookies,
) -> Result<Html<String>, (StatusCode, String)> {
    let repo = &state.chat_repo;

    let providers = match repo.get_all_providers().await {
        Ok(p) => p,
        Err(e) => return Err(internal_error(e)),
    };

    let mut html = String::from(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Providers Management - RustGPT</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <link href="https://cdn.jsdelivr.net/npm/daisyui@4.4.19/dist/full.min.css" rel="stylesheet" type="text/css" />
</head>
<body class="bg-base-200">
    <div class="container mx-auto px-4 py-8">
        <div class="mb-6">
            <h1 class="text-3xl font-bold mb-2">Providers Management</h1>
            <p class="text-gray-600">Manage AI service providers and their models</p>
        </div>

        <div class="mb-6">
            <button onclick="showCreateModal()" class="btn btn-primary">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                </svg>
                Add Provider
            </button>
        </div>

        <div class="overflow-x-auto">
            <table class="table table-zebra w-full">
                <thead>
                    <tr>
                        <th>Name</th>
                        <th>Type</th>
                        <th>Base URL</th>
                        <th>Models</th>
                        <th>Status</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
"#);

    for provider in providers {
        let status_class = if provider.is_active { "success" } else { "error" };
        let status_text = if provider.is_active { "Active" } else { "Inactive" };
        let provider_type = match provider.provider_type {
            ProviderType::OpenAI => "OpenAI",
            ProviderType::Gemini => "Gemini",
        };

        html.push_str(&format!(r#"
                    <tr>
                        <td class="font-medium">{}</td>
                        <td>{}</td>
                        <td class="text-sm">{}</td>
                        <td><a href="/providers/{}/models" class="link link-primary">View Models</a></td>
                        <td><div class="badge badge-{}">{}</div></td>
                        <td>
                            <div class="flex gap-2">
                                <button onclick="editProvider({})" class="btn btn-sm btn-outline">Edit</button>
                                <button onclick="deleteProvider({})" class="btn btn-sm btn-error btn-outline">Delete</button>
                            </div>
                        </td>
                    </tr>
"#,
            provider.name,
            provider_type,
            provider.base_url,
            provider.id,
            status_class,
            status_text,
            provider.id,
            provider.id
        ));
    }

    html.push_str(r#"
                </tbody>
            </table>
        </div>
    </div>

    <!-- Create/Edit Provider Modal -->
    <div id="providerModal" class="modal">
        <div class="modal-box">
            <h3 id="modalTitle" class="font-bold text-lg">Create Provider</h3>
            <form id="providerForm" class="py-4">
                <div class="form-control w-full mb-4">
                    <label class="label">
                        <span class="label-text">Provider Name</span>
                    </label>
                    <input type="text" id="name" name="name" class="input input-bordered w-full" required />
                </div>

                <div class="form-control w-full mb-4">
                    <label class="label">
                        <span class="label-text">Provider Type</span>
                    </label>
                    <select id="provider_type" name="provider_type" class="select select-bordered w-full" required>
                        <option value="openai">OpenAI Compatible</option>
                        <option value="gemini">Google Gemini</option>
                    </select>
                </div>

                <div class="form-control w-full mb-4">
                    <label class="label">
                        <span class="label-text">Base URL</span>
                    </label>
                    <input type="url" id="base_url" name="base_url" class="input input-bordered w-full" required />
                </div>

                <div class="form-control w-full mb-4">
                    <label class="label">
                        <span class="label-text">API Key</span>
                    </label>
                    <input type="password" id="api_key" name="api_key" class="input input-bordered w-full" required />
                </div>

                <div class="form-control w-full mb-4">
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input type="checkbox" id="is_active" name="is_active" class="checkbox" checked />
                        <span class="label-text">Active</span>
                    </label>
                </div>
            </form>

            <div class="modal-action">
                <button type="button" onclick="closeModal()" class="btn">Cancel</button>
                <button type="button" onclick="saveProvider()" class="btn btn-primary">Save</button>
            </div>
        </div>
    </div>

    <script>
        let currentProviderId = null;

        function showCreateModal() {
            currentProviderId = null;
            document.getElementById('modalTitle').textContent = 'Create Provider';
            document.getElementById('providerForm').reset();
            document.getElementById('is_active').checked = true;
            document.getElementById('providerModal').classList.add('modal-open');
        }

        function editProvider(id) {
            currentProviderId = id;
            document.getElementById('modalTitle').textContent = 'Edit Provider';

            fetch(`/api/providers/${id}`)
                .then(response => response.json())
                .then(data => {
                    document.getElementById('name').value = data.name;
                    document.getElementById('provider_type').value = data.provider_type;
                    document.getElementById('base_url').value = data.base_url;
                    document.getElementById('api_key').value = '';
                    document.getElementById('is_active').checked = data.is_active;
                    document.getElementById('providerModal').classList.add('modal-open');
                })
                .catch(error => console.error('Error:', error));
        }

        function deleteProvider(id) {
            if (confirm('Are you sure you want to delete this provider?')) {
                fetch(`/api/providers/${id}`, {
                    method: 'DELETE'
                })
                .then(response => {
                    if (response.ok) {
                        location.reload();
                    } else {
                        alert('Error deleting provider');
                    }
                })
                .catch(error => console.error('Error:', error));
            }
        }

        function closeModal() {
            document.getElementById('providerModal').classList.remove('modal-open');
        }

        function saveProvider() {
            const form = document.getElementById('providerForm');
            const formData = new FormData(form);
            const data = Object.fromEntries(formData);

            data.is_active = formData.get('is_active') === 'on';

            const url = currentProviderId ? `/api/providers/${currentProviderId}` : '/api/providers';
            const method = currentProviderId ? 'PUT' : 'POST';

            fetch(url, {
                method: method,
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(data)
            })
            .then(response => {
                if (response.ok) {
                    closeModal();
                    location.reload();
                } else {
                    alert('Error saving provider');
                }
            })
            .catch(error => console.error('Error:', error));
        }
    </script>
</body>
</html>
"#);

    Ok(Html(html))
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
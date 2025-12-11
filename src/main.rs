use axum::{http::StatusCode, Router};
use serde::Serialize;
use tera::Tera;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod router;
use router::app_router;
use std::{net::SocketAddr, sync::Arc, time::Duration};
mod ai;
mod middleware;
use middleware::extract_user;
mod data;
mod utils;
use data::{Database, ChatRepository, DatabaseError};
mod mcp;
mod local_agents;
mod acp;

use crate::middleware::handle_error;

#[derive(Clone)]
struct AppState {
    db: Arc<Database>,
    tera: Tera,
    chat_repo: ChatRepository,
    mcp_manager: Arc<std::sync::Mutex<Option<crate::mcp::SimplifiedMcpManager>>>,
    local_agent_manager: Arc<crate::local_agents::LocalAgentManager>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "axum_chat=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_path = dotenv::var("DATABASE_PATH").unwrap();

    // Initialize libsql database
    let db = Arc::new(Database::new(db_path));
    db.connect().await.unwrap_or_else(|e| {
        eprintln!("Failed to connect to database: {}", e);
        std::process::exit(1);
    });

    // Run migrations manually
    // For now, we'll assume the database is already migrated
    // TODO: Implement migration runner for libsql

    let chat_repo = ChatRepository::new(db.clone());

    let static_files = ServeDir::new("assets");

    let tera = match Tera::new("templates/**/*") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    // Initialize MCP Manager
    let mcp_manager_result = crate::mcp::SimplifiedMcpManager::new("mcp.json");
    let mcp_manager = match mcp_manager_result {
        Ok(manager) => {
            // Test configuration loading
            let manager_clone = manager.clone();
            tokio::spawn(async move {
                if let Err(e) = manager_clone.test_config_loading().await {
                    eprintln!("Failed to test MCP config loading: {}", e);
                }
            });
            Arc::new(std::sync::Mutex::new(Some(manager)))
        }
        Err(e) => {
            eprintln!("Failed to initialize MCP manager: {}", e);
            Arc::new(std::sync::Mutex::new(None))
        }
    };

    // Initialize Local Agent Manager
    let local_agent_manager = Arc::new(crate::local_agents::LocalAgentManager::new());

    let state = AppState {
        db,
        tera,
        chat_repo,
        mcp_manager,
        local_agent_manager,
    };
    let shared_app_state = Arc::new(state);

    // let jdoom = axum::middleware::from_fn_with_state(shared_app_state.clone(), auth);

    // build our application with some routes
    let app = Router::new()
        // .route(
        //     "/",
        //     get(using_connection_pool_extractor).post(using_connection_pool_extractor),
        // )
        // Use `merge` to combine routers
        .nest_service("/assets", static_files)
        .merge(app_router(shared_app_state.clone()))
        .layer(axum::middleware::from_fn_with_state(
            shared_app_state.clone(),
            handle_error,
        ))
        .layer(axum::middleware::from_fn_with_state(
            shared_app_state.clone(),
            extract_user,
        ))
        .layer(CookieManagerLayer::new());

    // run it with hyper
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug, Serialize, Clone)]
pub struct User {
    id: i64,
    email: String,
    password: String,
    created_at: String, // Changed from NaiveDateTime to String for libsql compatibility
    openai_api_key: Option<String>,
    syntax_theme: String,
    code_line_numbers: bool,
    code_wrap_lines: bool,
    enhanced_markdown: bool,
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

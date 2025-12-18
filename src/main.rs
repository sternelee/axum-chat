use axum::{http::StatusCode, Router};
use serde::Serialize;
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    types::chrono::NaiveDateTime,
    Pool, Sqlite,
};
use tera::Tera;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod router;
use router::app_router;
use std::{net::SocketAddr, path::Path, sync::Arc, time::Duration};
mod ai;
mod middleware;
use middleware::extract_user;
mod data;
mod mcp;
mod utils;
use data::repository::ChatRepository;

use crate::middleware::handle_error;

#[derive(Clone)]
struct AppState {
    pool: Arc<Pool<Sqlite>>,
    tera: Tera,
    chat_repo: ChatRepository,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_tokio_postgres=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_path = dotenv::var("DATABASE_PATH").unwrap();
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .create_if_missing(true);

    // setup connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect_with(options)
        .await
        .expect("can't connect to database");

    // Create a new instance of `Migrator` pointing to the migrations folder.
    let migrator = Migrator::new(Path::new(dotenv::var("MIGRATIONS_PATH").unwrap().as_str()))
        .await
        .unwrap();
    // Run the migrations.
    migrator.run(&pool).await.unwrap();

    let pool = Arc::new(pool);

    // Store a reference to the pool in a global static for access from save_tool_call_confirmation
    unsafe {
        DB_POOL = Some(Arc::as_ptr(&pool) as *const sqlx::Pool<sqlx::Sqlite>);
    }

    let chat_repo = ChatRepository { pool: pool.clone() };

    let static_files = ServeDir::new("assets");
    let uploads_files = ServeDir::new("uploads");

    let tera = match Tera::new("templates/**/*") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    // Initialize MCP manager
    let mcp_manager = mcp::get_mcp_manager();
    let mcp_config_path = std::path::PathBuf::from("mcp.json");

    if let Err(e) = mcp_manager.load_config(&mcp_config_path).await {
        println!("Warning: Could not load mcp.json: {}", e);
        println!("Using default MCP configuration. Copy mcp.json.example to mcp.json to configure MCP servers.");
    }

    // Initialize enabled MCP servers
    match mcp_manager.initialize_servers().await {
        Ok(count) => {
            println!("Successfully initialized {} MCP servers", count);
        }
        Err(e) => {
            println!("Warning: Failed to initialize some MCP servers: {}", e);
        }
    }

    let state = AppState {
        pool,
        tera,
        chat_repo,
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
        .nest_service("/uploads", uploads_files)
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

    // Set up graceful shutdown
    let shutdown_signal = async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");

        println!("Shutting down MCP servers...");
        mcp_manager.shutdown_all().await;
        println!("Shutdown complete.");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();
}

// Global function to access the database pool
static mut DB_POOL: Option<*const sqlx::Pool<sqlx::Sqlite>> = None;

pub fn get_db_pool() -> &'static sqlx::Pool<sqlx::Sqlite> {
    unsafe {
        DB_POOL.unwrap().as_ref().unwrap()
    }
}

#[derive(Debug, sqlx::FromRow, Serialize, Clone)]
pub struct User {
    id: i64,
    email: String,
    password: String,
    created_at: NaiveDateTime,
    openai_api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    system_prompt: Option<String>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_tokens: Option<i64>,
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

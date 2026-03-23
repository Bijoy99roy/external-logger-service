use crate::{config::Config, storage::RedisStore};
use anyhow::Result;
use axum::{
    Json, Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use dotenvy::dotenv;
use serde_json::json;
use tokio::sync::broadcast;
use tracing::info;

mod config;
mod ingest;
mod models;
mod storage;
// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub log_service: String,
    pub broadcaster: broadcast::Sender<String>,
}
#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let use_json = std::env::var("LOG_FORMAT").as_deref() == Ok("json");
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "tower_http=warn".into());

    if use_json {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_current_span(true)
            .with_span_list(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .init();
    }

    let config = Config::from_env();
    info!(
        http_port = config.http_port,
        redis_url = config.redis_url,
        stream_max_len = config.stream_max_len,
        "Starting external logger service"
    );

    let store = RedisStore::new(&config.redis_url, config.stream_max_len).await?;

    let app = Router::new()
        .route("/api/logs", post(ingest::ingestor::ingest_handler))
        .route("/api/logs", get(ingest::ingestor::history_handler))
        .route("/health", get(ingest::ingestor::health_check))
        .fallback(not_found_handler)
        .with_state(store);
    let addr = format!("0.0.0.0:{}", config.http_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!(addr = addr, "HTTP server listening");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn not_found_handler(uri: axum::http::Uri) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": format!("No route for {}", uri.path()),
            "code": "NOT_FOUND"
        })),
    )
}

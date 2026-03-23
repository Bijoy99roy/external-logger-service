use anyhow::Context;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::DateTime;
use serde::Deserialize;
use serde_json::json;
use tracing::{debug, warn};

use crate::{
    models::{IngestPayload, LogEntry},
    storage::RedisStore,
};

pub async fn ingest_handler(
    State(mut store): State<RedisStore>,
    Json(payload): Json<IngestPayload>,
) -> impl IntoResponse {
    let mut stored = Vec::new();
    let mut failed = 0;

    for entry in payload.logs {
        let lg_entry = LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            service: entry.service,
            level: entry.level,
            message: entry.message,
            timestamp: entry.timestamp.unwrap_or_else(chrono::Utc::now),
            fields: entry.fields,
        };
        match store.append(&lg_entry).await {
            Ok(_) => {
                debug!(
                    service = %lg_entry.service,
                    level   = %lg_entry.level,
                    id      = %lg_entry.id,
                    "Log entry persisted"
                );
                stored.push(lg_entry);
            }
            Err(e) => {
                warn!(
                    error   = %e,
                    service = %lg_entry.service,
                    "Failed to persist log entry, skipping"
                );
                failed += 1;
            }
        }
    }
    Json(json!({
        "inserted": stored.len(),
        "failed": failed,
    }))
}

pub async fn history_handler(
    State(mut store): State<RedisStore>,
    Query(params): Query<HistoryParams>,
) -> impl IntoResponse {
    let count = match params.count {
        Some(c) if c == 0 => return Err("Must be grater than zero"),
        Some(c) => c,
        None => 100,
    };

    let entries = store.tail(params.service.as_deref(), count).await.unwrap();
    let fetched_log_count = entries.len();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "logs": entries,
            "count": fetched_log_count,
            "service": params.service
        })),
    ))
}

pub async fn list_service_handler(State(mut store): State<RedisStore>) -> impl IntoResponse {
    let mut services = store.list_services().await.unwrap();
    services.sort();

    let count = services.len();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "services": services,
            "count": count
        })),
    )
}

pub async fn health_check() -> impl IntoResponse {
    let version = std::env::var("SERVICE_VERSION").unwrap_or_else(|_| "unknown".to_string());
    Json(json!({
        "status": "ok",
        "service": "external-log-service",
        "version": version
    }))
}

#[derive(Debug, Deserialize)]
pub struct HistoryParams {
    pub service: Option<String>,
    pub count: Option<usize>,
}

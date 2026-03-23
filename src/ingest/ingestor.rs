use axum::{Json, extract::State, response::IntoResponse};
use chrono::DateTime;
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

pub async fn health_check() -> impl IntoResponse {
    let version = std::env::var("SERVICE_VERSION").unwrap_or_else(|_| "unknown".to_string());
    Json(json!({
        "status": "ok",
        "service": "external-log-service",
        "version": version
    }))
}

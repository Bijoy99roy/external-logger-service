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
    AppState,
    errors::{LogSystemError, LogSystemResult},
    models::{IngestPayload, LogEntry},
    storage::RedisStore,
    validation::validate_ingest_input,
};

pub async fn ingest_handler(
    State(state): State<AppState>,
    Json(payload): Json<IngestPayload>,
) -> LogSystemResult<impl IntoResponse> {
    let mut stored = Vec::new();
    let mut failed = 0;

    let mut store = state.redis.clone();

    let broadcaster = state.broadcaster.clone();
    println!("{}", payload.logs.len());
    validate_ingest_input(&payload)?;
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
                let _ = broadcaster.send(lg_entry.clone());
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
    Ok(Json(json!({
        "inserted": stored.len(),
        "failed": failed,
    })))
}

pub async fn history_handler(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> LogSystemResult<impl IntoResponse> {
    let mut store = state.redis.clone();
    let count = match params.count {
        Some(c) if c == 0 => {
            return Err(LogSystemError::Validation(
                "Must be greater than zero".into(),
            ));
        }
        Some(c) => c,
        None => 100,
    };

    let entries = store.tail(params.service.as_deref(), count).await.unwrap();
    let fetched_log_count = entries.len();

    let service_exists = store
        .service_exists(params.service.as_deref().unwrap_or(""))
        .await
        .unwrap();
    println!("format: {}", service_exists);
    if !service_exists {
        return Err(LogSystemError::NotFound(format!(
            "Service {} has not sent any logs",
            params.service.as_deref().unwrap_or("")
        )));
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "logs": entries,
            "count": fetched_log_count,
            "service": params.service
        })),
    ))
}

pub async fn list_service_handler(
    State(state): State<AppState>,
) -> LogSystemResult<impl IntoResponse> {
    let mut store = state.redis.clone();
    let mut services = store.list_services().await.unwrap();
    services.sort();

    let count = services.len();
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "services": services,
            "count": count
        })),
    ))
}

pub async fn health_check() -> LogSystemResult<impl IntoResponse> {
    let version = std::env::var("SERVICE_VERSION").unwrap_or_else(|_| "unknown".to_string());
    Ok(Json(json!({
        "status": "ok",
        "service": "external-log-service",
        "version": version
    })))
}

#[derive(Debug, Deserialize)]
pub struct HistoryParams {
    pub service: Option<String>,
    pub count: Option<usize>,
}

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use tracing::error;
#[derive(Debug, Error)]
pub enum LogSystemError {
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

impl LogSystemError {
    pub fn status(&self) -> StatusCode {
        match self {
            LogSystemError::Validation(_) => StatusCode::BAD_REQUEST,
            LogSystemError::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }

    pub fn client_message(&self) -> String {
        match self {
            LogSystemError::Validation(msg) => msg.clone(),
            LogSystemError::NotFound(msg) => msg.clone(),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            LogSystemError::Validation(_) => "VALIDATION_ERROR",
            LogSystemError::NotFound(_) => "NOT_FOUND",
        }
    }
}

impl IntoResponse for LogSystemError {
    fn into_response(self) -> Response {
        let status = self.status();

        if status.is_server_error() {
            error!(error = %self, status = status.as_u16(), "Request failed with server error");
        }

        let body = Json(ErrorBody {
            error: self.client_message(),
            code: self.code(),
        });

        (status, body).into_response()
    }
}

#[derive(serde::Serialize)]
struct ErrorBody {
    error: String,
    code: &'static str,
}

pub type LogSystemResult<T> = Result<T, LogSystemError>;

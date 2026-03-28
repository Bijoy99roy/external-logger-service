use crate::{
    errors::{LogSystemError, LogSystemResult},
    models::IngestPayload,
};

pub fn validate_ingest_input(payload: &IngestPayload) -> LogSystemResult<()> {
    if payload.logs.is_empty() {
        return Err(LogSystemError::Validation("Logs can't be empty".into()));
    }

    for entry in &payload.logs {
        let service = entry.service.trim().to_string();

        if service.is_empty() {
            return Err(LogSystemError::Validation(
                "Service must not be blank".into(),
            ));
        }

        if service.contains(':') || service.contains('*') || service.contains('?') {
            return Err(LogSystemError::Validation(
                "Service must not contain ':', '*', or '?' characters".into(),
            ));
        }

        let message = entry.message.trim().to_string();
        if message.is_empty() {
            return Err(LogSystemError::Validation(
                "Message must not be blank".into(),
            ));
        }
    }
    Ok(())
}

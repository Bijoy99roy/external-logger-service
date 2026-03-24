use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Fatal => "fatal",
        };

        write!(f, "{}", s)
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "fatal" => Ok(LogLevel::Fatal),
            other => Err(format!("Unknown log level: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub service: String,
    pub level: LogLevel,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub fields: serde_json::Value,
}

impl LogEntry {
    pub fn new(service: impl Into<String>, level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            service: service.into(),
            level,
            message: message.into(),
            timestamp: Utc::now(),
            fields: serde_json::Value::Null,
        }
    }
}

/// A single entry in the HTTP ingest payload.
/// `id` and `timestamp` are optional — the server fills them if absent.
#[derive(Debug, Deserialize)]
pub struct IngestEntry {
    pub service: String,
    pub level: LogLevel,
    pub message: String,
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub fields: serde_json::Value,
}

/// Payload accepted by the HTTP ingest endpoint (POST /api/logs)
/// Allows batching multiple entries in one request.
#[derive(Debug, Deserialize)]
pub struct IngestPayload {
    pub logs: Vec<IngestEntry>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct WSFilter {
    pub service: Option<String>,
    pub min_level: Option<LogLevel>,
}

impl WSFilter {
    pub fn matches(&self, entry: &LogEntry) -> bool {
        if let Some(ref svc) = self.service {
            if &entry.service != svc {
                return false;
            }
        }
        if let Some(ref min) = self.min_level {
            if &entry.level < min {
                return false;
            }
        }
        true
    }
}

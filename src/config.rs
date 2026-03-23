#[derive(Debug, Clone)]
pub struct Config {
    pub redis_url: String,
    pub http_port: u16,
    /// Maximum log entries retained per Redis stream
    pub stream_max_len: usize,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://128.0.0.1::6379".into()),
            http_port: std::env::var("HTTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
            stream_max_len: std::env::var("STREAM_MAX_LEN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10_000),
        }
    }
}

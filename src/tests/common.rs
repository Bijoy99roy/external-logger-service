use crate::{AppState, build_app, models::LogEntry, storage::RedisStore};
use axum_test::{TestResponse, TestServer};
use tokio::sync::broadcast;
pub struct TestApp {
    pub server: TestServer,
    pub state: AppState,
    pub prefix: String,
    redis_url: String,
}

impl TestApp {
    pub async fn new() -> Self {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let prefix = format!("test:{}", uuid::Uuid::new_v4().simple());

        let store = RedisStore::new(&redis_url, 10000).await.unwrap();

        let (broadcaster, _) = broadcast::channel::<LogEntry>(1024);

        let state = AppState {
            redis: store,
            broadcaster,
        };
        let app = build_app(state.clone());

        let server = TestServer::new(app);

        Self {
            server,
            state,
            prefix,
            redis_url,
        }
    }

    pub async fn post_log(&self, service: &str, level: &str, message: &str) -> TestResponse {
        self.server
            .post("/api/logs")
            .json(&serde_json::json!({
                "logs": [{"service": service, "level": level, "message":message}]
            }))
            .await
    }

    pub async fn cleanup(&mut self) -> Result<(), redis::RedisError> {
        let redis = self.state.redis.clone();
        let mut conn = redis.get_connection().await;
        let _: () = redis::cmd("FLUSHDB").query_async(&mut conn).await?;

        Ok(())
    }
}

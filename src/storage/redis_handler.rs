use anyhow::{Context, Result};
use redis::{AsyncCommands, aio::ConnectionManager};
use tracing::{debug, info, instrument};

use crate::models::LogEntry;

const STREAM_PREFIX: &str = "logs";
const GLOBAL_STREAM: &str = "logs:__all__";

pub struct RedisStore {
    conn: ConnectionManager,
    max_len: usize,
}

impl RedisStore {
    pub async fn new(redis_url: &str, max_len: usize) -> Result<Self> {
        let client = redis::Client::open(redis_url).context("Invalid Redis Url")?;

        let conn = ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        Ok(Self { conn, max_len })
    }

    #[instrument(skip(self, entry), fields(service = %entry.service, level = %entry.level, id = %entry.id))]
    pub async fn append(&mut self, entry: &LogEntry) -> Result<()> {
        let payload = serde_json::to_string(entry).context("Failed to serialize Logentry")?;

        let service_stream = format!("{}:{}", STREAM_PREFIX, entry.service);

        // Per service logging
        let id: String = redis::cmd("XADD")
            .arg(&service_stream)
            .arg("MAXLEN")
            .arg("~")
            .arg(self.max_len)
            .arg("*")
            .arg("entry")
            .arg(&payload)
            .query_async(&mut self.conn)
            .await
            .with_context(|| format!("XADD to stream {} failed", service_stream))?;

        info!(stream = %service_stream, stream_id = %id, "Appended to per-service stream");

        // Global logging
        let _: String = redis::cmd("XADD")
            .arg(GLOBAL_STREAM)
            .arg("MAXLEN")
            .arg("~")
            .arg(self.max_len)
            .arg("*")
            .arg("entry")
            .arg(&payload)
            .query_async(&mut self.conn)
            .await
            .context("XADD to global stream failed")?;

        Ok(())
    }

    pub async fn tail(&mut self, service: Option<&str>, count: usize) -> Result<Vec<LogEntry>> {
        let stream = match service {
            Some(svc) => format!("{}:{}", STREAM_PREFIX, svc),
            None => GLOBAL_STREAM.to_string(),
        };

        let results: redis::streams::StreamRangeReply = redis::cmd("XREVRANGE")
            .arg(&stream)
            .arg("+")
            .arg("-")
            .arg("COUNT")
            .arg(count)
            .query_async(&mut self.conn)
            .await
            .unwrap_or_default();
        println!("{:?}", results);
        let mut entries: Vec<LogEntry> = results
            .ids
            .into_iter()
            .filter_map(|item| {
                item.map
                    .get("entry")
                    .and_then(|v| match v {
                        redis::Value::BulkString(b) => String::from_utf8(b.clone()).ok(),
                        _ => None,
                    })
                    .and_then(|s| serde_json::from_str(&s).ok())
            })
            .collect();
        entries.reverse();
        Ok(entries)
    }
}

impl Clone for RedisStore {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
            max_len: self.max_len,
        }
    }
}

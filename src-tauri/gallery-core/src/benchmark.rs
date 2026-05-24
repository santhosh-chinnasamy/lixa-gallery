use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use crate::models::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkEntry {
    pub timestamp: DateTime<Utc>,
    pub approach_name: String,
    pub operation: String,
    pub file_count: usize,
    pub duration_ms: u64,
    pub avg_ms_per_image: f64,
    pub cpu_cores_detected: usize,
    pub mode: String,
}

#[async_trait]
pub trait BenchmarkLogger: Send + Sync {
    async fn log(&self, entry: BenchmarkEntry) -> Result<()>;
}

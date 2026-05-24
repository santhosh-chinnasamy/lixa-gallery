use async_trait::async_trait;
use gallery_core::benchmark::{BenchmarkEntry, BenchmarkLogger};
use gallery_core::models::Result;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub struct JsonlBenchmarkLogger {
    log_path: PathBuf,
}

impl JsonlBenchmarkLogger {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let mut log_path = app_data_dir;
        log_path.push("benchmarks.jsonl");
        Self { log_path }
    }
}

#[async_trait]
impl BenchmarkLogger for JsonlBenchmarkLogger {
    async fn log(&self, entry: BenchmarkEntry) -> Result<()> {
        let json = serde_json::to_string(&entry).map_err(|e| gallery_core::models::GalleryError::Unknown(e.to_string()))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;
        
        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        
        Ok(())
    }
}

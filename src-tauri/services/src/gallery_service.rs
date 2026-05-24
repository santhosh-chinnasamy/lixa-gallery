use async_recursion::async_recursion;
use gallery_core::benchmark::BenchmarkLogger;
use gallery_core::fs::FileSystem;
use gallery_core::image::ImageProcessor;
use gallery_core::models::{FolderNode, LoadingMode, PhotoMetadata, Result};
use gallery_core::repos::PhotoRepository;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct GalleryService {
    photo_repo: Arc<dyn PhotoRepository>,
    image_processor: Arc<dyn ImageProcessor>,
    fs: Arc<dyn FileSystem>,
    benchmark_logger: Arc<dyn BenchmarkLogger>,
    runtime: tokio::runtime::Handle,
}

impl GalleryService {
    pub fn new(
        photo_repo: Arc<dyn PhotoRepository>,
        image_processor: Arc<dyn ImageProcessor>,
        fs: Arc<dyn FileSystem>,
        benchmark_logger: Arc<dyn BenchmarkLogger>,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        Self {
            photo_repo,
            image_processor,
            fs,
            benchmark_logger,
            runtime,
        }
    }

    pub async fn scan_folder(
        &self,
        folder: &str,
        thumbnail_path: &str,
        mode: LoadingMode,
    ) -> Result<Vec<PhotoMetadata>> {
        use futures::stream::{self, StreamExt};
        use gallery_core::benchmark::BenchmarkEntry;

        let start_time = std::time::Instant::now();
        log::info!("Scanning folder: {} (Mode: {:?})", folder, mode);

        let folder_path = PathBuf::from(folder);
        let image_files = self.fs.list_images_in_dir(&folder_path).await?;
        let file_count = image_files.len();

        let mut prefix = folder.to_string();
        if !prefix.ends_with(std::path::MAIN_SEPARATOR) {
            prefix.push(std::path::MAIN_SEPARATOR);
        }

        let cached_rows = self.photo_repo.get_cached_photos_for_path(&prefix).await?;
        let db_lookup: HashMap<String, (String, i64, i64)> = cached_rows
            .into_iter()
            .map(|record| {
                (
                    record.path,
                    (record.thumbnail_path, record.mtime, record.size),
                )
            })
            .collect();

        // 1. Concurrently read file metadata
        let fs_ref = &self.fs;
        let db_lookup_ref = &db_lookup;

        let mut needs_processing = Vec::new();
        let mut all_photos = Vec::new();

        enum MetaResult {
            Cached(PhotoMetadata),
            NeedsProcessing(PhotoMetadata),
            Failed,
        }

        let mut metadata_stream = stream::iter(image_files)
            .map(|file_path| async move {
                let path_str = file_path.to_string_lossy().to_string();
                if let Ok(current_metadata) = fs_ref.get_file_metadata(&file_path).await {
                    if let Some((thumb_path, db_mtime, db_size)) = db_lookup_ref.get(&path_str) {
                        if current_metadata.modified as i64 == *db_mtime
                            && current_metadata.size as i64 == *db_size
                            && Path::new(thumb_path).exists()
                        {
                            return MetaResult::Cached(PhotoMetadata {
                                metadata: current_metadata,
                                thumbnail_path: thumb_path.to_string(),
                                path: path_str,
                            });
                        }
                    }
                    return MetaResult::NeedsProcessing(PhotoMetadata {
                        metadata: current_metadata,
                        thumbnail_path: String::new(), // Signal for lazy loading/processing
                        path: path_str,
                    });
                }
                MetaResult::Failed
            })
            .buffer_unordered(32); // Max 32 concurrent OS stat calls

        while let Some(result) = metadata_stream.next().await {
            match result {
                MetaResult::Cached(photo) => all_photos.push(photo),
                MetaResult::NeedsProcessing(photo) => {
                    needs_processing.push(photo);
                }
                MetaResult::Failed => {}
            }
        }

        let final_result;

        if mode == LoadingMode::Lazy {
            let total_found = all_photos.len() + needs_processing.len();
            log::info!(
                "Lazy mode: Returning {} photos immediately. {} need background processing.",
                total_found,
                needs_processing.len()
            );

            let mut final_photos = all_photos;
            let background_photos = needs_processing.clone();
            final_photos.extend(needs_processing);

            // Spawn background indexer
            if !background_photos.is_empty() {
                let photo_repo = self.photo_repo.clone();
                let processor = self.image_processor.clone();
                let t_path = thumbnail_path.to_string();

                self.runtime.spawn(async move {
                    log::info!(
                        "Starting butter-smooth background indexing for {} photos",
                        background_photos.len()
                    );
                    
                    let mut chunk = Vec::new();
                    for photo in background_photos {
                        // Rate-limit the background indexer to ensure foreground "butter smoothness".
                        // This allows the CPU to breathe between heavy image decodes.
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                        match processor.convert_image(Path::new(&photo.path), &t_path).await {
                            Ok(processed_photo) => {
                                chunk.push(processed_photo);
                                if chunk.len() >= 10 {
                                    let _ = photo_repo.batch_insert_photos(&chunk).await;
                                    chunk.clear();
                                }
                            },
                            Err(e) => {
                                log::debug!("Background processing skipped for {}: {}", photo.path, e);
                            }
                        }
                    }
                    
                    if !chunk.is_empty() {
                        let _ = photo_repo.batch_insert_photos(&chunk).await;
                    }
                    log::info!("Background indexing complete.");
                });
            }

            final_result = Ok(final_photos);
        } else {
            // 2. Concurrently process new thumbnails (Sync Mode)
            if !needs_processing.is_empty() {
                let processor = self.image_processor.clone();
                let thumb_path_str = thumbnail_path.to_string();

                // Capped concurrency to avoid system freeze on older machines
                let max_concurrent_tasks = 2;

                let mut processing_stream = stream::iter(needs_processing)
                    .map(|photo| {
                        let p = processor.clone();
                        let t = thumb_path_str.clone();
                        async move { p.convert_image(Path::new(&photo.path), &t).await }
                    })
                    .buffer_unordered(max_concurrent_tasks);

                let mut processed_photos = Vec::new();
                while let Some(Ok(photo)) = processing_stream.next().await {
                    processed_photos.push(photo);
                }

                // 3. Chunk database insertions
                if !processed_photos.is_empty() {
                    for chunk in processed_photos.chunks(100) {
                        self.photo_repo.batch_insert_photos(chunk).await?;
                    }
                    all_photos.extend(processed_photos);
                }
            }
            final_result = Ok(all_photos);
        }

        let elapsed = start_time.elapsed();
        let duration_ms = elapsed.as_millis() as u64;

        if let Ok(_) = final_result {
            let entry = BenchmarkEntry {
                timestamp: chrono::Utc::now(),
                approach_name: "baseline".to_string(),
                operation: "full_folder_scan".to_string(),
                file_count,
                duration_ms,
                avg_ms_per_image: if file_count > 0 { duration_ms as f64 / file_count as f64 } else { 0.0 },
                cpu_cores_detected: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
                mode: format!("{:?}", mode),
            };
            let _ = self.benchmark_logger.log(entry).await;
        }

        log::info!("Scan folder ({:?}) took: {:?}", mode, elapsed);
        final_result
    }

    pub async fn get_or_create_thumbnail(
        &self,
        original_path: &Path,
        thumb_dir: &str,
    ) -> Result<String> {
        let metadata = self.image_processor.convert_image(original_path, thumb_dir).await?;
        self.photo_repo.batch_insert_photos(&[metadata.clone()]).await?;
        Ok(metadata.thumbnail_path)
    }

    #[async_recursion]
    pub async fn get_folder_tree(&self, root: &str) -> Result<FolderNode> {
        let root_path = Path::new(root);
        let name = root_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(root)
            .to_string();

        let mut children = Vec::new();
        let subfolders = self.fs.list_subfolders(root_path).await?;
        for folder in subfolders {
            let path_str = folder.to_string_lossy().to_string();
            let child = self.get_folder_tree(&path_str).await?;
            children.push(child);
        }

        Ok(FolderNode {
            name,
            path: root.to_string(),
            children,
        })
    }
}

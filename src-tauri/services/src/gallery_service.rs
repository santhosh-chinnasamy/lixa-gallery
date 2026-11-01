use async_recursion::async_recursion;
use gallery_core::benchmark::BenchmarkLogger;
use gallery_core::formats::{group_raw_jpeg_pairs, is_raw_path};
use gallery_core::fs::FileSystem;
use gallery_core::image::ImageProcessor;
use gallery_core::models::{ExifInfo, FolderNode, LoadingMode, PhotoMetadata, Result};
use gallery_core::repos::{CachedPhotoRecord, PhotoRepository};
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
        let groups = group_raw_jpeg_pairs(image_files);
        let file_count = groups.len();

        let mut prefix = folder.to_string();
        if !prefix.ends_with(std::path::MAIN_SEPARATOR) {
            prefix.push(std::path::MAIN_SEPARATOR);
        }

        let cached_rows = self.photo_repo.get_cached_photos_for_path(&prefix).await?;
        let db_lookup: HashMap<String, CachedPhotoRecord> = cached_rows
            .into_iter()
            .map(|record| (record.path.clone(), record))
            .collect();

        let fs_ref = &self.fs;
        let db_lookup_ref = &db_lookup;
        let processor = &self.image_processor;
        let lazy = mode == LoadingMode::Lazy;

        enum MetaResult {
            Cached { photo: PhotoMetadata, persist: bool },
            NeedsProcessing(PhotoMetadata),
        }

        let mut needs_processing = Vec::new();
        let mut all_photos = Vec::new();
        let mut pending_writes = Vec::new();

        let mut metadata_stream = stream::iter(groups)
            .map(|group| async move {
                let file_path = group.primary;
                let sidecar = group
                    .sidecar
                    .map(|path| path.to_string_lossy().into_owned());
                let path_str = file_path.to_string_lossy().to_string();
                let Ok(current_metadata) = fs_ref.get_file_metadata(&file_path).await else {
                    return None;
                };
                let is_raw = is_raw_path(&file_path);

                if let Some(record) = db_lookup_ref.get(&path_str) {
                    let thumb_ready = record.thumbnail_path.is_empty()
                        || Path::new(&record.thumbnail_path).exists();
                    if current_metadata.modified as i64 == record.mtime
                        && current_metadata.size as i64 == record.size
                        && thumb_ready
                    {
                        let mut photo = PhotoMetadata {
                            metadata: current_metadata,
                            thumbnail_path: record.thumbnail_path.clone(),
                            path: path_str,
                            sidecar_path: sidecar.clone(),
                            preview_path: record.preview_path.clone(),
                            exif: record.exif.clone(),
                            is_raw,
                        };
                        let mut persist = record.sidecar_path != sidecar;
                        if record.exif.is_none() {
                            photo.exif = processor.read_exif(&file_path).await.ok().flatten();
                            persist = true;
                        }
                        return Some(MetaResult::Cached { photo, persist });
                    }
                }

                let mut photo = PhotoMetadata {
                    metadata: current_metadata,
                    thumbnail_path: String::new(),
                    path: path_str,
                    sidecar_path: sidecar,
                    preview_path: None,
                    exif: None,
                    is_raw,
                };
                if lazy {
                    photo.exif = processor.read_exif(&file_path).await.ok().flatten();
                }
                Some(MetaResult::NeedsProcessing(photo))
            })
            .buffer_unordered(32);

        while let Some(result) = metadata_stream.next().await {
            match result {
                Some(MetaResult::Cached { photo, persist }) => {
                    if persist {
                        pending_writes.push(photo.clone());
                    }
                    all_photos.push(photo);
                }
                Some(MetaResult::NeedsProcessing(photo)) => needs_processing.push(photo),
                None => {}
            }
        }

        if !pending_writes.is_empty() {
            self.photo_repo.batch_insert_photos(&pending_writes).await?;
        }

        let photos = if mode == LoadingMode::Lazy {
            let total_found = all_photos.len() + needs_processing.len();
            log::info!(
                "Lazy mode: Returning {} photos immediately. {} need background processing.",
                total_found,
                needs_processing.len()
            );

            let mut final_photos = all_photos;
            let background_photos = needs_processing.clone();
            final_photos.extend(needs_processing);

            if !background_photos.is_empty() {
                let photo_repo = self.photo_repo.clone();
                let processor = self.image_processor.clone();
                let t_path = thumbnail_path.to_string();

                self.runtime.spawn(async move {
                    log::info!(
                        "Starting optimized background indexing for {} photos",
                        background_photos.len()
                    );

                    let mut chunk = Vec::new();
                    for photo in background_photos {
                        match processor
                            .convert_image(Path::new(&photo.path), &t_path)
                            .await
                        {
                            Ok(mut processed_photo) => {
                                processed_photo.sidecar_path = photo.sidecar_path;
                                processed_photo.is_raw = photo.is_raw;
                                if processed_photo.exif.is_none() {
                                    processed_photo.exif = photo.exif;
                                }
                                chunk.push(processed_photo);
                                if chunk.len() >= 10 {
                                    let _ = photo_repo.batch_insert_photos(&chunk).await;
                                    chunk.clear();
                                }
                            }
                            Err(err) => {
                                log::debug!(
                                    "Background processing skipped for {}: {}",
                                    photo.path,
                                    err
                                );
                            }
                        }
                    }

                    if !chunk.is_empty() {
                        let _ = photo_repo.batch_insert_photos(&chunk).await;
                    }
                    log::info!("Background indexing complete.");
                });
            }

            final_photos
        } else {
            if !needs_processing.is_empty() {
                let processor = self.image_processor.clone();
                let thumb_path_str = thumbnail_path.to_string();
                let max_concurrent_tasks = 8;

                let mut processing_stream = stream::iter(needs_processing)
                    .map(|photo| {
                        let p = processor.clone();
                        let t = thumb_path_str.clone();
                        async move {
                            let sidecar = photo.sidecar_path.clone();
                            let is_raw = photo.is_raw;
                            match p.convert_image(Path::new(&photo.path), &t).await {
                                Ok(mut processed) => {
                                    processed.sidecar_path = sidecar;
                                    processed.is_raw = is_raw;
                                    Ok(processed)
                                }
                                Err(err) => Err(err),
                            }
                        }
                    })
                    .buffer_unordered(max_concurrent_tasks);

                let mut processed_photos = Vec::new();
                while let Some(Ok(photo)) = processing_stream.next().await {
                    processed_photos.push(photo);
                }

                if !processed_photos.is_empty() {
                    for chunk in processed_photos.chunks(100) {
                        self.photo_repo.batch_insert_photos(chunk).await?;
                    }
                    all_photos.extend(processed_photos);
                }
            }
            all_photos
        };

        let elapsed = start_time.elapsed();
        let duration_ms = elapsed.as_millis() as u64;
        let entry = BenchmarkEntry {
            timestamp: chrono::Utc::now(),
            approach_name: "rayon_fixed_pool".to_string(),
            operation: "full_folder_scan".to_string(),
            file_count,
            duration_ms,
            avg_ms_per_image: if file_count > 0 {
                duration_ms as f64 / file_count as f64
            } else {
                0.0
            },
            cpu_cores_detected: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            mode: format!("{:?}", mode),
        };
        let _ = self.benchmark_logger.log(entry).await;

        log::info!("Scan folder ({:?}) took: {:?}", mode, elapsed);
        Ok(photos)
    }

    pub async fn get_or_create_thumbnail(
        &self,
        original_path: &Path,
        thumb_dir: &str,
    ) -> Result<String> {
        let metadata = self.materialize(original_path, thumb_dir).await?;
        Ok(metadata.thumbnail_path)
    }

    /// Build the viewer JPEG for a RAW file and remember it. JPEG paths come back unchanged.
    pub async fn ensure_raw_preview(
        &self,
        original_path: &Path,
        thumb_dir: &str,
    ) -> Result<Option<String>> {
        if !is_raw_path(original_path) {
            return Ok(Some(original_path.to_string_lossy().into_owned()));
        }
        let metadata = self.materialize(original_path, thumb_dir).await?;
        Ok(metadata.preview_path)
    }

    async fn materialize(&self, original_path: &Path, thumb_dir: &str) -> Result<PhotoMetadata> {
        let mut metadata = self
            .image_processor
            .convert_image(original_path, thumb_dir)
            .await?;
        if let Ok(existing) = self
            .photo_repo
            .get_photos_by_paths(&[metadata.path.clone()])
            .await
        {
            if let Some(row) = existing.into_iter().next() {
                if metadata.sidecar_path.is_none() {
                    metadata.sidecar_path = row.sidecar_path;
                }
                let incoming_empty = metadata
                    .exif
                    .as_ref()
                    .map(ExifInfo::is_empty)
                    .unwrap_or(true);
                if incoming_empty && row.exif.is_some() {
                    metadata.exif = row.exif;
                }
                if metadata.preview_path.is_none() {
                    metadata.preview_path = row.preview_path;
                }
            }
        }
        self.photo_repo
            .batch_insert_photos(&[metadata.clone()])
            .await?;
        Ok(metadata)
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

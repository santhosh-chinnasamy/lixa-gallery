use gallery_core::benchmark::BenchmarkLogger;
use gallery_core::events::EventHub;
use gallery_core::formats::{group_raw_jpeg_pairs, is_raw_path};
use gallery_core::fs::FileSystem;
use gallery_core::image::ImageProcessor;
use gallery_core::models::{ExifInfo, FolderNode, LoadingMode, PhotoMetadata, Result};
use gallery_core::repos::{CachedPhotoRecord, PhotoRepository};
use gallery_core::thumbnail_cache::stored_thumbnail_is_current;
use gallery_core::work::{WorkKey, WorkPriority};
use gallery_core::work_scheduler::ImageWorkScheduler;
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
    scheduler: ImageWorkScheduler,
    events: Arc<dyn EventHub>,
}

impl GalleryService {
    pub fn new(
        photo_repo: Arc<dyn PhotoRepository>,
        image_processor: Arc<dyn ImageProcessor>,
        fs: Arc<dyn FileSystem>,
        benchmark_logger: Arc<dyn BenchmarkLogger>,
        runtime: tokio::runtime::Handle,
        events: Arc<dyn EventHub>,
    ) -> Self {
        Self {
            photo_repo,
            image_processor,
            fs,
            benchmark_logger,
            runtime,
            scheduler: ImageWorkScheduler::new(),
            events,
        }
    }

    pub fn scheduler(&self) -> &ImageWorkScheduler {
        &self.scheduler
    }

    pub async fn scan_folder(
        &self,
        folder: &str,
        thumbnail_path: &str,
        mode: LoadingMode,
    ) -> Result<Vec<PhotoMetadata>> {
        use futures::stream::{self, StreamExt};
        use gallery_core::benchmark::BenchmarkEntry;

        let generation = self.scheduler.begin_generation().await;
        let start_time = std::time::Instant::now();
        log::info!(
            "Scanning folder: {} (Mode: {:?}, gen: {})",
            folder,
            mode,
            generation
        );

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

        enum MetaResult {
            Cached {
                photo: PhotoMetadata,
                persist: bool,
                needs_exif: bool,
            },
            NeedsProcessing(PhotoMetadata),
            Failed,
        }

        let mut needs_processing = Vec::new();
        let mut needs_exif = Vec::new();
        let mut all_photos = Vec::new();
        let mut pending_writes = Vec::new();

        // Critical path: stat + DB only. No EXIF, no decode.
        let mut metadata_stream = stream::iter(groups)
            .map(|group| async move {
                let file_path = group.primary;
                let sidecar = group
                    .sidecar
                    .map(|path| path.to_string_lossy().into_owned());
                let path_str = file_path.to_string_lossy().to_string();
                let Ok(current_metadata) = fs_ref.get_file_metadata(&file_path).await else {
                    log::warn!("Skipping file during scan: {path_str}");
                    return MetaResult::Failed;
                };
                let is_raw = is_raw_path(&file_path);

                if let Some(record) = db_lookup_ref.get(&path_str) {
                    let thumb_ready = thumbnail_is_ready(
                        &path_str,
                        &record.thumbnail_path,
                        record.mtime,
                        record.size,
                    );
                    if current_metadata.modified as i64 == record.mtime
                        && current_metadata.size as i64 == record.size
                        && thumb_ready
                    {
                        let photo = PhotoMetadata {
                            metadata: current_metadata,
                            thumbnail_path: record.thumbnail_path.clone(),
                            path: path_str,
                            sidecar_path: sidecar.clone(),
                            preview_path: record.preview_path.clone(),
                            exif: record.exif.clone(),
                            is_raw,
                        };
                        let persist = record.sidecar_path != sidecar;
                        let needs_exif = record.exif.is_none();
                        return MetaResult::Cached {
                            photo,
                            persist,
                            needs_exif,
                        };
                    }
                }

                MetaResult::NeedsProcessing(PhotoMetadata {
                    metadata: current_metadata,
                    thumbnail_path: String::new(),
                    path: path_str,
                    sidecar_path: sidecar,
                    preview_path: None,
                    exif: None,
                    is_raw,
                })
            })
            .buffer_unordered(32);

        while let Some(result) = metadata_stream.next().await {
            match result {
                MetaResult::Cached {
                    photo,
                    persist,
                    needs_exif: missing_exif,
                } => {
                    if persist {
                        pending_writes.push(photo.clone());
                    }
                    if missing_exif {
                        needs_exif.push(photo.clone());
                    }
                    all_photos.push(photo);
                }
                MetaResult::NeedsProcessing(photo) => needs_processing.push(photo),
                MetaResult::Failed => {}
            }
        }

        if !pending_writes.is_empty() {
            self.photo_repo.batch_insert_photos(&pending_writes).await?;
        }

        let final_result = if mode == LoadingMode::Lazy {
            let total_found = all_photos.len() + needs_processing.len();
            log::info!(
                "Lazy mode: Returning {} photos immediately. {} need materialize, {} need EXIF.",
                total_found,
                needs_processing.len(),
                needs_exif.len()
            );

            let mut final_photos = all_photos;
            final_photos.extend(needs_processing.iter().cloned());

            self.spawn_background_work(
                generation,
                thumbnail_path.to_string(),
                needs_processing,
                needs_exif,
            );

            Ok(final_photos)
        } else {
            // Sync: same scheduler, await every materialize before return.
            let thumb_path = thumbnail_path.to_string();
            let max_concurrent = self.scheduler.max_workers();
            let scheduler = self.scheduler.clone();
            let processor = self.image_processor.clone();
            let events = self.events.clone();

            let mut processed = Vec::new();
            let mut stream = stream::iter(needs_processing)
                .map(|photo| {
                    let scheduler = scheduler.clone();
                    let processor = processor.clone();
                    let thumb_path = thumb_path.clone();
                    async move {
                        let key = WorkKey::materialize(
                            photo.path.clone(),
                            photo.metadata.modified,
                            photo.metadata.size,
                        );
                        let sidecar = photo.sidecar_path.clone();
                        let is_raw = photo.is_raw;
                        let path = photo.path.clone();
                        match scheduler
                            .run(WorkPriority::Background, key, generation, move || {
                                let processor = processor.clone();
                                let thumb_path = thumb_path.clone();
                                let path = path.clone();
                                async move {
                                    processor.convert_image(Path::new(&path), &thumb_path).await
                                }
                            })
                            .await
                        {
                            Ok(mut meta) => {
                                meta.sidecar_path = sidecar;
                                meta.is_raw = is_raw;
                                Ok(meta)
                            }
                            Err(err) => Err(err),
                        }
                    }
                })
                .buffer_unordered(max_concurrent);

            while let Some(result) = stream.next().await {
                if let Ok(photo) = result {
                    processed.push(photo);
                }
            }

            if !processed.is_empty() {
                for chunk in processed.chunks(100) {
                    self.photo_repo.batch_insert_photos(chunk).await?;
                    let _ = events.emit_photos_updated(chunk);
                }
                all_photos.extend(processed);
            }

            // EXIF-only for already-thumbed rows missing tags.
            self.spawn_background_work(
                generation,
                thumbnail_path.to_string(),
                Vec::new(),
                needs_exif,
            );

            Ok(all_photos)
        };

        let elapsed = start_time.elapsed();
        let duration_ms = elapsed.as_millis() as u64;

        if final_result.is_ok() {
            let entry = BenchmarkEntry {
                timestamp: chrono::Utc::now(),
                approach_name: "priority_scheduler".to_string(),
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
        }

        log::info!("Scan folder ({:?}) took: {:?}", mode, elapsed);
        final_result
    }

    fn spawn_background_work(
        &self,
        generation: u64,
        thumbnail_path: String,
        needs_processing: Vec<PhotoMetadata>,
        needs_exif: Vec<PhotoMetadata>,
    ) {
        if needs_processing.is_empty() && needs_exif.is_empty() {
            return;
        }

        let photo_repo = self.photo_repo.clone();
        let processor = self.image_processor.clone();
        let scheduler = self.scheduler.clone();
        let events = self.events.clone();
        let bg_slots = gallery_core::work::recommended_background_slots(scheduler.max_workers());

        self.runtime.spawn(async move {
            use futures::stream::{self, StreamExt};

            log::info!(
                "Background work gen={generation}: {} materialize, {} exif",
                needs_processing.len(),
                needs_exif.len()
            );

            let mut chunk: Vec<PhotoMetadata> = Vec::new();

            let mut materialize_stream = stream::iter(needs_processing)
                .map(|photo| {
                    let scheduler = scheduler.clone();
                    let processor = processor.clone();
                    let thumb_path = thumbnail_path.clone();
                    async move {
                        let key = WorkKey::materialize(
                            photo.path.clone(),
                            photo.metadata.modified,
                            photo.metadata.size,
                        );
                        let sidecar = photo.sidecar_path.clone();
                        let is_raw = photo.is_raw;
                        let prior_exif = photo.exif.clone();
                        let path = photo.path.clone();
                        match scheduler
                            .run(WorkPriority::Background, key, generation, move || {
                                let processor = processor.clone();
                                let thumb_path = thumb_path.clone();
                                async move {
                                    processor
                                        .convert_image(Path::new(&path), &thumb_path)
                                        .await
                                }
                            })
                            .await
                        {
                            Ok(mut processed) => {
                                processed.sidecar_path = sidecar;
                                processed.is_raw = is_raw;
                                if processed.exif.is_none() {
                                    processed.exif = prior_exif;
                                }
                                Some(processed)
                            }
                            Err(err) => {
                                log::debug!("Background materialize skipped: {err}");
                                None
                            }
                        }
                    }
                })
                .buffer_unordered(bg_slots);

            while let Some(maybe) = materialize_stream.next().await {
                let Some(photo) = maybe else { continue };
                // Always persist so the next open is warm, even if the folder changed.
                chunk.push(photo);
                if chunk.len() >= 10 {
                    flush_chunk(&photo_repo, &events, &scheduler, generation, &mut chunk).await;
                }
            }

            let mut exif_stream = stream::iter(needs_exif)
                .map(|photo| {
                    let scheduler = scheduler.clone();
                    let processor = processor.clone();
                    async move {
                        let key = WorkKey::exif_only(
                            photo.path.clone(),
                            photo.metadata.modified,
                            photo.metadata.size,
                        );
                        let path = photo.path.clone();
                        match scheduler
                            .run(WorkPriority::Background, key, generation, move || {
                                let processor = processor.clone();
                                let mut photo = photo;
                                async move {
                                    photo.exif = processor
                                        .read_exif(Path::new(&path))
                                        .await
                                        .ok()
                                        .flatten();
                                    Ok(photo)
                                }
                            })
                            .await
                        {
                            Ok(photo) => Some(photo),
                            Err(err) => {
                                log::debug!("Background EXIF skipped: {err}");
                                None
                            }
                        }
                    }
                })
                .buffer_unordered(bg_slots);

            while let Some(maybe) = exif_stream.next().await {
                let Some(photo) = maybe else { continue };
                chunk.push(photo);
                if chunk.len() >= 10 {
                    flush_chunk(&photo_repo, &events, &scheduler, generation, &mut chunk).await;
                }
            }

            if !chunk.is_empty() {
                flush_chunk(&photo_repo, &events, &scheduler, generation, &mut chunk).await;
            }
            log::info!("Background indexing complete for gen={generation}.");
        });
    }

    pub async fn get_or_create_thumbnail(
        &self,
        original_path: &Path,
        thumb_dir: &str,
    ) -> Result<String> {
        let metadata = self
            .schedule_materialize(original_path, thumb_dir, WorkPriority::Interactive)
            .await?;
        Ok(metadata.thumbnail_path)
    }

    /// Build the viewer preview for a RAW file and remember it. JPEG paths come back unchanged.
    pub async fn ensure_raw_preview(
        &self,
        original_path: &Path,
        thumb_dir: &str,
    ) -> Result<Option<String>> {
        if !is_raw_path(original_path) {
            return Ok(Some(original_path.to_string_lossy().into_owned()));
        }
        let metadata = self
            .schedule_materialize(original_path, thumb_dir, WorkPriority::Interactive)
            .await?;
        Ok(metadata.preview_path)
    }

    async fn schedule_materialize(
        &self,
        original_path: &Path,
        thumb_dir: &str,
        priority: WorkPriority,
    ) -> Result<PhotoMetadata> {
        let file_meta = self.fs.get_file_metadata(original_path).await?;
        let path_str = original_path.to_string_lossy().into_owned();
        let key = WorkKey::materialize(path_str, file_meta.modified, file_meta.size);
        let generation = self.scheduler.current_generation();
        let processor = self.image_processor.clone();
        let photo_repo = self.photo_repo.clone();
        let events = self.events.clone();
        let scheduler = self.scheduler.clone();
        let path = original_path.to_path_buf();
        let thumb_dir = thumb_dir.to_string();

        let metadata = self
            .scheduler
            .run(priority, key, generation, move || {
                let processor = processor.clone();
                let photo_repo = photo_repo.clone();
                async move { materialize_inner(&processor, &photo_repo, &path, &thumb_dir).await }
            })
            .await?;

        if scheduler.is_current(generation) {
            let _ = events.emit_photos_updated(std::slice::from_ref(&metadata));
        }
        Ok(metadata)
    }

    /// One directory level. The sidebar asks again when a node is expanded,
    /// so opening a folder does not walk the rest of the tree.
    pub async fn get_folder_tree(&self, root: &str) -> Result<FolderNode> {
        let root_path = Path::new(root);
        let name = root_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(root)
            .to_string();

        let subfolders = self.fs.list_subfolders(root_path).await?;
        let mut children = Vec::with_capacity(subfolders.len());
        for folder in subfolders {
            let path_str = folder.to_string_lossy().to_string();
            let child_name = folder
                .file_name()
                .and_then(|s| s.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| path_str.clone());
            let has_children = self
                .fs
                .list_subfolders(&folder)
                .await
                .map(|nested| !nested.is_empty())
                .unwrap_or(false);
            children.push(FolderNode {
                name: child_name,
                path: path_str,
                children: Vec::new(),
                has_children,
            });
        }

        let has_children = !children.is_empty();
        Ok(FolderNode {
            name,
            path: root.to_string(),
            children,
            has_children,
        })
    }
}

async fn materialize_inner(
    processor: &Arc<dyn ImageProcessor>,
    photo_repo: &Arc<dyn PhotoRepository>,
    original_path: &Path,
    thumb_dir: &str,
) -> Result<PhotoMetadata> {
    let mut metadata = processor.convert_image(original_path, thumb_dir).await?;
    if let Ok(existing) = photo_repo
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
    photo_repo
        .batch_insert_photos(std::slice::from_ref(&metadata))
        .await?;
    Ok(metadata)
}

async fn flush_chunk(
    photo_repo: &Arc<dyn PhotoRepository>,
    events: &Arc<dyn EventHub>,
    scheduler: &ImageWorkScheduler,
    generation: u64,
    chunk: &mut Vec<PhotoMetadata>,
) {
    if chunk.is_empty() {
        return;
    }
    let _ = photo_repo.batch_insert_photos(chunk).await;
    if scheduler.is_current(generation) {
        let _ = events.emit_photos_updated(chunk);
    }
    chunk.clear();
}

/// True when the stored thumb path is usable for this identity.
/// Native web formats store the source path as the "thumbnail".
fn thumbnail_is_ready(source_path: &str, thumb_path: &str, mtime: i64, size: i64) -> bool {
    if thumb_path.is_empty() {
        // RAW with no embedded preview is remembered as empty.
        return true;
    }
    if thumb_path == source_path {
        // svg/webp/gif store the original path; the scan already saw the file.
        return true;
    }
    stored_thumbnail_is_current(thumb_path, mtime, size) && Path::new(thumb_path).exists()
}

#[cfg(test)]
mod thumb_ready_tests {
    use super::thumbnail_is_ready;

    #[test]
    fn native_format_source_path_counts_as_ready() {
        assert!(thumbnail_is_ready("/pics/a.webp", "/pics/a.webp", 1, 2));
        assert!(thumbnail_is_ready("/pics/a.gif", "/pics/a.gif", 1, 2));
    }

    #[test]
    fn empty_raw_placeholder_is_ready() {
        assert!(thumbnail_is_ready("/pics/a.CR2", "", 1, 2));
    }
}

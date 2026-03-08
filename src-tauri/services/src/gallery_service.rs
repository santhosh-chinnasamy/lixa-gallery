use async_recursion::async_recursion;
use gallery_core::fs::FileSystem;
use gallery_core::image::ImageProcessor;
use gallery_core::models::{FolderNode, PhotoMetadata, Result};
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
}

impl GalleryService {
    pub fn new(
        photo_repo: Arc<dyn PhotoRepository>,
        image_processor: Arc<dyn ImageProcessor>,
        fs: Arc<dyn FileSystem>,
    ) -> Self {
        Self {
            photo_repo,
            image_processor,
            fs,
        }
    }

    pub async fn scan_folder(
        &self,
        folder: &str,
        thumbnail_path: &str,
    ) -> Result<Vec<PhotoMetadata>> {
        use futures::stream::{self, StreamExt};

        let folder_path = PathBuf::from(folder);
        let image_files = self.fs.list_images_in_dir(&folder_path).await?;

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
            NeedsProcessing(PathBuf),
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
                    return MetaResult::NeedsProcessing(file_path);
                }
                MetaResult::Failed
            })
            .buffer_unordered(32); // Max 32 concurrent OS stat calls

        while let Some(result) = metadata_stream.next().await {
            match result {
                MetaResult::Cached(photo) => all_photos.push(photo),
                MetaResult::NeedsProcessing(path) => needs_processing.push(path),
                MetaResult::Failed => {}
            }
        }

        // 2. Concurrently process new thumbnails
        if !needs_processing.is_empty() {
            let processor = self.image_processor.clone();
            let thumb_path_str = thumbnail_path.to_string();

            // Determine parallelism based on available cores natively
            let max_concurrent_tasks = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);

            let mut processing_stream = stream::iter(needs_processing)
                .map(|file_path| {
                    let p = processor.clone();
                    let t = thumb_path_str.clone();
                    async move { p.convert_image(&file_path, &t).await }
                })
                .buffer_unordered(max_concurrent_tasks);

            let mut processed_photos = Vec::new();
            while let Some(Ok(photo)) = processing_stream.next().await {
                processed_photos.push(photo);
            }

            // 3. Chunk database insertions to avoid parameter limit (SQLite max is usually 999 to 32766, safe bet is chunks of 100 images = ~800 parameters)
            if !processed_photos.is_empty() {
                for chunk in processed_photos.chunks(100) {
                    self.photo_repo.batch_insert_photos(chunk).await?;
                }
                all_photos.extend(processed_photos);
            }
        }

        Ok(all_photos)
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

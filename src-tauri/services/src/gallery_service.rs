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

        let mut needs_processing = Vec::new();
        let mut all_photos = Vec::new();

        for file_path in image_files {
            let path_str = file_path.to_string_lossy().to_string();
            if let Ok(current_metadata) = self.fs.get_file_metadata(&file_path).await {
                if let Some((thumbnail_path, db_mtime, db_size)) = db_lookup.get(&path_str) {
                    if current_metadata.modified as i64 == *db_mtime
                        && current_metadata.size as i64 == *db_size
                        && Path::new(thumbnail_path).exists()
                    {
                        all_photos.push(PhotoMetadata {
                            metadata: current_metadata,
                            thumbnail_path: thumbnail_path.to_string(),
                            path: path_str,
                        });
                        continue;
                    }
                }
                needs_processing.push(file_path);
            }
        }

        if !needs_processing.is_empty() {
            let processor = self.image_processor.clone();
            let thumb_path = thumbnail_path.to_string();

            let mut processed_photos = Vec::new();
            for file_path in needs_processing {
                if let Ok(photo) = processor.convert_image(&file_path, &thumb_path).await {
                    processed_photos.push(photo);
                }
            }

            if !processed_photos.is_empty() {
                self.photo_repo
                    .batch_insert_photos(&processed_photos)
                    .await?;
            }
            all_photos.extend(processed_photos);
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

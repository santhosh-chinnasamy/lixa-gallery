use crate::domain::models::PhotoMetadata;
use crate::domain::repos::PhotoRepository;
use crate::infra::{fs_ops, image_proc};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct GalleryService {
    photo_repo: Arc<dyn PhotoRepository>,
}

impl GalleryService {
    pub fn new(photo_repo: Arc<dyn PhotoRepository>) -> Self {
        Self { photo_repo }
    }

    pub async fn scan_folder(
        &self,
        folder: &str,
        thumbnail_path: &str,
    ) -> anyhow::Result<Vec<PhotoMetadata>> {
        let folder_path = PathBuf::from(folder);
        let image_files = fs_ops::list_images_in_dir(&folder_path)?;

        let mut prefix = folder.to_string();
        if !prefix.ends_with(std::path::MAIN_SEPARATOR) {
            prefix.push(std::path::MAIN_SEPARATOR);
        }

        let cached_rows = self.photo_repo.get_cached_photos_for_path(&prefix).await?;
        let db_lookup: HashMap<String, (String, i64, i64)> = cached_rows
            .into_iter()
            .map(|(path, thumb, mtime, size)| (path, (thumb, mtime, size)))
            .collect();

        let mut needs_processing = Vec::new();
        let mut all_photos = Vec::new();

        for file_path in image_files {
            let path_str = file_path.to_string_lossy().to_string();
            if let Ok(current_metadata) = fs_ops::get_file_metadata(&file_path) {
                if let Some((thumbnail_path, db_mtime, db_size)) = db_lookup.get(&path_str) {
                    if current_metadata.modified as i64 == *db_mtime
                        && current_metadata.size as i64 == *db_size
                        && Path::new(thumbnail_path).exists()
                    {
                        all_photos.push(PhotoMetadata {
                            metadata: current_metadata,
                            thumbnail_path: thumbnail_path.clone(),
                            path: path_str,
                        });
                        continue;
                    }
                }
                needs_processing.push(file_path);
            }
        }

        if !needs_processing.is_empty() {
            let processed_photos: Vec<PhotoMetadata> = needs_processing
                .par_iter()
                .filter_map(|file_path| image_proc::convert_image(file_path, thumbnail_path).ok())
                .collect();

            if !processed_photos.is_empty() {
                self.photo_repo
                    .batch_insert_photos(&processed_photos)
                    .await?;
            }
            all_photos.extend(processed_photos);
        }

        Ok(all_photos)
    }
}

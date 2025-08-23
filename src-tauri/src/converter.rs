use crate::state::AppState;
use anyhow::Result;
use log::{error, info};
use rayon::prelude::*;
use sqlx::{QueryBuilder, Row, Sqlite};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileMetadata {
    pub name: String,
    pub modified: u64,
    pub created: u64,
    pub size: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhotoMetadata {
    pub metadata: FileMetadata,
    pub thumbnail_path: String,
    pub path: String,
}

#[derive(Debug)]
struct PhotoScanResult {
    needs_processing: Vec<PathBuf>,
    cached_photos: Vec<PhotoMetadata>,
}

pub async fn convert_images(
    folder: &str,
    thumbnail_path: String,
    db: tauri::State<'_, AppState>,
) -> Result<Vec<PhotoMetadata>, String> {
    let folder_path = PathBuf::from(folder);

    let t0 = std::time::Instant::now();
    // Step 1: Get all image files from directory
    let entries: Vec<_> = match fs::read_dir(&folder_path) {
        Ok(dir) => dir.filter_map(|e| e.ok()).collect(),
        Err(e) => return Err(format!("Failed to read directory: {}", e)),
    };
    log::info!("enumerate took: {:?}", t0.elapsed());

    let image_files: Vec<PathBuf> = entries
        .into_iter()
        .filter_map(|entry| {
            let file_path = entry.path();
            if file_path.is_dir() {
                return None;
            }

            if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
                if matches!(
                    ext.to_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif"
                ) {
                    Some(file_path)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    log::info!("filter took: {:?}", t0.elapsed());

    // Step 2: Check database for existing photos and compare mtime/size
    let scan_result = check_cached_photos(&image_files, &db, folder)
        .await
        .map_err(|e| format!("Database check failed: {}", e))?;

    log::info!("check_cached_photos took: {:?}", t0.elapsed());
    info!(
        "Found {} cached photos, {} need processing",
        scan_result.cached_photos.len(),
        scan_result.needs_processing.len()
    );

    // Step 3: Process new/changed photos in parallel using Rayon
    let new_photos: Vec<PhotoMetadata> = if !scan_result.needs_processing.is_empty() {
        let processed_photos: Vec<PhotoMetadata> = scan_result
            .needs_processing
            .par_iter()
            .filter_map(
                |file_path| match convert_image(file_path, &thumbnail_path) {
                    Ok(photo_metadata) => Some(photo_metadata),
                    Err(e) => {
                        error!("Failed to process image {}: {}", file_path.display(), e);
                        None
                    }
                },
            )
            .collect();

        log::info!("convert_image took: {:?}", t0.elapsed());

        // Step 4: Batch insert all processed photos into database
        if !processed_photos.is_empty() {
            if let Err(e) = batch_insert_photos(&processed_photos, &db).await {
                error!("Failed to batch insert photos: {}", e);
            }
        }

        log::info!("batch_insert_photos took: {:?}", t0.elapsed());

        processed_photos
    } else {
        Vec::new()
    };

    log::info!("convert_images took: {:?}", t0.elapsed());

    // Step 4: Combine cached and newly processed photos
    let mut all_photos = scan_result.cached_photos;
    all_photos.extend(new_photos);

    Ok(all_photos)
}

async fn check_cached_photos(
    image_files: &[PathBuf],
    db: &tauri::State<'_, AppState>,
    folder: &str,
) -> Result<PhotoScanResult, sqlx::Error> {
    // Only fetch rows for this folder
    let mut prefix = folder.to_string();
    if !prefix.ends_with(std::path::MAIN_SEPARATOR) {
        prefix.push(std::path::MAIN_SEPARATOR);
    }

    // Get all photos from database for this directory
    let rows = sqlx::query(
        "SELECT path, thumbnail_path, mtime, size
             FROM photos
             WHERE path LIKE ?1 || '%'",
    )
    .bind(&prefix)
    .fetch_all(&db.db)
    .await?;

    // Create lookup map for fast access
    let db_lookup: HashMap<String, (String, i64, i64)> = rows
        .into_iter()
        .map(|row| {
            let path: String = row.get("path");
            let thumbnail_path: String = row.get("thumbnail_path");
            let mtime: i64 = row.get("mtime");
            let size: i64 = row.get("size");
            (path, (thumbnail_path, mtime, size))
        })
        .collect();

    let mut needs_processing = Vec::new();
    let mut cached_photos = Vec::new();

    for file_path in image_files {
        let path_str = file_path.to_string_lossy().to_string();

        // Get current file metadata
        match get_file_metadata(file_path) {
            Ok(current_metadata) => {
                if let Some((thumbnail_path, db_mtime, db_size)) = db_lookup.get(&path_str) {
                    // Check if file has changed (mtime or size different)
                    if current_metadata.modified as i64 == *db_mtime
                        && current_metadata.size as i64 == *db_size
                        && PathBuf::from(thumbnail_path).exists()
                    {
                        // File unchanged and thumbnail exists, use cached version
                        cached_photos.push(PhotoMetadata {
                            metadata: current_metadata,
                            thumbnail_path: thumbnail_path.clone(),
                            path: path_str,
                        });
                    } else {
                        // File changed or thumbnail missing, needs reprocessing
                        needs_processing.push(file_path.clone());
                    }
                } else {
                    // New file, needs processing
                    needs_processing.push(file_path.clone());
                }
            }
            Err(e) => {
                error!("Failed to get metadata for {}: {}", file_path.display(), e);
                // Skip files we can't read metadata for
            }
        }
    }

    Ok(PhotoScanResult {
        needs_processing,
        cached_photos,
    })
}

async fn batch_insert_photos(
    photos: &[PhotoMetadata],
    db: &tauri::State<'_, AppState>,
) -> Result<(), sqlx::Error> {
    if photos.is_empty() {
        return Ok(());
    }

    let mut tx = db.db.begin().await?;

    const CHUNK: usize = 1_000;
    for chunk in photos.chunks(CHUNK) {
        let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT INTO photos (path, name, thumbnail_path, mtime, ctime, size) ",
        );
        qb.push_values(chunk, |mut b, p| {
            b.push_bind(&p.path)
                .push_bind(&p.metadata.name)
                .push_bind(&p.thumbnail_path)
                .push_bind(p.metadata.modified as i64)
                .push_bind(p.metadata.created as i64)
                .push_bind(p.metadata.size as i64);
        });
        qb.push(
            " ON CONFLICT(path) DO UPDATE SET
                name=excluded.name,
                thumbnail_path=excluded.thumbnail_path,
                mtime=excluded.mtime,
                ctime=excluded.ctime,
                size=excluded.size
              WHERE photos.mtime <> excluded.mtime
                 OR photos.size  <> excluded.size
                 OR photos.thumbnail_path <> excluded.thumbnail_path
                 OR photos.name  <> excluded.name",
        );

        qb.build().execute(&mut *tx).await?;
    }

    tx.commit().await
}

fn convert_image(file_path: &Path, thumbnail_path: &str) -> anyhow::Result<PhotoMetadata> {
    let img = image::open(file_path)?; // Use '?' to propagate errors, not unwrap()

    // Get original dimensions
    let (width, height) = (img.width(), img.height());

    // Calculate thumbnail dimensions while preserving aspect ratio
    let max_size = 512;
    let (thumb_width, thumb_height) = if width > height {
        let ratio = max_size as f32 / width as f32;
        (max_size, (height as f32 * ratio) as u32)
    } else {
        let ratio = max_size as f32 / height as f32;
        ((width as f32 * ratio) as u32, max_size)
    };

    // Resize while preserving orientation and aspect ratio
    let thumbnail = img.resize(
        thumb_width,
        thumb_height,
        image::imageops::FilterType::CatmullRom,
    );

    let file_stem = &file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Could not get file stem for {}", file_path.display()))?;

    let output_filename = format!("{}.webp", file_stem);
    let output_path = PathBuf::from(thumbnail_path).join(&output_filename);

    thumbnail.save(&output_path)?;

    let result = get_file_metadata(file_path).map_err(|e| {
        anyhow::anyhow!("Failed to get metadata for {}: {}", file_path.display(), e)
    })?;
    Ok(PhotoMetadata {
        metadata: result,
        path: file_path.to_string_lossy().into_owned(),
        thumbnail_path: output_path.to_string_lossy().into_owned(),
    })
}

fn get_file_metadata<P: AsRef<Path>>(path: P) -> std::io::Result<FileMetadata> {
    let metadata = fs::metadata(&path)?;

    let size = metadata.len();

    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let created = metadata
        .created()?
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let name = path
        .as_ref()
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Could not get file name for {}", path.as_ref().display()),
            )
        })?
        .to_string();

    Ok(FileMetadata {
        name,
        size,
        modified,
        created,
    })
}

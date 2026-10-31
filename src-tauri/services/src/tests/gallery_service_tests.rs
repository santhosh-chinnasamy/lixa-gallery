use super::fakes::*;
use crate::gallery_service::GalleryService;
use gallery_core::models::{FileMetadata, LoadingMode, PhotoMetadata};
use gallery_core::thumbnail_cache::CACHE_DIR_NAME;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_scan_folder_finds_new_images() {
    let fs = Arc::new(FakeFileSystem {
        files: Mutex::new(vec![(
            PathBuf::from("/pics/img1.jpg"),
            FileMetadata {
                name: "img1.jpg".to_string(),
                modified: 100,
                created: 100,
                size: 500,
            },
        )]),
    });
    let repo = Arc::new(FakePhotoRepository {
        photos: Mutex::new(vec![]),
    });
    let processor = Arc::new(FakeImageProcessor);
    let logger = Arc::new(FakeBenchmarkLogger);

    let service = GalleryService::new(
        repo.clone(),
        processor,
        fs,
        logger,
        tokio::runtime::Handle::current(),
    );

    let results = service
        .scan_folder("/pics", "/thumbs", LoadingMode::Sync)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.name, "img1.jpg");

    // Check if it was cached in repo
    let cached = repo.photos.lock().unwrap();
    assert_eq!(cached.len(), 1);
}

fn build_service(
    files: Vec<(PathBuf, FileMetadata)>,
    cached: Vec<PhotoMetadata>,
) -> (GalleryService, Arc<FakePhotoRepository>) {
    let fs = Arc::new(FakeFileSystem {
        files: Mutex::new(files),
    });
    let repo = Arc::new(FakePhotoRepository {
        photos: Mutex::new(cached),
    });
    let service = GalleryService::new(
        repo.clone(),
        Arc::new(FakeImageProcessor),
        fs,
        Arc::new(FakeBenchmarkLogger),
        tokio::runtime::Handle::current(),
    );
    (service, repo)
}

fn photo(path: &str, thumbnail_path: &str) -> (PathBuf, FileMetadata, PhotoMetadata) {
    let metadata = FileMetadata {
        name: "IMG_0001.JPG".to_string(),
        modified: 100,
        created: 100,
        size: 500,
    };
    (
        PathBuf::from(path),
        metadata.clone(),
        PhotoMetadata {
            metadata,
            thumbnail_path: thumbnail_path.to_string(),
            path: path.to_string(),
        },
    )
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lixa-{label}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_file(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, b"cached-thumb").unwrap();
}

#[tokio::test]
async fn legacy_stem_thumbnail_is_not_reused_when_the_file_still_exists() {
    let dir = TempDir::new("legacy-thumb");
    let legacy = dir.0.join("IMG_0001.webp");
    write_file(&legacy);
    let (file, metadata, record) = photo("/pics/IMG_0001.JPG", &legacy.to_string_lossy());
    let (service, repo) = build_service(vec![(file, metadata)], vec![record]);

    let results = service
        .scan_folder("/pics", "/thumbs", LoadingMode::Sync)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].thumbnail_path, "thumb");
    assert!(repo.photos.lock().unwrap().len() > 1);
}

#[tokio::test]
async fn current_identity_thumbnail_is_reused() {
    let dir = TempDir::new("current-thumb");
    let hash = "ab".repeat(32);
    let current = dir
        .0
        .join(CACHE_DIR_NAME)
        .join("ab")
        .join(format!("{hash}_100_500.webp"));
    write_file(&current);
    let thumb = current.to_string_lossy().into_owned();
    let (file, metadata, record) = photo("/pics/IMG_0001.JPG", &thumb);
    let (service, repo) = build_service(vec![(file, metadata)], vec![record]);

    let results = service
        .scan_folder("/pics", "/thumbs", LoadingMode::Sync)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].thumbnail_path, thumb);
    assert_eq!(repo.photos.lock().unwrap().len(), 1);
}

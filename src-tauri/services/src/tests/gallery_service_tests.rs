use super::fakes::*;
use crate::gallery_service::GalleryService;
use gallery_core::models::{FileMetadata, LoadingMode};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

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
        copied: Mutex::new(vec![]),
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

#[tokio::test]
async fn test_scan_folder_groups_a_raw_jpeg_pair() {
    let fs = Arc::new(FakeFileSystem {
        files: Mutex::new(vec![
            (
                PathBuf::from("/pics/IMG_0001.CR2"),
                FileMetadata {
                    name: "IMG_0001.CR2".to_string(),
                    modified: 10,
                    created: 10,
                    size: 30,
                },
            ),
            (
                PathBuf::from("/pics/IMG_0001.jpg"),
                FileMetadata {
                    name: "IMG_0001.jpg".to_string(),
                    modified: 10,
                    created: 10,
                    size: 5,
                },
            ),
            (
                PathBuf::from("/pics/solo.NEF"),
                FileMetadata {
                    name: "solo.NEF".to_string(),
                    modified: 11,
                    created: 11,
                    size: 40,
                },
            ),
        ]),
        copied: Mutex::new(vec![]),
    });
    let repo = Arc::new(FakePhotoRepository {
        photos: Mutex::new(vec![]),
    });
    let service = GalleryService::new(
        repo,
        Arc::new(FakeImageProcessor),
        fs,
        Arc::new(FakeBenchmarkLogger),
        tokio::runtime::Handle::current(),
    );

    let results = service
        .scan_folder("/pics", "/thumbs", LoadingMode::Sync)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    let pair = results
        .iter()
        .find(|photo| photo.metadata.name == "IMG_0001.jpg")
        .unwrap();
    assert_eq!(pair.sidecar_path.as_deref(), Some("/pics/IMG_0001.CR2"));
    assert!(!pair.is_raw);
    let raw = results
        .iter()
        .find(|photo| photo.metadata.name == "solo.NEF")
        .unwrap();
    assert!(raw.sidecar_path.is_none());
    assert!(raw.is_raw);
}

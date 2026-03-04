use super::fakes::*;
use crate::gallery_service::GalleryService;
use gallery_core::models::FileMetadata;
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
    });
    let repo = Arc::new(FakePhotoRepository {
        photos: Mutex::new(vec![]),
    });
    let processor = Arc::new(FakeImageProcessor);

    let service = GalleryService::new(repo.clone(), processor, fs);

    let results = service.scan_folder("/pics", "/thumbs").await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.name, "img1.jpg");

    // Check if it was cached in repo
    let cached = repo.photos.lock().unwrap();
    assert_eq!(cached.len(), 1);
}

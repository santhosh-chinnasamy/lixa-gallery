use super::fakes::*;
use crate::gallery_service::GalleryService;
use async_trait::async_trait;
use gallery_core::fs::FileSystem;
use gallery_core::models::{FileMetadata, GalleryError, LoadingMode, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

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

/// Returns a fixed listing per directory and records every call.
struct ListingFs {
    listings: Vec<(&'static str, Vec<&'static str>)>,
    calls: Mutex<Vec<String>>,
}

impl ListingFs {
    fn new(listings: Vec<(&'static str, Vec<&'static str>)>) -> Self {
        Self {
            listings,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl FileSystem for ListingFs {
    async fn get_file_metadata(&self, _path: &Path) -> Result<FileMetadata> {
        Err(GalleryError::Unknown("unused".to_string()))
    }

    async fn list_images_in_dir(&self, _dir: &Path) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    async fn list_subfolders(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let key = dir.to_string_lossy().to_string();
        self.calls.lock().unwrap().push(key.clone());
        let children = self
            .listings
            .iter()
            .find(|(path, _)| *path == key)
            .map(|(_, children)| children.clone())
            .unwrap_or_default();
        Ok(children.into_iter().map(PathBuf::from).collect())
    }

    async fn copy(&self, _from: &Path, _to: &Path) -> Result<u64> {
        Ok(0)
    }

    async fn rename(&self, _from: &Path, _to: &Path) -> Result<()> {
        Ok(())
    }

    async fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        Ok(path.to_path_buf())
    }

    async fn create_dir_all(&self, _path: &Path) -> Result<()> {
        Ok(())
    }
}

fn service_with(fs: Arc<dyn FileSystem>) -> GalleryService {
    GalleryService::new(
        Arc::new(FakePhotoRepository {
            photos: Mutex::new(vec![]),
        }),
        Arc::new(FakeImageProcessor),
        fs,
        Arc::new(FakeBenchmarkLogger),
        tokio::runtime::Handle::current(),
    )
}

#[tokio::test]
async fn folder_tree_loads_one_level_and_does_not_walk_deeper() {
    let fs = Arc::new(ListingFs::new(vec![
        ("/pics", vec!["/pics/deep", "/pics/leaf"]),
        ("/pics/deep", vec!["/pics/deep/nested"]),
        ("/pics/deep/nested", vec!["/pics/deep/nested/more"]),
        ("/pics/leaf", vec![]),
    ]));
    let service = service_with(fs.clone());

    let tree = service.get_folder_tree("/pics").await.unwrap();

    assert_eq!(tree.name, "pics");
    assert!(tree.has_children);
    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[0].name, "deep");
    assert_eq!(tree.children[0].path, "/pics/deep");
    assert!(tree.children[0].has_children);
    assert!(tree.children[0].children.is_empty());
    assert_eq!(tree.children[1].name, "leaf");
    assert!(!tree.children[1].has_children);
    assert!(tree.children[1].children.is_empty());

    // Probes the opened folder and its immediate children, then stops.
    // The old recursion would also have listed `nested` and `more`.
    assert_eq!(
        fs.calls(),
        vec![
            "/pics".to_string(),
            "/pics/deep".to_string(),
            "/pics/leaf".to_string(),
        ]
    );
}

#[tokio::test]
async fn folder_tree_returns_when_a_child_points_back_at_its_parent() {
    let fs = Arc::new(ListingFs::new(vec![
        ("/loop", vec!["/loop/child"]),
        ("/loop/child", vec!["/loop"]),
    ]));
    let service = service_with(fs.clone());

    let tree = tokio::time::timeout(Duration::from_secs(2), service.get_folder_tree("/loop"))
        .await
        .expect("folder tree walked a cycle")
        .unwrap();

    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].path, "/loop/child");
    assert!(tree.children[0].children.is_empty());
    assert_eq!(
        fs.calls(),
        vec!["/loop".to_string(), "/loop/child".to_string()]
    );
}

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

#[async_trait::async_trait]
impl gallery_core::fs::FileSystem for ListingFs {
    async fn file_len(&self, _path: &Path) -> gallery_core::models::Result<u64> {
        Ok(0)
    }

    async fn sync_file(&self, _path: &Path) -> gallery_core::models::Result<()> {
        Ok(())
    }

    async fn remove_file(&self, _path: &Path) -> gallery_core::models::Result<()> {
        Ok(())
    }

    async fn get_file_metadata(&self, _path: &Path) -> gallery_core::models::Result<FileMetadata> {
        Err(gallery_core::models::GalleryError::Unknown(
            "unused".to_string(),
        ))
    }

    async fn list_images_in_dir(&self, _dir: &Path) -> gallery_core::models::Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    async fn list_subfolders(&self, dir: &Path) -> gallery_core::models::Result<Vec<PathBuf>> {
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

    async fn copy(&self, _from: &Path, _to: &Path) -> gallery_core::models::Result<u64> {
        Ok(0)
    }

    async fn rename(&self, _from: &Path, _to: &Path) -> gallery_core::models::Result<()> {
        Ok(())
    }

    async fn canonicalize(&self, path: &Path) -> gallery_core::models::Result<PathBuf> {
        Ok(path.to_path_buf())
    }

    async fn create_dir_all(&self, _path: &Path) -> gallery_core::models::Result<()> {
        Ok(())
    }

    async fn list_dir_names(
        &self,
        _dir: &Path,
    ) -> gallery_core::models::Result<Vec<std::ffi::OsString>> {
        Ok(Vec::new())
    }

    async fn copy_exclusive(&self, _from: &Path, _to: &Path) -> gallery_core::models::Result<u64> {
        Ok(0)
    }

    async fn rename_exclusive(&self, _from: &Path, _to: &Path) -> gallery_core::models::Result<()> {
        Ok(())
    }
}

fn service_with(fs: Arc<dyn gallery_core::fs::FileSystem>) -> GalleryService {
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

    let tree = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        service.get_folder_tree("/loop"),
    )
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

use super::fakes::*;
use crate::gallery_service::GalleryService;
use gallery_core::models::{FileMetadata, LoadingMode};
use std::path::{Path, PathBuf};
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
        Arc::new(FakeEventHub),
    );

    let results = service
        .scan_folder("/pics", "/thumbs", LoadingMode::Sync)
        .await
        .unwrap();

    assert_eq!(results.photos.len(), 1);
    assert_eq!(results.photos[0].metadata.name, "img1.jpg");
    assert!(results.generation >= 1);

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
        Arc::new(FakeEventHub),
    );

    let results = service
        .scan_folder("/pics", "/thumbs", LoadingMode::Sync)
        .await
        .unwrap();

    assert_eq!(results.photos.len(), 2);
    let pair = results
        .photos
        .iter()
        .find(|photo| photo.metadata.name == "IMG_0001.jpg")
        .unwrap();
    assert_eq!(pair.sidecar_path.as_deref(), Some("/pics/IMG_0001.CR2"));
    assert!(!pair.is_raw);
    let raw = results
        .photos
        .iter()
        .find(|photo| photo.metadata.name == "solo.NEF")
        .unwrap();
    assert!(raw.sidecar_path.is_none());
    assert!(raw.is_raw);
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
        Arc::new(FakeEventHub),
    )
}

/// Counts EXIF / convert calls so lazy scan can assert the critical path
/// does not wait on them.
struct CountingProcessor {
    convert_calls: Mutex<usize>,
    exif_calls: Mutex<usize>,
}

#[async_trait::async_trait]
impl gallery_core::image::ImageProcessor for CountingProcessor {
    async fn convert_image(
        &self,
        path: &Path,
        _thumb_dir: &str,
    ) -> gallery_core::models::Result<gallery_core::models::PhotoMetadata> {
        *self.convert_calls.lock().unwrap() += 1;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        FakeImageProcessor.convert_image(path, _thumb_dir).await
    }

    async fn read_exif(
        &self,
        path: &Path,
    ) -> gallery_core::models::Result<Option<gallery_core::models::ExifInfo>> {
        *self.exif_calls.lock().unwrap() += 1;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        FakeImageProcessor.read_exif(path).await
    }

    async fn ensure_decoded_preview(
        &self,
        source: &Path,
        cache_dir: &Path,
    ) -> gallery_core::models::Result<PathBuf> {
        FakeImageProcessor
            .ensure_decoded_preview(source, cache_dir)
            .await
    }
}

#[tokio::test]
async fn lazy_scan_returns_without_waiting_on_exif_or_convert() {
    let fs = Arc::new(FakeFileSystem {
        files: Mutex::new(vec![
            (
                PathBuf::from("/pics/a.jpg"),
                FileMetadata {
                    name: "a.jpg".to_string(),
                    modified: 1,
                    created: 1,
                    size: 10,
                },
            ),
            (
                PathBuf::from("/pics/b.jpg"),
                FileMetadata {
                    name: "b.jpg".to_string(),
                    modified: 2,
                    created: 2,
                    size: 20,
                },
            ),
        ]),
        copied: Mutex::new(vec![]),
    });
    let processor = Arc::new(CountingProcessor {
        convert_calls: Mutex::new(0),
        exif_calls: Mutex::new(0),
    });
    let events = Arc::new(RecordingEventHub {
        photos: Mutex::new(vec![]),
    });
    let service = GalleryService::new(
        Arc::new(FakePhotoRepository {
            photos: Mutex::new(vec![]),
        }),
        processor.clone(),
        fs,
        Arc::new(FakeBenchmarkLogger),
        tokio::runtime::Handle::current(),
        events.clone(),
    );

    let started = std::time::Instant::now();
    let results = service
        .scan_folder("/pics", "/thumbs", LoadingMode::Lazy)
        .await
        .unwrap();
    let elapsed = started.elapsed();

    assert_eq!(results.photos.len(), 2);
    // Two 50ms EXIF/convert waits would push this well over 40ms if awaited.
    assert!(
        elapsed < std::time::Duration::from_millis(40),
        "lazy scan blocked on image work: {elapsed:?}"
    );
    assert_eq!(*processor.exif_calls.lock().unwrap(), 0);
    assert_eq!(*processor.convert_calls.lock().unwrap(), 0);

    // Background work waits for interactive idle, then converts and emits.
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    assert!(*processor.convert_calls.lock().unwrap() >= 1);
    assert!(!events.photos.lock().unwrap().is_empty());
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

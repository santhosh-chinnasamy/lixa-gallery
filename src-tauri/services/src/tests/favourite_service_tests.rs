use super::fakes::*;
use crate::favourite_service::FavouriteService;
use gallery_core::models::{ExportOptions, FileMetadata, PhotoMetadata, Workspace};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

#[tokio::test]
async fn test_favourite_flow() {
    let repo = Arc::new(FakeFavouriteRepository {
        favourites: Mutex::new(vec![]),
    });
    let photo_repo = Arc::new(FakePhotoRepository {
        photos: Mutex::new(vec![]),
    });
    let workspace_repo = Arc::new(FakeWorkspaceRepository {
        workspaces: Mutex::new(vec![]),
    });
    let fs = Arc::new(FakeFileSystem {
        files: Mutex::new(vec![]),
        copied: Mutex::new(vec![]),
    });

    let service = FavouriteService::new(repo.clone(), photo_repo, workspace_repo, fs);

    service
        .add_favourite("/path/to/photo.jpg".to_string())
        .await
        .unwrap();

    let favs = service.get_favourites().await.unwrap();
    assert_eq!(favs.len(), 1);
    assert_eq!(favs[0].path, "/path/to/photo.jpg");

    service
        .remove_favourite("/path/to/photo.jpg".to_string())
        .await
        .unwrap();
    let favs = service.get_favourites().await.unwrap();
    assert_eq!(favs.len(), 0);
}

#[tokio::test]
async fn test_get_favourite_photos_and_groups() {
    let repo = Arc::new(FakeFavouriteRepository {
        favourites: Mutex::new(vec![]),
    });
    let photo_repo = Arc::new(FakePhotoRepository {
        photos: Mutex::new(vec![PhotoMetadata {
            metadata: FileMetadata {
                name: "img1.jpg".to_string(),
                modified: 100,
                created: 100,
                size: 200,
            },
            thumbnail_path: "thumb1".to_string(),
            path: "/wedding/img1.jpg".to_string(),
            sidecar_path: None,
            preview_path: None,
            exif: None,
            is_raw: false,
        }]),
    });
    let workspace_repo = Arc::new(FakeWorkspaceRepository {
        workspaces: Mutex::new(vec![Workspace {
            id: "ws-1".to_string(),
            name: "Wedding Shoot".to_string(),
            root_path: "/wedding".to_string(),
            created_at: 1000,
            last_opened_at: 1000,
        }]),
    });
    let fs = Arc::new(FakeFileSystem {
        files: Mutex::new(vec![(
            PathBuf::from("/vacation/beach.jpg"),
            FileMetadata {
                name: "beach.jpg".to_string(),
                modified: 50,
                created: 50,
                size: 300,
            },
        )]),
        copied: Mutex::new(vec![]),
    });

    let service = FavouriteService::new(repo.clone(), photo_repo, workspace_repo, fs);

    service
        .add_favourite("/wedding/img1.jpg".to_string())
        .await
        .unwrap();
    service
        .add_favourite("/vacation/beach.jpg".to_string())
        .await
        .unwrap();

    // 1. Get all favourite photos
    let all_photos = service.get_favourite_photos(None).await.unwrap();
    assert_eq!(all_photos.len(), 2);

    // 2. Filter by scope /wedding
    let wedding_photos = service
        .get_favourite_photos(Some("/wedding".to_string()))
        .await
        .unwrap();
    assert_eq!(wedding_photos.len(), 1);
    assert_eq!(wedding_photos[0].path, "/wedding/img1.jpg");

    // 3. Get folder groups
    let groups = service.get_favourite_folder_groups().await.unwrap();
    assert_eq!(groups.len(), 2);
    // One group mapped to workspace "Wedding Shoot", one to "/vacation"
    let wedding_group = groups.iter().find(|g| g.folder_path == "/wedding").unwrap();
    assert_eq!(wedding_group.folder_name, "Wedding Shoot");
    assert_eq!(wedding_group.count, 1);
}

#[tokio::test]
async fn test_export_favourites_selective() {
    let repo = Arc::new(FakeFavouriteRepository {
        favourites: Mutex::new(vec![]),
    });
    let photo_repo = Arc::new(FakePhotoRepository {
        photos: Mutex::new(vec![]),
    });
    let workspace_repo = Arc::new(FakeWorkspaceRepository {
        workspaces: Mutex::new(vec![]),
    });
    let fs = Arc::new(FakeFileSystem {
        files: Mutex::new(vec![]),
        copied: Mutex::new(vec![]),
    });

    let service = FavouriteService::new(repo.clone(), photo_repo, workspace_repo, fs.clone());

    service.add_favourite("/a/1.jpg".to_string()).await.unwrap();
    service.add_favourite("/b/2.jpg".to_string()).await.unwrap();

    let events = FakeEventHub;

    // Export only /a/1.jpg
    let opts = ExportOptions {
        destination: "/export".to_string(),
        mode: "copy".to_string(),
        paths: Some(vec!["/a/1.jpg".to_string()]),
        preserve_folder_structure: true,
    };

    service.export_favourites(&events, opts).await.unwrap();
    assert_eq!(fs.copied.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_export_copies_the_raw_sibling_of_a_jpeg_pick() {
    let repo = Arc::new(FakeFavouriteRepository {
        favourites: Mutex::new(vec![]),
    });
    let photo_repo = Arc::new(FakePhotoRepository {
        photos: Mutex::new(vec![]),
    });
    let workspace_repo = Arc::new(FakeWorkspaceRepository {
        workspaces: Mutex::new(vec![]),
    });
    let fs = Arc::new(FakeFileSystem {
        files: Mutex::new(vec![
            (
                PathBuf::from("/cards/IMG_0001.jpg"),
                FileMetadata {
                    name: "IMG_0001.jpg".to_string(),
                    modified: 1,
                    created: 1,
                    size: 10,
                },
            ),
            (
                PathBuf::from("/cards/IMG_0001.CR2"),
                FileMetadata {
                    name: "IMG_0001.CR2".to_string(),
                    modified: 1,
                    created: 1,
                    size: 20,
                },
            ),
        ]),
        copied: Mutex::new(vec![]),
    });

    let service = FavouriteService::new(repo, photo_repo, workspace_repo, fs.clone());
    let opts = ExportOptions {
        destination: "/export".to_string(),
        mode: "copy".to_string(),
        paths: Some(vec!["/cards/IMG_0001.jpg".to_string()]),
        preserve_folder_structure: false,
    };
    service
        .export_favourites(&FakeEventHub, opts)
        .await
        .unwrap();

    let copied = fs.copied.lock().unwrap();
    let sources: Vec<_> = copied.iter().map(|(from, _)| from.clone()).collect();
    assert!(sources.contains(&PathBuf::from("/cards/IMG_0001.jpg")));
    assert!(sources.contains(&PathBuf::from("/cards/IMG_0001.CR2")));
}

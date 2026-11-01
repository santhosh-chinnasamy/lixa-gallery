use super::fakes::*;
use crate::favourite_service::FavouriteService;
use gallery_core::models::{
    ExportFileStatus, ExportOptions, FileMetadata, PhotoMetadata, Workspace,
};
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
        fail_copies: Mutex::new(vec![]),
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
        fail_copies: Mutex::new(vec![]),
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
        fail_copies: Mutex::new(vec![]),
    });

    let service = FavouriteService::new(repo.clone(), photo_repo, workspace_repo, fs);

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

    let result = service.export_favourites(&events, opts).await.unwrap();
    assert_eq!(result.copied, 1);
    assert_eq!(result.moved, 0);
    assert_eq!(result.skipped, 0);
    assert_eq!(result.failed, 0);
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].status, ExportFileStatus::Copied);
    assert_eq!(result.files[0].source, "/a/1.jpg");
    assert_eq!(result.destination, "/export");

    let favs = service.get_favourites().await.unwrap();
    assert_eq!(favs.len(), 2);
}

#[tokio::test]
async fn test_export_reports_skip_and_file_failure() {
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
        fail_copies: Mutex::new(vec!["/shots/broken.jpg".to_string()]),
    });

    let service = FavouriteService::new(repo, photo_repo, workspace_repo, fs);
    let events = FakeEventHub;

    let skipped = ExportOptions {
        destination: "/already".to_string(),
        mode: "copy".to_string(),
        paths: Some(vec!["/already/keep.jpg".to_string()]),
        preserve_folder_structure: false,
    };
    let skipped_result = service.export_favourites(&events, skipped).await.unwrap();
    assert_eq!(skipped_result.skipped, 1);
    assert_eq!(skipped_result.files[0].status, ExportFileStatus::Skipped);
    assert!(skipped_result.files[0]
        .reason
        .as_deref()
        .unwrap()
        .contains("same file"));

    let failed = ExportOptions {
        destination: "/export".to_string(),
        mode: "copy".to_string(),
        paths: Some(vec![
            "/shots/broken.jpg".to_string(),
            "/shots/ok.jpg".to_string(),
        ]),
        preserve_folder_structure: true,
    };
    let failed_result = service.export_favourites(&events, failed).await.unwrap();
    assert_eq!(failed_result.failed, 1);
    assert_eq!(failed_result.copied, 1);
    assert_eq!(failed_result.files[0].status, ExportFileStatus::Failed);
    assert!(failed_result.files[0]
        .reason
        .as_deref()
        .unwrap()
        .contains("permission denied"));
    assert_eq!(failed_result.files[1].status, ExportFileStatus::Copied);
}

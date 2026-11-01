use super::export_fs::ExportFs;
use super::fakes::{FakeFavouriteRepository, FakePhotoRepository, FakeWorkspaceRepository};
use crate::favourite_service::FavouriteService;
use gallery_core::events::EventHub;
use gallery_core::fs::is_cross_device;
use gallery_core::models::{ExportOptions, ExportReport};
use std::io::{Error, ErrorKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn service_for(fs: Arc<ExportFs>) -> (FavouriteService, Arc<FakeFavouriteRepository>) {
    let favourites = Arc::new(FakeFavouriteRepository {
        favourites: Mutex::new(vec![]),
    });
    let service = FavouriteService::new(
        favourites.clone(),
        Arc::new(FakePhotoRepository {
            photos: Mutex::new(vec![]),
        }),
        Arc::new(FakeWorkspaceRepository {
            workspaces: Mutex::new(vec![]),
        }),
        fs,
    );
    (service, favourites)
}

fn options(dest: &str, mode: &str, paths: &[&str], preserve: bool) -> ExportOptions {
    ExportOptions {
        destination: dest.to_string(),
        mode: mode.to_string(),
        paths: Some(paths.iter().map(|path| (*path).to_string()).collect()),
        preserve_folder_structure: preserve,
    }
}

fn idle() -> AtomicBool {
    AtomicBool::new(false)
}

struct CancelAfter {
    flag: Arc<AtomicBool>,
    after: u32,
}

impl EventHub for CancelAfter {
    fn emit_progress(&self, _event: &str, progress: u32) -> anyhow::Result<()> {
        if progress >= self.after {
            self.flag.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
}

struct Quiet;
impl EventHub for Quiet {
    fn emit_progress(&self, _event: &str, _progress: u32) -> anyhow::Result<()> {
        Ok(())
    }
}

fn report_of(copied: u32, renamed: u32, skipped: u32, failed: u32, total: u32) -> ExportReport {
    ExportReport {
        copied,
        renamed,
        skipped,
        failed,
        total,
        ..ExportReport::default()
    }
}

#[test]
fn permission_denied_is_not_a_cross_device_error() {
    let err = Error::new(
        ErrorKind::PermissionDenied,
        "Permission denied (os error 13)",
    );
    assert!(!is_cross_device(&err));
    let crosses = Error::new(ErrorKind::CrossesDevices, "cross-device link");
    assert!(is_cross_device(&crosses));
}

#[tokio::test]
async fn same_drive_move_renames_without_copying_or_deleting_twice() {
    let fs = Arc::new(ExportFs::new(false));
    fs.insert("/shoot/a.jpg", b"aaa");
    let (service, _) = service_for(fs.clone());

    let report = service
        .export_favourites(
            &Quiet,
            options("/out", "move", &["/shoot/a.jpg"], false),
            &idle(),
        )
        .await
        .unwrap();

    assert_eq!(report, report_of(1, 0, 0, 0, 1));
    assert_eq!(fs.rename_count(), 1);
    assert_eq!(fs.copy_count(), 0);
    assert!(fs.read("/shoot/a.jpg").is_none());
    assert_eq!(fs.read("/out/a.jpg").unwrap(), b"aaa");
    assert!(fs.removed_paths().is_empty());
}

#[tokio::test]
async fn cross_device_rename_copies_then_deletes_the_source() {
    let fs = Arc::new(ExportFs::new(false));
    fs.fail_every_rename(ErrorKind::CrossesDevices);
    fs.insert("/shoot/IMG_0001.JPG", b"photo-bytes");
    let (service, favourites) = service_for(fs.clone());
    service
        .add_favourite("/shoot/IMG_0001.JPG".to_string())
        .await
        .unwrap();

    let report = service
        .export_favourites(
            &Quiet,
            options("/out", "move", &["/shoot/IMG_0001.JPG"], false),
            &idle(),
        )
        .await
        .unwrap();

    assert_eq!(report, report_of(1, 0, 0, 0, 1));
    assert_eq!(fs.rename_count(), 1);
    assert_eq!(fs.copy_count(), 1);
    assert!(fs.read("/shoot/IMG_0001.JPG").is_none());
    assert_eq!(fs.read("/out/IMG_0001.JPG").unwrap(), b"photo-bytes");
    assert!(favourites.favourites.lock().unwrap().is_empty());
    let removed = fs.removed_paths();
    assert!(
        removed.iter().any(|path| path.ends_with("IMG_0001.JPG")),
        "source is deleted only after the copy, via remove_file: {removed:?}"
    );
}

#[tokio::test]
async fn cross_device_move_still_renames_when_the_destination_name_is_taken() {
    let fs = Arc::new(ExportFs::new(false));
    fs.fail_every_rename(ErrorKind::CrossesDevices);
    fs.insert("/out/IMG_0001.JPG", b"keep-me");
    fs.insert("/shoot/IMG_0001.JPG", b"incoming");
    let (service, _) = service_for(fs.clone());

    let report = service
        .export_favourites(
            &Quiet,
            options("/out", "move", &["/shoot/IMG_0001.JPG"], false),
            &idle(),
        )
        .await
        .unwrap();

    assert_eq!(report, report_of(0, 1, 0, 0, 1));
    assert_eq!(fs.read("/out/IMG_0001.JPG").unwrap(), b"keep-me");
    assert_eq!(fs.read("/out/IMG_0001 (1).JPG").unwrap(), b"incoming");
    assert!(fs.read("/shoot/IMG_0001.JPG").is_none());
}

#[tokio::test]
async fn short_copy_keeps_the_source_and_the_next_file_still_moves() {
    let fs = Arc::new(ExportFs::new(false));
    fs.fail_every_rename(ErrorKind::CrossesDevices);
    fs.short_copy(4);
    fs.insert("/shoot/a.jpg", b"photo-bytes");
    fs.insert("/shoot/b.jpg", b"second-ok");
    let (service, favourites) = service_for(fs.clone());
    service
        .add_favourite("/shoot/a.jpg".to_string())
        .await
        .unwrap();
    service
        .add_favourite("/shoot/b.jpg".to_string())
        .await
        .unwrap();

    let report = service
        .export_favourites(
            &Quiet,
            options("/out", "move", &["/shoot/a.jpg", "/shoot/b.jpg"], false),
            &idle(),
        )
        .await
        .unwrap();

    assert_eq!(report.copied, 1);
    assert_eq!(report.failed, 1);
    assert_eq!(report.total, 2);
    assert!(!report.cancelled);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].path, "/shoot/a.jpg");
    assert!(
        report.failures[0]
            .reason
            .contains("did not match the source"),
        "{}",
        report.failures[0].reason
    );
    assert!(
        !report.failures[0]
            .reason
            .to_lowercase()
            .contains("permission"),
        "{}",
        report.failures[0].reason
    );

    assert_eq!(fs.read("/shoot/a.jpg").unwrap(), b"photo-bytes");
    assert!(fs.read("/out/a.jpg").is_none());
    assert!(fs.read("/shoot/b.jpg").is_none());
    assert_eq!(fs.read("/out/b.jpg").unwrap(), b"second-ok");
    assert_eq!(favourites.favourites.lock().unwrap().len(), 1);
    assert_eq!(
        favourites.favourites.lock().unwrap()[0].path,
        "/shoot/a.jpg"
    );

    let removed = fs.removed_paths();
    assert!(
        removed
            .iter()
            .any(|path| path == std::path::Path::new("/out/a.jpg")),
        "partial destination is removed: {removed:?}"
    );
    assert!(
        !removed
            .iter()
            .any(|path| path == std::path::Path::new("/shoot/a.jpg")),
        "source is not deleted when the destination is short: {removed:?}"
    );
}

#[tokio::test]
async fn failed_copy_removes_the_partial_and_keeps_the_source() {
    let fs = Arc::new(ExportFs::new(false));
    fs.fail_every_rename(ErrorKind::CrossesDevices);
    fs.fail_copy_after_writing(b"partial", ErrorKind::StorageFull);
    fs.insert("/shoot/a.jpg", b"photo-bytes");
    let (service, _) = service_for(fs.clone());

    let report = service
        .export_favourites(
            &Quiet,
            options("/out", "move", &["/shoot/a.jpg"], false),
            &idle(),
        )
        .await
        .unwrap();

    assert_eq!(report.failed, 1);
    assert_eq!(report.copied, 0);
    assert!(
        report.failures[0].reason.to_lowercase().contains("storage"),
        "{}",
        report.failures[0].reason
    );
    assert_eq!(fs.read("/shoot/a.jpg").unwrap(), b"photo-bytes");
    assert!(fs.read("/out/a.jpg").is_none());
    assert!(!fs
        .removed_paths()
        .iter()
        .any(|path| path == std::path::Path::new("/shoot/a.jpg")));
}

#[tokio::test]
async fn fsync_failure_keeps_the_source_and_removes_the_copy() {
    let fs = Arc::new(ExportFs::new(false));
    fs.fail_every_rename(ErrorKind::CrossesDevices);
    fs.fail_sync();
    fs.insert("/shoot/a.jpg", b"photo-bytes");
    let (service, _) = service_for(fs.clone());

    let report = service
        .export_favourites(
            &Quiet,
            options("/out", "move", &["/shoot/a.jpg"], false),
            &idle(),
        )
        .await
        .unwrap();

    assert_eq!(report.failed, 1);
    assert_eq!(report.copied, 0);
    assert!(
        report.failures[0].reason.contains("fsync failed"),
        "{}",
        report.failures[0].reason
    );
    assert_eq!(fs.read("/shoot/a.jpg").unwrap(), b"photo-bytes");
    assert!(fs.read("/out/a.jpg").is_none());
    assert!(!fs
        .removed_paths()
        .iter()
        .any(|path| path == std::path::Path::new("/shoot/a.jpg")));
}

#[tokio::test]
async fn source_is_kept_when_it_cannot_be_deleted_after_the_copy() {
    let fs = Arc::new(ExportFs::new(false));
    fs.fail_every_rename(ErrorKind::CrossesDevices);
    fs.fail_remove_of("/shoot/a.jpg");
    fs.insert("/shoot/a.jpg", b"photo-bytes");
    let (service, favourites) = service_for(fs.clone());
    service
        .add_favourite("/shoot/a.jpg".to_string())
        .await
        .unwrap();

    let report = service
        .export_favourites(
            &Quiet,
            options("/out", "move", &["/shoot/a.jpg"], false),
            &idle(),
        )
        .await
        .unwrap();

    assert_eq!(report.copied, 0);
    assert_eq!(report.failed, 1);
    assert!(
        report.failures[0].reason.contains("original was kept"),
        "{}",
        report.failures[0].reason
    );
    assert_eq!(fs.read("/shoot/a.jpg").unwrap(), b"photo-bytes");
    assert!(
        fs.read("/out/a.jpg").is_none(),
        "the finished copy is removed so the file is not in both places"
    );
    assert_eq!(favourites.favourites.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn one_permission_failure_does_not_stop_the_rest() {
    let fs = Arc::new(ExportFs::new(false));
    fs.fail_rename_of("/shoot/b.jpg", ErrorKind::PermissionDenied);
    fs.insert("/shoot/a.jpg", b"aaa");
    fs.insert("/shoot/b.jpg", b"bbb");
    fs.insert("/shoot/c.jpg", b"ccc");
    let (service, favourites) = service_for(fs.clone());
    for path in ["/shoot/a.jpg", "/shoot/b.jpg", "/shoot/c.jpg"] {
        service.add_favourite(path.to_string()).await.unwrap();
    }

    let report = service
        .export_favourites(
            &Quiet,
            options(
                "/out",
                "move",
                &["/shoot/a.jpg", "/shoot/b.jpg", "/shoot/c.jpg"],
                false,
            ),
            &idle(),
        )
        .await
        .unwrap();

    assert_eq!(report.copied, 2);
    assert_eq!(report.renamed, 0);
    assert_eq!(report.skipped, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(report.total, 3);
    assert!(!report.cancelled);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].path, "/shoot/b.jpg");
    assert!(
        report.failures[0]
            .reason
            .to_lowercase()
            .contains("permission denied"),
        "{}",
        report.failures[0].reason
    );
    assert_eq!(
        fs.copy_count(),
        0,
        "permission denied is not a cross-device fallback"
    );
    assert!(fs.read("/shoot/a.jpg").is_none());
    assert_eq!(fs.read("/out/a.jpg").unwrap(), b"aaa");
    assert_eq!(fs.read("/shoot/b.jpg").unwrap(), b"bbb");
    assert!(fs.read("/out/b.jpg").is_none());
    assert!(fs.read("/shoot/c.jpg").is_none());
    assert_eq!(fs.read("/out/c.jpg").unwrap(), b"ccc");
    assert_eq!(favourites.favourites.lock().unwrap().len(), 1);
    assert_eq!(
        favourites.favourites.lock().unwrap()[0].path,
        "/shoot/b.jpg"
    );
}

#[tokio::test]
async fn cancel_stops_between_files_and_leaves_each_file_in_one_place() {
    let fs = Arc::new(ExportFs::new(false));
    fs.insert("/shoot/a.jpg", b"aaa");
    fs.insert("/shoot/b.jpg", b"bbb");
    fs.insert("/shoot/c.jpg", b"ccc");
    let (service, _) = service_for(fs.clone());
    let cancel = Arc::new(AtomicBool::new(false));
    let events = CancelAfter {
        flag: cancel.clone(),
        after: 1,
    };

    let report = service
        .export_favourites(
            &events,
            options(
                "/out",
                "move",
                &["/shoot/a.jpg", "/shoot/b.jpg", "/shoot/c.jpg"],
                false,
            ),
            &cancel,
        )
        .await
        .unwrap();

    assert!(report.cancelled);
    assert_eq!(report.copied, 1);
    assert_eq!(report.failed, 0);
    assert_eq!(report.total, 3);
    assert_eq!(fs.rename_count(), 1);
    assert!(fs.read("/shoot/a.jpg").is_none());
    assert_eq!(fs.read("/out/a.jpg").unwrap(), b"aaa");
    assert_eq!(fs.read("/shoot/b.jpg").unwrap(), b"bbb");
    assert!(fs.read("/out/b.jpg").is_none());
    assert_eq!(fs.read("/shoot/c.jpg").unwrap(), b"ccc");
    assert!(fs.read("/out/c.jpg").is_none());
}

#[tokio::test]
async fn cancel_during_a_cross_device_move_does_not_split_a_file() {
    let fs = Arc::new(ExportFs::new(false));
    fs.fail_every_rename(ErrorKind::CrossesDevices);
    fs.insert("/shoot/a.jpg", b"aaa");
    fs.insert("/shoot/b.jpg", b"bbb");
    let (service, _) = service_for(fs.clone());
    let cancel = Arc::new(AtomicBool::new(false));
    let events = CancelAfter {
        flag: cancel.clone(),
        after: 1,
    };

    let report = service
        .export_favourites(
            &events,
            options("/out", "move", &["/shoot/a.jpg", "/shoot/b.jpg"], false),
            &cancel,
        )
        .await
        .unwrap();

    assert!(report.cancelled);
    assert_eq!(report.copied, 1);
    assert_eq!(report.total, 2);
    assert_eq!(fs.copy_count(), 1);
    assert_eq!(fs.rename_count(), 1);
    assert!(fs.read("/shoot/a.jpg").is_none());
    assert_eq!(fs.read("/out/a.jpg").unwrap(), b"aaa");
    assert_eq!(fs.read("/shoot/b.jpg").unwrap(), b"bbb");
    assert!(fs.read("/out/b.jpg").is_none());
}

#[tokio::test]
async fn cancel_before_the_first_file_moves_nothing() {
    let fs = Arc::new(ExportFs::new(false));
    fs.insert("/shoot/a.jpg", b"aaa");
    fs.insert("/shoot/b.jpg", b"bbb");
    let (service, _) = service_for(fs.clone());
    let cancel = AtomicBool::new(true);

    let report = service
        .export_favourites(
            &Quiet,
            options("/out", "move", &["/shoot/a.jpg", "/shoot/b.jpg"], false),
            &cancel,
        )
        .await
        .unwrap();

    assert!(report.cancelled);
    assert_eq!(report.copied, 0);
    assert_eq!(report.failed, 0);
    assert_eq!(report.total, 2);
    assert_eq!(fs.rename_count(), 0);
    assert_eq!(fs.copy_count(), 0);
    assert_eq!(fs.read("/shoot/a.jpg").unwrap(), b"aaa");
    assert_eq!(fs.read("/shoot/b.jpg").unwrap(), b"bbb");
    assert!(fs.read("/out/a.jpg").is_none());
    assert!(fs.read("/out/b.jpg").is_none());
}

#[tokio::test]
async fn copy_mode_does_not_delete_the_source_when_a_later_file_fails() {
    let fs = Arc::new(ExportFs::new(false));
    fs.insert("/shoot/a.jpg", b"aaa");
    fs.insert("/missing/b.jpg", b"bbb");
    let (service, _) = service_for(fs.clone());

    let report = service
        .export_favourites(
            &Quiet,
            options("/out", "copy", &["/shoot/a.jpg", "/gone/b.jpg"], false),
            &idle(),
        )
        .await
        .unwrap();

    assert_eq!(report.copied, 1);
    assert_eq!(report.failed, 1);
    assert_eq!(report.failures[0].path, "/gone/b.jpg");
    assert_eq!(fs.read("/shoot/a.jpg").unwrap(), b"aaa");
    assert_eq!(fs.read("/out/a.jpg").unwrap(), b"aaa");
    assert!(fs.read("/out/b.jpg").is_none());
}

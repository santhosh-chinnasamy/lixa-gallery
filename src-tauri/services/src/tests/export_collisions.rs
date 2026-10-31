use super::export_fs::ExportFs;
use super::fakes::{
    FakeEventHub, FakeFavouriteRepository, FakePhotoRepository, FakeWorkspaceRepository,
};
use crate::favourite_service::FavouriteService;
use gallery_core::models::{ExportOptions, ExportReport};
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

async fn export(service: &FavouriteService, options: ExportOptions) -> ExportReport {
    service
        .export_favourites(&FakeEventHub, options)
        .await
        .unwrap()
}

#[tokio::test]
async fn copy_and_move_keep_both_same_named_files() {
    for move_files in [false, true] {
        for preserve in [false, true] {
            let fs = Arc::new(ExportFs::new(false));
            let mode = if move_files { "move" } else { "copy" };
            let first = "/cardA/DCIM/100CANON/IMG_0001.JPG";
            let second = "/cardB/DCIM/100CANON/IMG_0001.JPG";
            fs.insert(first, b"from-card-a");
            fs.insert(second, b"from-card-b");
            let (service, favourites) = service_for(fs.clone());
            service.add_favourite(first.to_string()).await.unwrap();
            service.add_favourite(second.to_string()).await.unwrap();

            let report = export(&service, options("/out", mode, &[first, second], preserve)).await;

            assert_eq!(
                report,
                ExportReport {
                    copied: 1,
                    renamed: 1,
                    skipped: 0,
                },
                "mode={mode} preserve={preserve}"
            );

            let dir = if preserve { "/out/100CANON" } else { "/out" };
            assert_eq!(
                fs.read(&format!("{dir}/IMG_0001.JPG")).unwrap(),
                b"from-card-a"
            );
            assert_eq!(
                fs.read(&format!("{dir}/IMG_0001 (1).JPG")).unwrap(),
                b"from-card-b"
            );
            assert_eq!(fs.names_under(dir).len(), 2);

            if move_files {
                assert!(fs.read(first).is_none());
                assert!(fs.read(second).is_none());
                assert!(favourites.favourites.lock().unwrap().is_empty());
            } else {
                assert_eq!(fs.read(first).unwrap(), b"from-card-a");
                assert_eq!(fs.read(second).unwrap(), b"from-card-b");
                assert_eq!(favourites.favourites.lock().unwrap().len(), 2);
            }
        }
    }
}

#[tokio::test]
async fn existing_destination_file_is_kept_for_copy_and_move() {
    for mode in ["copy", "move"] {
        let fs = Arc::new(ExportFs::new(false));
        fs.insert("/out/IMG_0001.JPG", b"keep-me");
        fs.insert("/shoot/IMG_0001.JPG", b"incoming");
        let (service, favourites) = service_for(fs.clone());
        service
            .add_favourite("/shoot/IMG_0001.JPG".to_string())
            .await
            .unwrap();

        let report = export(
            &service,
            options("/out", mode, &["/shoot/IMG_0001.JPG"], false),
        )
        .await;

        assert_eq!(
            report,
            ExportReport {
                copied: 0,
                renamed: 1,
                skipped: 0,
            }
        );
        assert_eq!(fs.read("/out/IMG_0001.JPG").unwrap(), b"keep-me");
        assert_eq!(fs.read("/out/IMG_0001 (1).JPG").unwrap(), b"incoming");

        if mode == "move" {
            assert!(fs.read("/shoot/IMG_0001.JPG").is_none());
            assert!(favourites.favourites.lock().unwrap().is_empty());
        } else {
            assert_eq!(fs.read("/shoot/IMG_0001.JPG").unwrap(), b"incoming");
            assert_eq!(favourites.favourites.lock().unwrap().len(), 1);
        }
    }
}

#[tokio::test]
async fn case_only_names_do_not_overwrite_on_a_case_insensitive_volume() {
    let fs = Arc::new(ExportFs::new(true));
    fs.insert("/shootA/IMG_0001.JPG", b"upper");
    fs.insert("/shootB/img_0001.jpg", b"lower");
    let (service, _) = service_for(fs.clone());

    let report = export(
        &service,
        options(
            "/out",
            "copy",
            &["/shootA/IMG_0001.JPG", "/shootB/img_0001.jpg"],
            false,
        ),
    )
    .await;

    assert_eq!(
        report,
        ExportReport {
            copied: 1,
            renamed: 1,
            skipped: 0,
        }
    );
    assert_eq!(
        fs.names_under("/out"),
        vec!["IMG_0001.JPG".to_string(), "img_0001 (1).jpg".to_string()]
    );
    assert_eq!(fs.read("/out/IMG_0001.JPG").unwrap(), b"upper");
    assert_eq!(fs.read("/out/img_0001 (1).jpg").unwrap(), b"lower");
}

#[tokio::test]
async fn pre_existing_case_variant_is_not_replaced() {
    let fs = Arc::new(ExportFs::new(true));
    fs.insert("/out/IMG_0001.JPG", b"keep-me");
    fs.insert("/shoot/img_0001.jpg", b"incoming");
    let (service, _) = service_for(fs.clone());

    let report = export(
        &service,
        options("/out", "copy", &["/shoot/img_0001.jpg"], false),
    )
    .await;

    assert_eq!(report.renamed, 1);
    assert_eq!(fs.read("/out/IMG_0001.JPG").unwrap(), b"keep-me");
    assert_eq!(fs.read("/out/img_0001 (1).jpg").unwrap(), b"incoming");
}

#[tokio::test]
async fn non_ascii_names_and_case_variants_are_both_kept() {
    let fs = Arc::new(ExportFs::new(true));
    fs.insert("/a/写真.JPG", b"one");
    fs.insert("/b/写真.JPG", b"two");
    fs.insert("/c/CAFÉ.JPG", b"upper");
    fs.insert("/d/café.jpg", b"lower");
    let (service, _) = service_for(fs.clone());

    let report = export(
        &service,
        options(
            "/out",
            "copy",
            &["/a/写真.JPG", "/b/写真.JPG", "/c/CAFÉ.JPG", "/d/café.jpg"],
            false,
        ),
    )
    .await;

    assert_eq!(
        report,
        ExportReport {
            copied: 2,
            renamed: 2,
            skipped: 0,
        }
    );
    assert_eq!(fs.read("/out/写真.JPG").unwrap(), b"one");
    assert_eq!(fs.read("/out/写真 (1).JPG").unwrap(), b"two");
    assert_eq!(fs.read("/out/CAFÉ.JPG").unwrap(), b"upper");
    assert_eq!(fs.read("/out/café (1).jpg").unwrap(), b"lower");
}

#[tokio::test]
async fn occupied_suffix_advances_to_the_next_free_name() {
    let fs = Arc::new(ExportFs::new(false));
    fs.insert("/out/IMG_0001.JPG", b"original");
    fs.insert("/out/IMG_0001 (1).JPG", b"already-suffixed");
    fs.insert("/shoot/IMG_0001.JPG", b"incoming");
    let (service, _) = service_for(fs.clone());

    let report = export(
        &service,
        options("/out", "copy", &["/shoot/IMG_0001.JPG"], false),
    )
    .await;

    assert_eq!(report.renamed, 1);
    assert_eq!(fs.read("/out/IMG_0001.JPG").unwrap(), b"original");
    assert_eq!(
        fs.read("/out/IMG_0001 (1).JPG").unwrap(),
        b"already-suffixed"
    );
    assert_eq!(fs.read("/out/IMG_0001 (2).JPG").unwrap(), b"incoming");
}

#[tokio::test]
async fn exporting_a_file_onto_itself_is_skipped() {
    for mode in ["copy", "move"] {
        let fs = Arc::new(ExportFs::new(true));
        fs.insert("/export/IMG_0001.JPG", b"same-file");
        let (service, favourites) = service_for(fs.clone());
        service
            .add_favourite("/export/img_0001.jpg".to_string())
            .await
            .unwrap();

        let report = export(
            &service,
            options("/export", mode, &["/export/img_0001.jpg"], false),
        )
        .await;

        assert_eq!(
            report,
            ExportReport {
                copied: 0,
                renamed: 0,
                skipped: 1,
            }
        );
        assert_eq!(fs.names_under("/export"), vec!["IMG_0001.JPG".to_string()]);
        assert_eq!(fs.read("/export/IMG_0001.JPG").unwrap(), b"same-file");
        assert_eq!(favourites.favourites.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn preserve_folder_structure_still_renames_inside_the_shared_parent() {
    let fs = Arc::new(ExportFs::new(false));
    fs.insert("/cardA/DCIM/100CANON/IMG_0001.JPG", b"a");
    fs.insert("/cardB/DCIM/100CANON/IMG_0001.JPG", b"b");
    fs.insert("/cardB/DCIM/100CANON/IMG_0002.JPG", b"unique");
    let (service, _) = service_for(fs.clone());

    let report = export(
        &service,
        options(
            "/out",
            "copy",
            &[
                "/cardA/DCIM/100CANON/IMG_0001.JPG",
                "/cardB/DCIM/100CANON/IMG_0001.JPG",
                "/cardB/DCIM/100CANON/IMG_0002.JPG",
            ],
            true,
        ),
    )
    .await;

    assert_eq!(
        report,
        ExportReport {
            copied: 2,
            renamed: 1,
            skipped: 0,
        }
    );
    assert_eq!(fs.read("/out/100CANON/IMG_0001.JPG").unwrap(), b"a");
    assert_eq!(fs.read("/out/100CANON/IMG_0001 (1).JPG").unwrap(), b"b");
    assert_eq!(fs.read("/out/100CANON/IMG_0002.JPG").unwrap(), b"unique");
}

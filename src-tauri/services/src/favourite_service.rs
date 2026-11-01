use gallery_core::events::EventHub;
use gallery_core::fs::{collision_key, is_cross_device, uniquified_file_name, FileSystem};
use gallery_core::models::{
    ExportFailure, ExportOptions, ExportReport, Favourite, FavouriteFolderGroup, FileMetadata,
    GalleryError, PhotoMetadata, Result, Workspace,
};
use gallery_core::repos::{FavouriteRepository, PhotoRepository, WorkspaceRepository};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

const MAX_NAME_ATTEMPTS: u32 = 10_000;

pub struct FavouriteService {
    favourite_repo: Arc<dyn FavouriteRepository>,
    photo_repo: Arc<dyn PhotoRepository>,
    workspace_repo: Arc<dyn WorkspaceRepository>,
    fs: Arc<dyn FileSystem>,
}

impl FavouriteService {
    pub fn new(
        favourite_repo: Arc<dyn FavouriteRepository>,
        photo_repo: Arc<dyn PhotoRepository>,
        workspace_repo: Arc<dyn WorkspaceRepository>,
        fs: Arc<dyn FileSystem>,
    ) -> Self {
        Self {
            favourite_repo,
            photo_repo,
            workspace_repo,
            fs,
        }
    }

    pub async fn export_favourites(
        &self,
        events: &dyn EventHub,
        options: ExportOptions,
        cancel: &AtomicBool,
    ) -> Result<ExportReport> {
        let file_paths: Vec<String> = if let Some(paths) = options.paths.clone() {
            paths
        } else {
            self.favourite_repo
                .get_favourites()
                .await?
                .into_iter()
                .map(|f| f.path)
                .collect()
        };

        let mut report = ExportReport {
            total: file_paths.len() as u32,
            ..ExportReport::default()
        };
        let mut counter = 0u32;
        let move_files = options.mode == "move";
        let mut taken_by_dir: HashMap<PathBuf, HashSet<String>> = HashMap::new();

        for file_path in file_paths {
            if cancel.load(Ordering::SeqCst) {
                report.cancelled = true;
                break;
            }

            match self
                .export_one(&file_path, &options, move_files, &mut taken_by_dir)
                .await
            {
                Ok(Placed::Skipped) => report.skipped += 1,
                Ok(placed) => {
                    if placed == Placed::Renamed {
                        report.renamed += 1;
                    } else {
                        report.copied += 1;
                    }
                    if move_files {
                        // The source path is gone. Drop the favourite that pointed at it.
                        if let Err(err) = self.remove_favourite(file_path.clone()).await {
                            push_failure(
                                &mut report,
                                &file_path,
                                format!(
                                    "the file was moved, but it could not be cleared from favourites: {err}"
                                ),
                            );
                        }
                    }
                }
                Err(err) => push_failure(&mut report, &file_path, failure_reason(&err)),
            }

            counter += 1;
            if let Err(err) = events.emit_progress("export-progress", counter) {
                log::warn!("export progress stopped updating: {err}");
            }
        }

        Ok(report)
    }

    async fn export_one(
        &self,
        file_path: &str,
        options: &ExportOptions,
        move_files: bool,
        taken_by_dir: &mut HashMap<PathBuf, HashSet<String>>,
    ) -> Result<Placed> {
        let file = PathBuf::from(file_path);
        let name = file
            .file_name()
            .ok_or_else(|| GalleryError::InvalidPath(file_path.to_string()))?;

        let destination_dir = if options.preserve_folder_structure {
            let folder_name = file
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or("photos");
            let sub_dir = PathBuf::from(&options.destination).join(folder_name);
            self.fs.create_dir_all(&sub_dir).await?;
            sub_dir
        } else {
            let dest = PathBuf::from(&options.destination);
            self.fs.create_dir_all(&dest).await?;
            dest
        };

        self.seed_taken_names(&destination_dir, taken_by_dir)
            .await?;

        let natural = destination_dir.join(name);
        if self.is_same_file(&file, &natural).await? {
            return Ok(Placed::Skipped);
        }

        let taken = taken_by_dir.get_mut(&destination_dir).ok_or_else(|| {
            GalleryError::Unknown(
                "export destination was not prepared for name reservation".to_string(),
            )
        })?;
        let renamed = self
            .place_without_overwrite(&file, &destination_dir, name, move_files, taken)
            .await?;
        Ok(if renamed {
            Placed::Renamed
        } else {
            Placed::Copied
        })
    }

    async fn seed_taken_names(
        &self,
        dir: &Path,
        taken_by_dir: &mut HashMap<PathBuf, HashSet<String>>,
    ) -> Result<()> {
        if taken_by_dir.contains_key(dir) {
            return Ok(());
        }
        let names = self.fs.list_dir_names(dir).await?;
        let taken = names.iter().map(|name| collision_key(name)).collect();
        taken_by_dir.insert(dir.to_path_buf(), taken);
        Ok(())
    }

    async fn is_same_file(&self, src: &Path, dest: &Path) -> Result<bool> {
        let canonical_src = self.fs.canonicalize(src).await?;
        match self.fs.canonicalize(dest).await {
            Ok(canonical_dest) => Ok(canonical_src == canonical_dest),
            Err(GalleryError::Io(err)) if err.kind() == ErrorKind::NotFound => Ok(false),
            Err(err) => Err(err),
        }
    }

    /// Write `src` into `dir` under `original_name`, or `name (n).ext` when that
    /// name is already reserved. Returns whether a suffix was required.
    async fn place_without_overwrite(
        &self,
        src: &Path,
        dir: &Path,
        original_name: &OsStr,
        move_files: bool,
        taken: &mut HashSet<String>,
    ) -> Result<bool> {
        for index in 0..MAX_NAME_ATTEMPTS {
            let candidate = uniquified_file_name(original_name, index);
            let key = collision_key(&candidate);
            if taken.contains(&key) {
                continue;
            }

            let dest = dir.join(&candidate);
            let placed = if move_files {
                self.move_exclusive(src, &dest).await
            } else {
                self.fs.copy_exclusive(src, &dest).await.map(|_| ())
            };
            match placed {
                Ok(()) => {
                    taken.insert(key);
                    return Ok(index > 0);
                }
                Err(err) if name_is_taken(&err) => {
                    taken.insert(key);
                }
                Err(err) => return Err(err),
            }
        }

        Err(GalleryError::Unknown(format!(
            "could not place '{}' in {} without overwriting an existing file",
            original_name.to_string_lossy(),
            dir.display()
        )))
    }

    /// Rename on the same drive. Across drives, copy with the exclusive
    /// create, fsync, check the size, and only then delete the source.
    async fn move_exclusive(&self, src: &Path, dest: &Path) -> Result<()> {
        match self.fs.rename_exclusive(src, dest).await {
            Ok(()) => Ok(()),
            Err(GalleryError::Io(err)) if is_cross_device(&err) => {
                // Unix rename claims the name with an empty placeholder first.
                // That placeholder is removed before the cross-device error is
                // returned; drop a leftover so the exclusive copy can use the
                // name. A non-empty file is left alone.
                self.remove_empty_placeholder(dest).await?;
                self.copy_verify_then_delete_source(src, dest).await
            }
            Err(err) => Err(err),
        }
    }

    async fn remove_empty_placeholder(&self, dest: &Path) -> Result<()> {
        match self.fs.file_len(dest).await {
            Ok(0) => self.fs.remove_file(dest).await,
            Ok(_) => Ok(()),
            Err(GalleryError::Io(err)) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }

    /// Copy `src` to `dest`, fsync, and require the destination size to match.
    /// The source is deleted only after that check. Any failure deletes the
    /// destination (when one was created) and leaves the source in place.
    async fn copy_verify_then_delete_source(&self, src: &Path, dest: &Path) -> Result<()> {
        let copied = match self.fs.copy_exclusive(src, dest).await {
            Ok(bytes) => bytes,
            // The name is already a real file. Leave it untouched; the caller
            // tries the next suffix.
            Err(err) if name_is_taken(&err) => return Err(err),
            Err(err) => return Err(self.cleanup_partial(dest, err).await),
        };

        if let Err(err) = self.fs.sync_file(dest).await {
            return Err(self.cleanup_partial(dest, err).await);
        }

        let src_len = match self.fs.file_len(src).await {
            Ok(len) => len,
            Err(err) => return Err(self.cleanup_partial(dest, err).await),
        };
        let dest_len = match self.fs.file_len(dest).await {
            Ok(len) => len,
            Err(err) => return Err(self.cleanup_partial(dest, err).await),
        };
        if src_len != dest_len || dest_len != copied {
            let mismatch = GalleryError::Unknown(format!(
                "destination size ({dest_len} bytes) did not match the source ({src_len} bytes) after copying, so the original was kept"
            ));
            return Err(self.cleanup_partial(dest, mismatch).await);
        }

        if let Err(err) = self.fs.remove_file(src).await {
            // The copy is complete, but the original is still there. Remove the
            // copy so the file is not left in both places.
            if let Err(rollback) = self.fs.remove_file(dest).await {
                return Err(GalleryError::Unknown(format!(
                    "could not remove the original ({err}) and could not remove the export copy ({rollback})"
                )));
            }
            return Err(GalleryError::Unknown(format!(
                "could not remove the original after copying ({err}), so the export copy was removed and the original was kept"
            )));
        }

        Ok(())
    }

    async fn cleanup_partial(&self, dest: &Path, err: GalleryError) -> GalleryError {
        match self.fs.remove_file(dest).await {
            Ok(()) => err,
            Err(cleanup) => GalleryError::Unknown(format!(
                "{}; also failed to remove the partial copy ({cleanup})",
                failure_reason(&err)
            )),
        }
    }

    pub async fn get_favourite_photos(
        &self,
        folder_scope: Option<String>,
    ) -> Result<Vec<PhotoMetadata>> {
        let favourites = if let Some(ref scope) = folder_scope {
            if !scope.is_empty() {
                self.favourite_repo.get_favourites_by_prefix(scope).await?
            } else {
                self.favourite_repo.get_favourites().await?
            }
        } else {
            self.favourite_repo.get_favourites().await?
        };

        if favourites.is_empty() {
            return Ok(Vec::new());
        }

        let fav_paths: Vec<String> = favourites.into_iter().map(|f| f.path).collect();

        // 1. Check cached photo repository
        let cached_photos = self.photo_repo.get_photos_by_paths(&fav_paths).await?;
        let cached_map: HashMap<String, PhotoMetadata> = cached_photos
            .into_iter()
            .map(|p| (p.path.clone(), p))
            .collect();

        // 2. Build final list, resolving any missing photos via fs
        let mut results = Vec::new();
        for path_str in fav_paths {
            if let Some(photo) = cached_map.get(&path_str) {
                results.push(photo.clone());
            } else {
                let path = Path::new(&path_str);
                let metadata = match self.fs.get_file_metadata(path).await {
                    Ok(m) => m,
                    Err(_) => {
                        let name = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or(&path_str)
                            .to_string();
                        FileMetadata {
                            name,
                            modified: 0,
                            created: 0,
                            size: 0,
                        }
                    }
                };

                results.push(PhotoMetadata {
                    metadata,
                    thumbnail_path: String::new(),
                    path: path_str,
                });
            }
        }

        Ok(results)
    }

    pub async fn get_favourite_folder_groups(&self) -> Result<Vec<FavouriteFolderGroup>> {
        let all_fav_photos = self.get_favourite_photos(None).await?;
        if all_fav_photos.is_empty() {
            return Ok(Vec::new());
        }

        let workspaces = self
            .workspace_repo
            .get_workspaces()
            .await
            .unwrap_or_default();

        // Group photos by workspace root or parent folder
        // key: folder_path -> (folder_name, photos)
        let mut groups: HashMap<String, (String, Vec<PhotoMetadata>)> = HashMap::new();

        for photo in all_fav_photos {
            // Find if photo matches any registered workspace
            let matched_ws = workspaces
                .iter()
                .filter(|ws| photo.path.starts_with(&ws.root_path))
                .max_by_key(|ws| ws.root_path.len());

            let (folder_path, folder_name) = if let Some(ws) = matched_ws {
                (ws.root_path.clone(), ws.name.clone())
            } else {
                let parent = Path::new(&photo.path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string());
                let name = Path::new(&parent)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&parent)
                    .to_string();
                (parent, name)
            };

            groups
                .entry(folder_path)
                .or_insert_with(|| (folder_name, Vec::new()))
                .1
                .push(photo);
        }

        let mut result_groups: Vec<FavouriteFolderGroup> = groups
            .into_iter()
            .map(
                |(folder_path, (folder_name, photos))| FavouriteFolderGroup {
                    folder_name,
                    count: photos.len(),
                    folder_path,
                    photos,
                },
            )
            .collect();

        // Sort by folder name alphabetically
        result_groups.sort_by(|a, b| {
            a.folder_name
                .to_lowercase()
                .cmp(&b.folder_name.to_lowercase())
        });

        Ok(result_groups)
    }

    pub async fn add_favourite(&self, path: String) -> Result<()> {
        self.favourite_repo.add_favourite(path).await
    }

    pub async fn get_favourites(&self) -> Result<Vec<Favourite>> {
        self.favourite_repo.get_favourites().await
    }

    pub async fn remove_favourite(&self, path: String) -> Result<()> {
        self.favourite_repo.remove_favourite(path).await
    }

    pub async fn clear_favourites(&self) -> Result<()> {
        self.favourite_repo.clear_favourites().await
    }

    pub async fn clear_favourites_by_prefix(&self, prefix: &str) -> Result<()> {
        self.favourite_repo.clear_favourites_by_prefix(prefix).await
    }

    pub async fn upsert_workspace(&self, workspace: Workspace) -> Result<()> {
        self.workspace_repo.upsert_workspace(&workspace).await
    }

    pub async fn get_workspaces(&self) -> Result<Vec<Workspace>> {
        self.workspace_repo.get_workspaces().await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placed {
    Copied,
    Renamed,
    Skipped,
}

fn push_failure(report: &mut ExportReport, path: &str, reason: String) {
    log::warn!("export failed for {path}: {reason}");
    report.failed += 1;
    report.failures.push(ExportFailure {
        path: path.to_string(),
        reason,
    });
}

fn failure_reason(err: &GalleryError) -> String {
    match err {
        GalleryError::Io(err) => err.to_string(),
        err => err.to_string(),
    }
}

fn name_is_taken(err: &GalleryError) -> bool {
    match err {
        GalleryError::Io(err) => matches!(
            err.kind(),
            ErrorKind::AlreadyExists | ErrorKind::IsADirectory
        ),
        _ => false,
    }
}

use gallery_core::events::EventHub;
use gallery_core::fs::FileSystem;
use gallery_core::models::{
    ExportFileResult, ExportFileStatus, ExportOptions, ExportResult, Favourite,
    FavouriteFolderGroup, FileMetadata, GalleryError, PhotoMetadata, Result, Workspace,
};
use gallery_core::repos::{FavouriteRepository, PhotoRepository, WorkspaceRepository};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

fn failed_file(source: &str, destination: Option<String>, reason: String) -> ExportFileResult {
    ExportFileResult {
        source: source.to_string(),
        destination,
        status: ExportFileStatus::Failed,
        reason: Some(reason),
    }
}

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
    ) -> Result<ExportResult> {
        let file_paths: Vec<String> = if let Some(paths) = &options.paths {
            paths.clone()
        } else {
            self.favourite_repo
                .get_favourites()
                .await?
                .into_iter()
                .map(|f| f.path)
                .collect()
        };

        let mut files = Vec::with_capacity(file_paths.len());

        for (index, file_path) in file_paths.iter().enumerate() {
            files.push(self.export_one(file_path, &options).await);
            events
                .emit_progress("export-progress", (index + 1) as u32)
                .map_err(|e| GalleryError::Unknown(e.to_string()))?;
        }

        Ok(Self::summarize(options, files))
    }

    async fn export_one(&self, file_path: &str, options: &ExportOptions) -> ExportFileResult {
        let file = PathBuf::from(file_path);
        let Some(name) = file.file_name().map(|name| name.to_owned()) else {
            return failed_file(
                file_path,
                None,
                GalleryError::InvalidPath(file_path.to_string()).to_string(),
            );
        };

        let destination_dir = if options.preserve_folder_structure {
            let folder_name = file
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or("photos");
            PathBuf::from(&options.destination).join(folder_name)
        } else {
            PathBuf::from(&options.destination)
        };

        if let Err(error) = self.fs.create_dir_all(&destination_dir).await {
            return failed_file(file_path, None, error.to_string());
        }

        let destination_path = destination_dir.join(name);
        let destination = destination_path.to_string_lossy().to_string();

        let canonical_src = match self.fs.canonicalize(&file).await {
            Ok(path) => path,
            Err(error) => return failed_file(file_path, Some(destination), error.to_string()),
        };
        let canonical_dst = self
            .fs
            .canonicalize(&destination_path)
            .await
            .unwrap_or_else(|_| destination_path.clone());

        if canonical_src == canonical_dst {
            return ExportFileResult {
                source: file_path.to_string(),
                destination: Some(destination),
                status: ExportFileStatus::Skipped,
                reason: Some("Source and destination are the same file".to_string()),
            };
        }

        if options.mode == "move" {
            if let Err(error) = self.fs.rename(&file, &destination_path).await {
                return failed_file(file_path, Some(destination), error.to_string());
            }
            if let Err(error) = self.remove_favourite(file_path.to_string()).await {
                return failed_file(
                    file_path,
                    Some(destination),
                    format!("Moved the file, but it could not be removed from favourites: {error}"),
                );
            }
            ExportFileResult {
                source: file_path.to_string(),
                destination: Some(destination),
                status: ExportFileStatus::Moved,
                reason: None,
            }
        } else if let Err(error) = self.fs.copy(&file, &destination_path).await {
            failed_file(file_path, Some(destination), error.to_string())
        } else {
            ExportFileResult {
                source: file_path.to_string(),
                destination: Some(destination),
                status: ExportFileStatus::Copied,
                reason: None,
            }
        }
    }

    fn summarize(options: ExportOptions, files: Vec<ExportFileResult>) -> ExportResult {
        let mut copied = 0;
        let mut moved = 0;
        let mut skipped = 0;
        let mut failed = 0;
        for file in &files {
            match file.status {
                ExportFileStatus::Copied => copied += 1,
                ExportFileStatus::Moved => moved += 1,
                ExportFileStatus::Skipped => skipped += 1,
                ExportFileStatus::Failed => failed += 1,
            }
        }
        ExportResult {
            destination: options.destination,
            mode: options.mode,
            copied,
            moved,
            skipped,
            failed,
            files,
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

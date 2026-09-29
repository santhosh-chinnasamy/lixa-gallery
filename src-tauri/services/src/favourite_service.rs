use gallery_core::events::EventHub;
use gallery_core::fs::FileSystem;
use gallery_core::models::{
    ExportOptions, Favourite, FavouriteFolderGroup, FileMetadata, GalleryError, PhotoMetadata,
    Result, Workspace,
};
use gallery_core::repos::{FavouriteRepository, PhotoRepository, WorkspaceRepository};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

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
    ) -> Result<()> {
        let file_paths: Vec<String> = if let Some(paths) = options.paths {
            paths
        } else {
            self.favourite_repo
                .get_favourites()
                .await?
                .into_iter()
                .map(|f| f.path)
                .collect()
        };

        let mut counter = 0;
        let mode = options.mode.as_str();

        for file_path in file_paths {
            let file = PathBuf::from(&file_path);
            let name = file
                .file_name()
                .ok_or_else(|| GalleryError::InvalidPath(file_path.clone()))?;

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

            let destination_path = destination_dir.join(name);

            let canonical_src = self.fs.canonicalize(&file).await?;
            let canonical_dst = self
                .fs
                .canonicalize(&destination_path)
                .await
                .unwrap_or_else(|_| destination_path.clone());

            if canonical_src == canonical_dst {
                continue;
            }

            if mode == "move" {
                self.fs.rename(&file, &destination_path).await?;
                // Remove from DB since the file was physically moved
                self.remove_favourite(file_path).await?;
            } else {
                self.fs.copy(&file, &destination_path).await?;
            }

            counter += 1;
            events
                .emit_progress("export-progress", counter)
                .map_err(|e| GalleryError::Unknown(e.to_string()))?;
        }

        Ok(())
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

use async_trait::async_trait;
use gallery_core::fs::FileSystem;
use gallery_core::models::{FileMetadata, GalleryError, Result};
use std::{
    ffi::OsString,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct LocalFileSystem;

#[async_trait]
impl FileSystem for LocalFileSystem {
    async fn get_file_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let metadata = fs::metadata(path).await?;
        let size = metadata.len();

        let modified = metadata
            .modified()
            .map_err(|e| GalleryError::Io(e))?
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let created = metadata
            .created()
            .map_err(|e| GalleryError::Io(e))?
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                GalleryError::InvalidPath(format!("Could not get file name for {}", path.display()))
            })?
            .to_string();

        Ok(FileMetadata {
            name,
            size,
            modified,
            created,
        })
    }

    async fn list_images_in_dir(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = fs::read_dir(dir).await?;
        let mut images = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if matches!(
                    ext.to_lowercase().as_str(),
                    "jpg"
                        | "jpeg"
                        | "png"
                        | "webp"
                        | "bmp"
                        | "gif"
                        | "tiff"
                        | "tif"
                        | "avif"
                        | "heic"
                        | "heif"
                        | "svg"
                ) {
                    images.push(path);
                }
            }
        }
        Ok(images)
    }

    async fn list_subfolders(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = fs::read_dir(dir).await?;
        let mut folders = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            }
        }
        Ok(folders)
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<u64> {
        fs::copy(from, to).await.map_err(|e| GalleryError::Io(e))
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        fs::rename(from, to).await.map_err(|e| GalleryError::Io(e))
    }

    async fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        fs::canonicalize(path)
            .await
            .map_err(|e| GalleryError::Io(e))
    }

    async fn create_dir_all(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)
            .await
            .map_err(|e| GalleryError::Io(e))
    }

    async fn list_dir_names(&self, dir: &Path) -> Result<Vec<OsString>> {
        let mut entries = match fs::read_dir(dir).await {
            Ok(entries) => entries,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(GalleryError::Io(err)),
        };
        let mut names = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(GalleryError::Io)? {
            names.push(entry.file_name());
        }
        Ok(names)
    }

    async fn copy_exclusive(&self, from: &Path, to: &Path) -> Result<u64> {
        let mut src = fs::File::open(from).await.map_err(GalleryError::Io)?;
        let mut dst = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(to)
            .await
        {
            Ok(file) => file,
            Err(err) => return Err(GalleryError::Io(err)),
        };

        let copied = match tokio::io::copy(&mut src, &mut dst).await {
            Ok(bytes) => bytes,
            Err(err) => {
                drop(dst);
                let _ = fs::remove_file(to).await;
                return Err(GalleryError::Io(err));
            }
        };

        if let Err(err) = dst.flush().await {
            drop(dst);
            let _ = fs::remove_file(to).await;
            return Err(GalleryError::Io(err));
        }

        if let Ok(metadata) = src.metadata().await {
            let _ = dst.set_permissions(metadata.permissions()).await;
        }

        Ok(copied)
    }

    async fn rename_exclusive(&self, from: &Path, to: &Path) -> Result<()> {
        rename_without_overwrite(from, to).await
    }

    async fn file_len(&self, path: &Path) -> Result<u64> {
        let metadata = fs::metadata(path).await.map_err(GalleryError::Io)?;
        Ok(metadata.len())
    }

    async fn sync_file(&self, path: &Path) -> Result<()> {
        let file = fs::File::open(path).await.map_err(GalleryError::Io)?;
        file.sync_all().await.map_err(GalleryError::Io)
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        match fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(GalleryError::Io(err)),
        }
    }
}

/// Move `from` to `to` without replacing an existing file.
///
/// Windows `fs::rename` already fails when the destination exists. On Unix,
/// `rename` replaces the destination, so the name is claimed first with
/// `create_new` and the source is renamed over that placeholder only. If the
/// rename fails (including a cross-device move), the placeholder is removed
/// and the source is left in place.
async fn rename_without_overwrite(from: &Path, to: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        fs::rename(from, to).await.map_err(GalleryError::Io)
    }

    #[cfg(not(windows))]
    {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(to)
            .await
        {
            Ok(placeholder) => drop(placeholder),
            Err(err) => return Err(GalleryError::Io(err)),
        }

        match fs::rename(from, to).await {
            Ok(()) => Ok(()),
            Err(err) => {
                let _ = fs::remove_file(to).await;
                Err(GalleryError::Io(err))
            }
        }
    }
}

pub async fn get_file_metadata<P: AsRef<Path>>(path: P) -> Result<FileMetadata> {
    LocalFileSystem.get_file_metadata(path.as_ref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_list_images_in_dir() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.jpg");
        File::create(&file_path).unwrap();

        let other_path = dir.path().join("test.txt");
        File::create(&other_path).unwrap();

        let fs = LocalFileSystem;
        let images = fs.list_images_in_dir(dir.path()).await.unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].file_name().unwrap(), "test.jpg");
    }

    #[tokio::test]
    async fn test_get_file_metadata() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("meta.png");
        File::create(&file_path).unwrap();

        let fs = LocalFileSystem;
        let metadata = fs.get_file_metadata(&file_path).await.unwrap();

        assert_eq!(metadata.name, "meta.png");
        assert!(metadata.size == 0);
    }

    #[tokio::test]
    async fn copy_exclusive_does_not_overwrite_existing_bytes() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.jpg");
        let dest = dir.path().join("IMG_0001.JPG");
        std::fs::write(&source, b"new-photo").unwrap();
        std::fs::write(&dest, b"keep-me").unwrap();

        let fs = LocalFileSystem;
        let err = fs.copy_exclusive(&source, &dest).await.unwrap_err();
        assert!(is_already_exists(&err), "{err}");
        assert_eq!(std::fs::read(&dest).unwrap(), b"keep-me");
        assert_eq!(std::fs::read(&source).unwrap(), b"new-photo");
    }

    #[tokio::test]
    async fn copy_exclusive_writes_a_free_name_including_non_ascii() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.jpg");
        std::fs::write(&source, "写真".as_bytes()).unwrap();
        let dest = dir.path().join("写真.JPG");

        let fs = LocalFileSystem;
        let bytes = fs.copy_exclusive(&source, &dest).await.unwrap();
        assert_eq!(bytes, "写真".len() as u64);
        assert_eq!(std::fs::read(&dest).unwrap(), "写真".as_bytes());

        let err = fs.copy_exclusive(&source, &dest).await.unwrap_err();
        assert!(is_already_exists(&err), "{err}");
        assert_eq!(std::fs::read(&dest).unwrap(), "写真".as_bytes());
    }

    #[tokio::test]
    async fn rename_exclusive_does_not_overwrite_and_keeps_the_source() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.jpg");
        let dest = dir.path().join("IMG_0001.JPG");
        std::fs::write(&source, b"moving").unwrap();
        std::fs::write(&dest, b"already-there").unwrap();

        let fs = LocalFileSystem;
        let err = fs.rename_exclusive(&source, &dest).await.unwrap_err();
        assert!(is_already_exists(&err), "{err}");
        assert_eq!(std::fs::read(&dest).unwrap(), b"already-there");
        assert_eq!(std::fs::read(&source).unwrap(), b"moving");
    }

    #[tokio::test]
    async fn rename_exclusive_moves_when_the_name_is_free() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.jpg");
        let dest = dir.path().join("写真.JPG");
        std::fs::write(&source, b"payload").unwrap();

        let fs = LocalFileSystem;
        fs.rename_exclusive(&source, &dest).await.unwrap();
        assert!(!source.exists());
        assert_eq!(std::fs::read(&dest).unwrap(), b"payload");
    }

    #[tokio::test]
    async fn list_dir_names_returns_existing_entries_and_empty_when_missing() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("IMG_0001.JPG"), b"a").unwrap();
        std::fs::write(dir.path().join("写真.JPG"), b"b").unwrap();

        let fs = LocalFileSystem;
        let mut names = fs.list_dir_names(dir.path()).await.unwrap();
        names.sort();
        assert_eq!(
            names,
            vec![
                std::ffi::OsString::from("IMG_0001.JPG"),
                std::ffi::OsString::from("写真.JPG"),
            ]
        );

        let missing = dir.path().join("nope");
        assert!(fs.list_dir_names(&missing).await.unwrap().is_empty());
    }

    fn is_already_exists(err: &GalleryError) -> bool {
        matches!(err, GalleryError::Io(err) if err.kind() == ErrorKind::AlreadyExists)
    }

    #[tokio::test]
    async fn sync_file_flushes_and_remove_file_is_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("photo.jpg");
        std::fs::write(&path, b"payload").unwrap();

        let fs = LocalFileSystem;
        fs.sync_file(&path).await.unwrap();
        let meta = fs.get_file_metadata(&path).await.unwrap();
        assert_eq!(meta.size, b"payload".len() as u64);

        fs.remove_file(&path).await.unwrap();
        assert!(!path.exists());
        fs.remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn copy_exclusive_reports_the_source_size_that_sync_can_see() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.jpg");
        let dest = dir.path().join("dest.jpg");
        std::fs::write(&source, b"across-device-bytes").unwrap();

        let fs = LocalFileSystem;
        let copied = fs.copy_exclusive(&source, &dest).await.unwrap();
        fs.sync_file(&dest).await.unwrap();
        let src_len = fs.get_file_metadata(&source).await.unwrap().size;
        let dest_len = fs.get_file_metadata(&dest).await.unwrap().size;
        assert_eq!(copied, src_len);
        assert_eq!(dest_len, src_len);
        assert_eq!(
            std::fs::read(&source).unwrap(),
            std::fs::read(&dest).unwrap()
        );
    }
}

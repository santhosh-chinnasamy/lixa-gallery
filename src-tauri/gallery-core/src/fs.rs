use crate::models::{FileMetadata, Result};
use async_trait::async_trait;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn get_file_metadata(&self, path: &Path) -> Result<FileMetadata>;
    async fn list_images_in_dir(&self, dir: &Path) -> Result<Vec<PathBuf>>;
    async fn list_subfolders(&self, dir: &Path) -> Result<Vec<PathBuf>>;
    async fn copy(&self, from: &Path, to: &Path) -> Result<u64>;
    async fn rename(&self, from: &Path, to: &Path) -> Result<()>;
    async fn canonicalize(&self, path: &Path) -> Result<PathBuf>;
    async fn create_dir_all(&self, path: &Path) -> Result<()>;

    /// Entry names already present in `dir`. A missing directory is an empty list.
    async fn list_dir_names(&self, dir: &Path) -> Result<Vec<OsString>>;

    /// Copy `from` onto `to`, failing if `to` already exists.
    ///
    /// The destination is created with `create_new` semantics, so an existing
    /// file is left untouched.
    async fn copy_exclusive(&self, from: &Path, to: &Path) -> Result<u64>;

    /// Move `from` onto `to`, failing if `to` already exists.
    ///
    /// An existing destination is never replaced. Cross-device moves still fail;
    /// the source stays in place.
    async fn rename_exclusive(&self, from: &Path, to: &Path) -> Result<()>;
}

/// Identity used to decide that two file names would collide on a
/// case-insensitive volume (macOS / Windows) and for non-ASCII names.
pub fn collision_key(name: &OsStr) -> String {
    name.to_string_lossy().to_lowercase()
}

/// `IMG_0001.JPG` at index 0, `IMG_0001 (1).JPG` at index 1, and so on.
///
/// The original stem and extension are preserved, including case and non-ASCII
/// characters. Index 0 returns `original` unchanged.
pub fn uniquified_file_name(original: &OsStr, index: u32) -> OsString {
    if index == 0 {
        return original.to_os_string();
    }

    let path = Path::new(original);
    let stem = path.file_stem().unwrap_or(original);
    let mut name = OsString::from(stem);
    name.push(format!(" ({index})"));
    if let Some(extension) = path.extension() {
        name.push(".");
        name.push(extension);
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_preserves_extension_case() {
        assert_eq!(
            uniquified_file_name(OsStr::new("IMG_0001.JPG"), 0),
            OsStr::new("IMG_0001.JPG")
        );
        assert_eq!(
            uniquified_file_name(OsStr::new("IMG_0001.JPG"), 1),
            OsStr::new("IMG_0001 (1).JPG")
        );
        assert_eq!(
            uniquified_file_name(OsStr::new("IMG_0001.JPG"), 2),
            OsStr::new("IMG_0001 (2).JPG")
        );
    }

    #[test]
    fn suffix_handles_unicode_dotfiles_and_no_extension() {
        assert_eq!(
            uniquified_file_name(OsStr::new("写真.JPG"), 1),
            OsStr::new("写真 (1).JPG")
        );
        assert_eq!(
            uniquified_file_name(OsStr::new("README"), 1),
            OsStr::new("README (1)")
        );
        assert_eq!(
            uniquified_file_name(OsStr::new(".hidden"), 1),
            OsStr::new(".hidden (1)")
        );
        assert_eq!(
            uniquified_file_name(OsStr::new("archive.tar.gz"), 1),
            OsStr::new("archive.tar (1).gz")
        );
    }

    #[test]
    fn collision_key_folds_ascii_and_non_ascii_case() {
        assert_eq!(
            collision_key(OsStr::new("IMG_0001.JPG")),
            collision_key(OsStr::new("img_0001.jpg"))
        );
        assert_eq!(
            collision_key(OsStr::new("CAFÉ.JPG")),
            collision_key(OsStr::new("café.jpg"))
        );
        assert_ne!(
            collision_key(OsStr::new("IMG_0001.JPG")),
            collision_key(OsStr::new("IMG_0001 (1).JPG"))
        );
    }
}

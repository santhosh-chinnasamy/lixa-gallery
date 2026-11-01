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
    /// An existing destination is never replaced. A cross-device move fails
    /// with the operating-system error and leaves the source in place. The
    /// export loop falls back to [`FileSystem::copy_exclusive`] when that
    /// happens.
    async fn rename_exclusive(&self, from: &Path, to: &Path) -> Result<()>;

    /// Byte length of `path`. This does not read timestamps: creation time is
    /// missing on some external drives, and a cross-device move must still be
    /// able to compare sizes.
    async fn file_len(&self, path: &Path) -> Result<u64>;

    /// Flush `path` to stable storage. Missing files are an error.
    async fn sync_file(&self, path: &Path) -> Result<()>;

    /// Delete `path`. A missing file is success, so a partial copy can be
    /// cleaned up more than once.
    async fn remove_file(&self, path: &Path) -> Result<()>;
}

/// Unix `EXDEV`. On Windows this number is a different error, so it is only
/// treated as cross-device on Unix.
pub const EXDEV: i32 = 18;

/// Windows `ERROR_NOT_SAME_DEVICE`. On Unix this number is `EEXIST`, so it is
/// only treated as cross-device on Windows.
pub const ERROR_NOT_SAME_DEVICE: i32 = 17;

/// `rename` across drives: `EXDEV` on Unix, `ERROR_NOT_SAME_DEVICE` on Windows.
///
/// `ErrorKind::CrossesDevices` is the portable signal. The raw codes are
/// checked as well, and only the code that means "different device" on this
/// platform, because the two numbers collide with unrelated errors on the
/// other platform.
pub fn is_cross_device(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::CrossesDevices {
        return true;
    }
    err.raw_os_error().is_some_and(raw_os_error_is_cross_device)
}

pub fn raw_os_error_is_cross_device(code: i32) -> bool {
    if cfg!(windows) {
        code == ERROR_NOT_SAME_DEVICE
    } else {
        code == EXDEV
    }
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
    fn cross_device_errors_map_to_the_platform_code() {
        let by_kind = std::io::Error::new(std::io::ErrorKind::CrossesDevices, "cross-device link");
        assert!(is_cross_device(&by_kind));

        let permission = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Permission denied (os error 13)",
        );
        assert!(!is_cross_device(&permission));

        // Unix EXDEV is 18. Windows ERROR_NOT_SAME_DEVICE is 17.
        // `from_raw_os_error` follows the host, so the other platform's number
        // must not be classified as cross-device here.
        assert!(is_cross_device(&std::io::Error::from_raw_os_error(
            if cfg!(windows) {
                ERROR_NOT_SAME_DEVICE
            } else {
                EXDEV
            }
        )));
        assert!(!is_cross_device(&std::io::Error::from_raw_os_error(
            if cfg!(windows) {
                EXDEV
            } else {
                ERROR_NOT_SAME_DEVICE
            }
        )));

        if cfg!(windows) {
            assert!(raw_os_error_is_cross_device(ERROR_NOT_SAME_DEVICE));
            assert!(!raw_os_error_is_cross_device(EXDEV));
        } else {
            assert!(raw_os_error_is_cross_device(EXDEV));
            assert!(!raw_os_error_is_cross_device(ERROR_NOT_SAME_DEVICE));
        }
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

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GalleryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Db(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Image processing error: {0}")]
    Image(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LoadingMode {
    Sync,
    Lazy,
}

pub type Result<T> = std::result::Result<T, GalleryError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub name: String,
    pub modified: u64,
    pub created: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoMetadata {
    pub metadata: FileMetadata,
    pub thumbnail_path: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderNode {
    pub name: String,
    pub path: String,
    pub children: Vec<FolderNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favourite {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavouriteFolderGroup {
    pub folder_path: String,
    pub folder_name: String,
    pub count: usize,
    pub photos: Vec<PhotoMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub destination: String,
    pub mode: String, // "copy" | "move"
    pub paths: Option<Vec<String>>,
    #[serde(default)]
    pub preserve_folder_structure: bool,
}

/// One file that could not be exported. `reason` is the underlying error
/// (for example "Permission denied" or "No space left on device"), not a
/// generic hint.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportFailure {
    pub path: String,
    pub reason: String,
}

/// Outcome of one export. `copied` counts files transferred under their
/// original file name (copy or move). `renamed` counts files written as
/// `name (n).ext` because that name was already taken. `skipped` counts files
/// left untouched because the source and destination were the same file.
/// `failed` matches `failures`. `cancelled` is set when the user stopped the
/// export; counts are only the files finished before the stop. `total` is the
/// number of files in the job.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportReport {
    pub copied: u32,
    pub renamed: u32,
    pub skipped: u32,
    pub failed: u32,
    pub failures: Vec<ExportFailure>,
    pub cancelled: bool,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub created_at: i64,
    pub last_opened_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_photo_metadata_serialization() {
        let metadata = PhotoMetadata {
            metadata: FileMetadata {
                name: "test.jpg".to_string(),
                modified: 12345678,
                created: 12345670,
                size: 1024,
            },
            thumbnail_path: "/tmp/thumb.webp".to_string(),
            path: "/home/user/test.jpg".to_string(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: PhotoMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.metadata.name, "test.jpg");
        assert_eq!(deserialized.path, "/home/user/test.jpg");
    }

    #[test]
    fn test_favourite_serialization() {
        let fav = Favourite {
            path: "/path/to/fav".to_string(),
        };
        let json = serde_json::to_string(&fav).unwrap();
        let deserialized: Favourite = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, "/path/to/fav");
    }

    #[test]
    fn export_report_keeps_per_file_failures_and_cancellation() {
        let report = ExportReport {
            copied: 12,
            renamed: 0,
            skipped: 0,
            failed: 1,
            failures: vec![ExportFailure {
                path: "/card/IMG.JPG".to_string(),
                reason: "Permission denied (os error 13)".to_string(),
            }],
            cancelled: true,
            total: 40,
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: ExportReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back, report);
        assert!(json.contains("Permission denied (os error 13)"));
    }
}

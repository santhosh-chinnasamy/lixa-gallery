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

/// Shooting settings read once from the file and cached on the photo row.
/// Missing tags stay unset so the info panel can hide them.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ExifInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lens: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focal_length: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aperture: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shutter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iso: Option<u32>,
    /// Camera clock as written, with no timezone conversion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// `DateTimeOriginal` as a unix second, for sorting. Naive camera time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<i64>,
    /// Fractional second of `DateTimeOriginal`, in microseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_subsec: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Short label when the file is tagged with a non-upright orientation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
}

impl ExifInfo {
    pub fn is_empty(&self) -> bool {
        self.camera.is_none()
            && self.lens.is_none()
            && self.focal_length.is_none()
            && self.aperture.is_none()
            && self.shutter.is_none()
            && self.iso.is_none()
            && self.date.is_none()
            && self.width.is_none()
            && self.height.is_none()
            && self.orientation.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoMetadata {
    pub metadata: FileMetadata,
    pub thumbnail_path: String,
    pub path: String,
    /// RAW sibling of a JPEG, or the JPEG sibling when the primary is the RAW.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sidecar_path: Option<String>,
    /// JPEG the viewer should open instead of `path` (embedded RAW preview).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_path: Option<String>,
    /// `Some` after a read, including an empty struct when the file had no tags.
    /// `None` means this process has not read the file yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exif: Option<ExifInfo>,
    #[serde(default)]
    pub is_raw: bool,
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
            sidecar_path: None,
            preview_path: None,
            exif: None,
            is_raw: false,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: PhotoMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.metadata.name, "test.jpg");
        assert_eq!(deserialized.path, "/home/user/test.jpg");
        assert!(deserialized.exif.is_none());
        assert!(!deserialized.is_raw);
    }

    #[test]
    fn photo_metadata_from_older_rows_defaults_the_new_fields() {
        let json = r#"{
            "metadata": {"name": "old.jpg", "modified": 1, "created": 1, "size": 2},
            "thumbnail_path": "/t.webp",
            "path": "/old.jpg"
        }"#;
        let photo: PhotoMetadata = serde_json::from_str(json).unwrap();
        assert!(photo.sidecar_path.is_none());
        assert!(photo.preview_path.is_none());
        assert!(photo.exif.is_none());
        assert!(!photo.is_raw);
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
}

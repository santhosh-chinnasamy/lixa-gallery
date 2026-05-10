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
}

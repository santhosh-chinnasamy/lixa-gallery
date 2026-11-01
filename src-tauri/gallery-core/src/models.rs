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

/// Pick / reject / unflagged for a single photo.
///
/// Stored the same way favourites were: one SQLite row keyed by path.
/// A missing row is [`CullFlag::Unflagged`]. Picked and rejected cannot both
/// be set, because this is one enum rather than two booleans.
///
/// Sidecar export (issue #90) should map through [`CullFlag::to_xmp_pick`]
/// (`1` picked, `0` unflagged, `-1` rejected). That integer is the Lightroom
/// pick flag, not a star rating. A rating or colour label belongs in its own
/// column beside this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CullFlag {
    Unflagged,
    Picked,
    Rejected,
}

impl CullFlag {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unflagged => "unflagged",
            Self::Picked => "picked",
            Self::Rejected => "rejected",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "unflagged" => Some(Self::Unflagged),
            "picked" => Some(Self::Picked),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }

    /// Lightroom Classic `xmp:Pick` / `crs:Pick`: `1`, `0`, or `-1`.
    pub fn to_xmp_pick(self) -> i32 {
        match self {
            Self::Picked => 1,
            Self::Unflagged => 0,
            Self::Rejected => -1,
        }
    }

    /// Inverse of [`CullFlag::to_xmp_pick`]. Any other integer is unflagged,
    /// so a star rating is not accidentally read as a pick.
    pub fn from_xmp_pick(value: i32) -> Self {
        match value {
            1 => Self::Picked,
            -1 => Self::Rejected,
            _ => Self::Unflagged,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhotoCullState {
    pub path: String,
    pub flag: CullFlag,
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
    fn cull_flag_round_trips_and_maps_to_xmp_pick() {
        for flag in [CullFlag::Unflagged, CullFlag::Picked, CullFlag::Rejected] {
            let parsed = CullFlag::parse(flag.as_str()).unwrap();
            assert_eq!(parsed, flag);
            assert_eq!(CullFlag::from_xmp_pick(flag.to_xmp_pick()), flag);

            let state = PhotoCullState {
                path: "/photo.jpg".to_string(),
                flag,
            };
            let json = serde_json::to_string(&state).unwrap();
            let back: PhotoCullState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, state);
        }

        assert_eq!(CullFlag::Picked.to_xmp_pick(), 1);
        assert_eq!(CullFlag::Unflagged.to_xmp_pick(), 0);
        assert_eq!(CullFlag::Rejected.to_xmp_pick(), -1);
        // A star rating must not become a pick.
        assert_eq!(CullFlag::from_xmp_pick(5), CullFlag::Unflagged);
        assert_eq!(CullFlag::parse("nope"), None);
    }
}

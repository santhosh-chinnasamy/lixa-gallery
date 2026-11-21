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

    /// Keep existing fields; fill only gaps from `other`.
    pub fn merge_gaps_from(&mut self, other: &ExifInfo) {
        fn take<T: Clone>(slot: &mut Option<T>, extra: &Option<T>) {
            if slot.is_none() {
                *slot = extra.clone();
            }
        }
        take(&mut self.camera, &other.camera);
        take(&mut self.lens, &other.lens);
        take(&mut self.focal_length, &other.focal_length);
        take(&mut self.aperture, &other.aperture);
        take(&mut self.shutter, &other.shutter);
        take(&mut self.iso, &other.iso);
        take(&mut self.date, &other.date);
        take(&mut self.captured_at, &other.captured_at);
        take(&mut self.captured_subsec, &other.captured_subsec);
        take(&mut self.width, &other.width);
        take(&mut self.height, &other.height);
        take(&mut self.orientation, &other.orientation);
    }
}

/// Payload for `photos-updated`. Generation lets the UI drop stale folder work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotosUpdatedEvent {
    pub generation: u64,
    pub photos: Vec<PhotoMetadata>,
}

/// Return value of `scan_folder` so the UI can bind to the folder session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderScanResult {
    pub generation: u64,
    pub photos: Vec<PhotoMetadata>,
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
    /// Subfolders exist. `children` stays empty until that level is requested.
    #[serde(default)]
    pub has_children: bool,
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

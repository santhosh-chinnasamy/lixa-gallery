//! Image extensions and RAW+JPEG pairing.
//!
//! Pairing is a directory listing concern: one card per stem, and export
//! copies both files. It does not decode anything.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Extensions whose pixels the webview cannot decode. The app shows the
/// embedded JPEG preview instead of demosaicing the sensor data.
pub fn is_raw_extension(ext: &str) -> bool {
    matches!(
        ext,
        "3fr"
            | "arw"
            | "cr2"
            | "cr3"
            | "crw"
            | "dcr"
            | "dng"
            | "erf"
            | "fff"
            | "iiq"
            | "kdc"
            | "mef"
            | "mos"
            | "nef"
            | "nrw"
            | "orf"
            | "pef"
            | "raf"
            | "raw"
            | "rw2"
            | "rwl"
            | "sr2"
            | "srf"
            | "srw"
            | "x3f"
    )
}

pub fn is_jpeg_extension(ext: &str) -> bool {
    matches!(ext, "jpg" | "jpeg")
}

pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

pub fn is_raw_path(path: &Path) -> bool {
    is_raw_extension(&extension(path))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageGroup {
    /// File the grid and viewer open. A JPEG when a RAW sibling shares the stem.
    pub primary: PathBuf,
    /// The other half of a RAW+JPEG pair.
    pub sidecar: Option<PathBuf>,
}

/// Collapse RAW+JPEG siblings that share a directory and file stem into one item.
///
/// The JPEG is the primary so the webview can paint it directly. Any file that
/// is not part of such a pair is returned on its own, in first-seen order.
pub fn group_raw_jpeg_pairs(paths: Vec<PathBuf>) -> Vec<ImageGroup> {
    let mut order = Vec::new();
    let mut grouped: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for path in paths {
        let key = pair_key(&path);
        if !grouped.contains_key(&key) {
            order.push(key.clone());
        }
        grouped.entry(key).or_default().push(path);
    }

    let mut items = Vec::new();
    for key in order {
        if let Some(files) = grouped.remove(&key) {
            items.extend(collapse_stem(files));
        }
    }
    items
}

fn pair_key(path: &Path) -> String {
    let parent = path
        .parent()
        .map(|parent| parent.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    format!("{parent}\u{0}{stem}")
}

fn collapse_stem(files: Vec<PathBuf>) -> Vec<ImageGroup> {
    if files.len() == 1 {
        return vec![ImageGroup {
            primary: files.into_iter().next().expect("len checked"),
            sidecar: None,
        }];
    }

    let mut raws = Vec::new();
    let mut jpegs = Vec::new();
    let mut others = Vec::new();
    for file in files {
        let ext = extension(&file);
        if is_raw_extension(&ext) {
            raws.push(file);
        } else if is_jpeg_extension(&ext) {
            jpegs.push(file);
        } else {
            others.push(file);
        }
    }

    let mut items = Vec::new();
    if !jpegs.is_empty() && !raws.is_empty() {
        items.push(ImageGroup {
            primary: jpegs.remove(0),
            sidecar: Some(raws.remove(0)),
        });
    }
    for file in jpegs.into_iter().chain(raws).chain(others) {
        items.push(ImageGroup {
            primary: file,
            sidecar: None,
        });
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_extensions_cover_the_common_camera_formats() {
        for ext in ["cr2", "cr3", "nef", "arw", "raf", "orf", "rw2", "dng"] {
            assert!(is_raw_extension(ext), "{ext}");
        }
        assert!(!is_raw_extension("jpg"));
        assert!(!is_raw_extension("tiff"));
    }

    #[test]
    fn pairs_raw_and_jpeg_with_the_same_stem_ignoring_case() {
        let items = group_raw_jpeg_pairs(vec![
            PathBuf::from("/Shoot/IMG_0001.CR2"),
            PathBuf::from("/Shoot/IMG_0001.jpg"),
            PathBuf::from("/Shoot/solo.NEF"),
            PathBuf::from("/Shoot/notes.png"),
            PathBuf::from("/Other/IMG_0001.jpg"),
        ]);

        assert_eq!(items.len(), 4);
        assert_eq!(items[0].primary, PathBuf::from("/Shoot/IMG_0001.jpg"));
        assert_eq!(
            items[0].sidecar.as_deref(),
            Some(Path::new("/Shoot/IMG_0001.CR2"))
        );
        assert_eq!(items[1].primary, PathBuf::from("/Shoot/solo.NEF"));
        assert!(items[1].sidecar.is_none());
        assert!(is_raw_path(&items[1].primary));
        assert_eq!(items[2].primary, PathBuf::from("/Shoot/notes.png"));
        assert_eq!(items[3].primary, PathBuf::from("/Other/IMG_0001.jpg"));
        assert!(items[3].sidecar.is_none());
    }

    #[test]
    fn leaves_unrelated_files_that_share_a_stem_alone() {
        let items = group_raw_jpeg_pairs(vec![
            PathBuf::from("/a/same.png"),
            PathBuf::from("/a/same.jpg"),
        ]);
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|item| item.sidecar.is_none()));
    }
}

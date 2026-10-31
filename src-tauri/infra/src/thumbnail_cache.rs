//! Thumbnail filenames for the on-disk cache.
//!
//! v1 named every thumbnail `{file_stem}.webp` in one directory, so
//! `A/IMG_0001.JPG` and `B/IMG_0001.JPG` (and `a.jpg` / `a.png`) shared a file.
//! The protocol then served that file whenever it existed, including after the
//! source was edited.
//!
//! v2 names the file from a versioned hash of the normalized source path, plus
//! the source mtime (seconds) and size. The same photo always maps to one
//! path; a different photo, extension, or edit maps to another. Edited copies
//! replace the previous file for that path. Startup deletes the v1 files so
//! they are not reused and do not accumulate next to the new cache.

use gallery_core::thumbnail_cache::{
    identity_suffix, stored_thumbnail_is_current, CACHE_DIR_NAME, CACHE_KEY_HEX_LEN, CACHE_VERSION,
};
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

/// mtime (seconds since the Unix epoch) and length of the source file.
///
/// These are the same values stored on the photo row. A cache hit is valid
/// only while both still match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThumbnailIdentity {
    pub mtime_secs: u64,
    pub size: u64,
}

impl ThumbnailIdentity {
    pub fn from_metadata(metadata: &std::fs::Metadata) -> std::io::Result<Self> {
        let modified = metadata.modified()?;
        let mtime_secs = modified
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        Ok(Self {
            mtime_secs,
            size: metadata.len(),
        })
    }

    pub fn read(path: &Path) -> std::io::Result<Self> {
        Self::from_metadata(&std::fs::metadata(path)?)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PurgeStats {
    pub removed_entries: u64,
    pub removed_bytes: u64,
    pub errors: u64,
}

/// Absolute path of the thumbnail for `source` at `identity`.
pub fn thumbnail_disk_path(
    cache_dir: &Path,
    source: &Path,
    identity: ThumbnailIdentity,
) -> PathBuf {
    let relative = relative_cache_path(&source_key_material(source), identity);
    let full = cache_dir.join(relative);
    debug_assert!(stored_thumbnail_is_current(
        &full.to_string_lossy(),
        identity.mtime_secs as i64,
        identity.size as i64,
    ));
    full
}

/// Drop a trailing `?v=<mtime>_<size>` cache-buster added by the webview.
///
/// The source path is percent-encoded, so a literal `?` is not part of the
/// filename the UI sends. Anything that is not that exact suffix is left
/// unchanged.
pub fn strip_cache_buster(encoded_path: &str) -> &str {
    let Some(index) = encoded_path.rfind("?v=") else {
        return encoded_path;
    };
    let token = &encoded_path[index + 3..];
    if is_cache_buster_token(token) {
        &encoded_path[..index]
    } else {
        encoded_path
    }
}

/// Delete thumbnails from older cache schemes.
///
/// Flat `*.webp` files in `cache_dir` are the v1 stem cache. Directories named
/// `vN` with `N < CACHE_VERSION` are retired version folders. The current
/// version directory is not walked. Symlinks are unlinked and never followed,
/// so a link that points outside the cache cannot delete that target.
pub fn purge_obsolete_thumbnails(cache_dir: &Path) -> std::io::Result<PurgeStats> {
    let mut stats = PurgeStats::default();
    if !cache_dir.exists() {
        return Ok(stats);
    }

    let entries = std::fs::read_dir(cache_dir)?;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                stats.errors += 1;
                continue;
            }
        };
        let path = entry.path();
        if !path.starts_with(cache_dir) {
            stats.errors += 1;
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => {
                stats.errors += 1;
                continue;
            }
        };

        let action = obsolete_action(&name, &file_type);
        if action == ObsoleteAction::Keep {
            continue;
        }
        let bytes = match action {
            ObsoleteAction::Unlink => path_bytes(&path),
            ObsoleteAction::RemoveTree => tree_bytes(&path),
            ObsoleteAction::Keep => 0,
        };
        let removed = match action {
            ObsoleteAction::Unlink => std::fs::remove_file(&path),
            ObsoleteAction::RemoveTree => std::fs::remove_dir_all(&path),
            ObsoleteAction::Keep => Ok(()),
        };
        match removed {
            Ok(()) => {
                stats.removed_entries += 1;
                stats.removed_bytes += bytes;
            }
            Err(_) => stats.errors += 1,
        }
    }
    Ok(stats)
}

/// Delete other `{path_key}_*.webp` files next to `current`.
///
/// An edit changes mtime or size and therefore the filename. The previous
/// file would otherwise stay in the shard forever.
pub(crate) fn remove_other_identity_files(current: &Path) -> std::io::Result<u64> {
    let Some(name) = current.file_name().and_then(|value| value.to_str()) else {
        return Ok(0);
    };
    let Some(prefix) = variant_prefix(name) else {
        return Ok(0);
    };
    let Some(parent) = current.parent() else {
        return Ok(0);
    };
    if !parent.is_dir() {
        return Ok(0);
    }

    let mut removed = 0u64;
    for entry in std::fs::read_dir(parent)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                log::warn!("thumbnail cache entry: {err}");
                continue;
            }
        };
        let path = entry.path();
        if path == current {
            continue;
        }
        let Some(other) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !other.starts_with(prefix) || !other.ends_with(".webp") || !path.is_file() {
            continue;
        }
        if let Err(err) = std::fs::remove_file(&path) {
            log::warn!("could not remove stale thumbnail {}: {err}", path.display());
        } else {
            removed += 1;
        }
    }
    Ok(removed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PathStyle {
    Windows,
    MacOs,
    Linux,
}

fn current_style() -> PathStyle {
    if cfg!(windows) {
        PathStyle::Windows
    } else if cfg!(target_os = "macos") {
        PathStyle::MacOs
    } else {
        PathStyle::Linux
    }
}

fn source_key_material(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    normalize_for_key(&path_as_key_string(&canonical), current_style())
}

fn path_as_key_string(path: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        match path.to_str() {
            Some(text) => text.to_owned(),
            None => {
                let mut encoded = String::from("unix-bytes:");
                for byte in path.as_os_str().as_bytes() {
                    encoded.push_str(&format!("{byte:02x}"));
                }
                encoded
            }
        }
    }
    #[cfg(not(unix))]
    {
        path.to_string_lossy().into_owned()
    }
}

fn relative_cache_path(normalized: &str, identity: ThumbnailIdentity) -> PathBuf {
    let key = path_key(normalized);
    let shard = key.get(..2).expect("blake3 hex key");
    let name = format!(
        "{key}{}",
        identity_suffix(identity.mtime_secs, identity.size)
    );
    PathBuf::from(CACHE_DIR_NAME).join(shard).join(name)
}

fn path_key(normalized: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(hash_domain().as_bytes());
    hasher.update(normalized.as_bytes());
    let key = hasher.finalize().to_hex().to_string();
    debug_assert_eq!(key.len(), CACHE_KEY_HEX_LEN);
    key
}

fn hash_domain() -> String {
    format!("lixa-gallery-thumbnail-v{CACHE_VERSION}\0")
}

fn normalize_for_key(raw: &str, style: PathStyle) -> String {
    let nfc;
    let text = if style == PathStyle::MacOs {
        nfc = raw.nfc().collect::<String>();
        nfc.as_str()
    } else {
        raw
    };
    let fold = matches!(style, PathStyle::Windows | PathStyle::MacOs);
    let (prefix, rest, rooted) = split_root(text, style);
    let prefix = if fold { prefix.to_lowercase() } else { prefix };

    let mut parts = Vec::new();
    for component in split_components(rest, style == PathStyle::Windows) {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            match parts.last().map(String::as_str) {
                Some("..") => parts.push("..".to_string()),
                Some(_) => {
                    parts.pop();
                }
                None if !rooted => parts.push("..".to_string()),
                None => {}
            }
            continue;
        }
        parts.push(if fold {
            component.to_lowercase()
        } else {
            component.to_string()
        });
    }

    let body = parts.join("/");
    if !prefix.is_empty() {
        if body.is_empty() {
            prefix
        } else {
            format!("{prefix}/{body}")
        }
    } else if rooted {
        if body.is_empty() {
            "/".to_string()
        } else {
            format!("/{body}")
        }
    } else {
        body
    }
}

fn split_components(rest: &str, backslash_is_separator: bool) -> impl Iterator<Item = &str> {
    rest.split(move |character| character == '/' || (backslash_is_separator && character == '\\'))
}

fn split_root(text: &str, style: PathStyle) -> (String, &str, bool) {
    match style {
        PathStyle::Linux | PathStyle::MacOs => {
            if text.starts_with('/') {
                (String::new(), text, true)
            } else {
                (String::new(), text, false)
            }
        }
        PathStyle::Windows => split_windows_root(text),
    }
}

fn split_windows_root(text: &str) -> (String, &str, bool) {
    let (after_verbatim, verbatim) = strip_verbatim_prefix(text);
    if verbatim && starts_with_unc_device(after_verbatim) {
        return unc_prefix(&after_verbatim[4..]);
    }
    if starts_with_double_sep(after_verbatim) {
        return unc_prefix(trim_separators(after_verbatim));
    }
    let bytes = after_verbatim.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let rooted = bytes.len() == 2 || is_sep(bytes[2]);
        if rooted {
            let rest = if bytes.len() == 2 {
                ""
            } else {
                &after_verbatim[2..]
            };
            return (after_verbatim[..2].to_string(), rest, true);
        }
    }
    (String::new(), after_verbatim, false)
}

fn strip_verbatim_prefix(text: &str) -> (&str, bool) {
    let bytes = text.as_bytes();
    if bytes.len() >= 4
        && is_sep(bytes[0])
        && is_sep(bytes[1])
        && bytes[2] == b'?'
        && is_sep(bytes[3])
    {
        (&text[4..], true)
    } else {
        (text, false)
    }
}

fn starts_with_unc_device(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 4 && bytes[..3].eq_ignore_ascii_case(b"unc") && is_sep(bytes[3])
}

fn starts_with_double_sep(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 2 && is_sep(bytes[0]) && is_sep(bytes[1])
}

fn trim_separators(text: &str) -> &str {
    text.trim_start_matches(['\\', '/'])
}

fn is_sep(byte: u8) -> bool {
    byte == b'\\' || byte == b'/'
}

fn unc_prefix(text: &str) -> (String, &str, bool) {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() && !is_sep(bytes[index]) {
        index += 1;
    }
    let server = &text[..index];
    while index < bytes.len() && is_sep(bytes[index]) {
        index += 1;
    }
    let share_start = index;
    while index < bytes.len() && !is_sep(bytes[index]) {
        index += 1;
    }
    let share = &text[share_start..index];
    (format!("//{server}/{share}"), &text[index..], true)
}

fn is_cache_buster_token(token: &str) -> bool {
    let Some((mtime, size)) = token.split_once('_') else {
        return false;
    };
    !mtime.is_empty()
        && !size.is_empty()
        && mtime.bytes().all(|byte| byte.is_ascii_digit())
        && size.bytes().all(|byte| byte.is_ascii_digit())
}

fn variant_prefix(file_name: &str) -> Option<&str> {
    let stem = file_name.strip_suffix(".webp")?;
    let (hash, rest) = stem.split_once('_')?;
    if hash.len() != CACHE_KEY_HEX_LEN || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let (mtime, size) = rest.split_once('_')?;
    if mtime.is_empty()
        || size.is_empty()
        || !mtime.bytes().all(|byte| byte.is_ascii_digit())
        || !size.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some(&file_name[..hash.len() + 1])
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ObsoleteAction {
    Keep,
    Unlink,
    RemoveTree,
}

fn obsolete_action(name: &str, file_type: &std::fs::FileType) -> ObsoleteAction {
    if file_type.is_symlink() {
        if is_flat_webp(name) || is_obsolete_version_dir(name) {
            ObsoleteAction::Unlink
        } else {
            ObsoleteAction::Keep
        }
    } else if file_type.is_dir() {
        if is_obsolete_version_dir(name) {
            ObsoleteAction::RemoveTree
        } else {
            ObsoleteAction::Keep
        }
    } else if file_type.is_file() && is_flat_webp(name) {
        ObsoleteAction::Unlink
    } else {
        ObsoleteAction::Keep
    }
}

fn is_flat_webp(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("webp"))
}

fn is_obsolete_version_dir(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('v') else {
        return false;
    };
    match rest.parse::<u32>() {
        Ok(version) => version < CACHE_VERSION,
        Err(_) => false,
    }
}

fn path_bytes(path: &Path) -> u64 {
    std::fs::symlink_metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

fn tree_bytes(path: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if !metadata.is_dir() {
        return metadata.len();
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| tree_bytes(&entry.path()))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn identity(mtime_secs: u64, size: u64) -> ThumbnailIdentity {
        ThumbnailIdentity { mtime_secs, size }
    }

    fn linux(path: &str) -> String {
        normalize_for_key(path, PathStyle::Linux)
    }

    fn mac(path: &str) -> String {
        normalize_for_key(path, PathStyle::MacOs)
    }

    fn windows(path: &str) -> String {
        normalize_for_key(path, PathStyle::Windows)
    }

    #[test]
    fn legacy_stem_names_collide_and_hashed_keys_do_not() {
        let left = Path::new("/shoots/a/IMG_0001.JPG");
        let right = Path::new("/shoots/b/IMG_0001.JPG");
        let left_stem = left.file_stem().unwrap().to_string_lossy().into_owned();
        let right_stem = right.file_stem().unwrap().to_string_lossy().into_owned();
        assert_eq!(left_stem, right_stem, "the old cache key collides");

        let id = identity(1_700_000_000, 4096);
        let left_key = relative_cache_path(&linux("/shoots/a/IMG_0001.JPG"), id);
        let right_key = relative_cache_path(&linux("/shoots/b/IMG_0001.JPG"), id);
        assert_ne!(left_key, right_key);
        assert_ne!(
            left_key.file_name().unwrap(),
            std::ffi::OsStr::new("IMG_0001.webp")
        );
    }

    #[test]
    fn same_stem_with_different_extensions_do_not_collide() {
        let id = identity(10, 20);
        let jpg = relative_cache_path(&linux("/card/a.jpg"), id);
        let png = relative_cache_path(&linux("/card/a.png"), id);
        assert_ne!(jpg, png);
    }

    #[test]
    fn mtime_or_size_change_changes_the_filename_only() {
        let normalized = linux("/card/IMG_0001.JPG");
        let original = relative_cache_path(&normalized, identity(10, 20));
        let edited_mtime = relative_cache_path(&normalized, identity(11, 20));
        let edited_size = relative_cache_path(&normalized, identity(10, 21));
        assert_ne!(original, edited_mtime);
        assert_ne!(original, edited_size);
        assert_eq!(original.parent(), edited_mtime.parent());
        assert_eq!(relative_cache_path(&normalized, identity(10, 20)), original);
    }

    #[test]
    fn windows_separators_case_and_verbatim_prefixes_share_a_key() {
        let expected = "c:/photos/img_0001.jpg";
        assert_eq!(windows(r"C:\Photos\IMG_0001.JPG"), expected);
        assert_eq!(windows("c:/photos/img_0001.jpg"), expected);
        assert_eq!(windows(r"\\?\C:\Photos\IMG_0001.JPG"), expected);
        assert_eq!(windows("//?/C:/Photos/IMG_0001.JPG"), expected);
        assert_eq!(windows(r"C:\Photos\.\Sub\..\IMG_0001.JPG"), expected);

        let unc = "//server/share/photos/img_0001.jpg";
        assert_eq!(windows(r"\\Server\Share\Photos\IMG_0001.JPG"), unc);
        assert_eq!(windows("//server/share/photos/img_0001.jpg"), unc);
        assert_eq!(windows(r"\\?\UNC\Server\Share\Photos\IMG_0001.JPG"), unc);
        assert_eq!(windows("//?/unc/Server/Share/Photos/IMG_0001.JPG"), unc);
        assert_eq!(windows(r"\\?\UNC/Server/Share/Photos/IMG_0001.JPG"), unc);
    }

    #[test]
    fn windows_drive_relative_paths_do_not_match_absolute_ones() {
        assert_eq!(windows(r"C:Photos\IMG.JPG"), "c:photos/img.jpg");
        assert_ne!(windows(r"C:Photos\IMG.JPG"), windows(r"C:\Photos\IMG.JPG"));
        assert_eq!(windows(r"unc\server\IMG.JPG"), "unc/server/img.jpg");
        assert_ne!(
            windows(r"unc\server\IMG.JPG"),
            windows(r"\\unc\server\IMG.JPG")
        );
    }

    #[test]
    fn linux_keeps_case_backslash_and_unicode_form_distinct() {
        assert_ne!(linux("/Photos/IMG.JPG"), linux("/photos/img.jpg"));
        assert_ne!(linux(r"a\b.jpg"), linux("a/b.jpg"));
        let nfc = "/caf\u{e9}/IMG.JPG";
        let nfd = "/cafe\u{301}/IMG.JPG";
        assert_ne!(linux(nfc), linux(nfd));
        assert_ne!(windows(nfc), windows(nfd));
    }

    #[test]
    fn macos_folds_case_and_unicode_form() {
        let nfc = "/Users/caf\u{e9}/IMG.JPG";
        let nfd = "/Users/cafe\u{301}/img.jpg";
        assert_eq!(mac(nfc), mac(nfd));
        assert_eq!(mac("/Users/Photo/IMG.JPG"), mac("/users/photo/img.jpg"));
        assert_ne!(mac(r"a\b.jpg"), mac("a/b.jpg"));
    }

    #[test]
    fn dot_segments_collapse_without_escaping_a_root() {
        assert_eq!(linux("/a/b/../c/./d.jpg"), "/a/c/d.jpg");
        assert_eq!(linux("/a//b.jpg"), "/a/b.jpg");
        assert_eq!(linux("a/../../b.jpg"), "../b.jpg");
        assert_eq!(linux("/../../b.jpg"), "/b.jpg");
        assert_eq!(linux("../.."), "../..");
        assert_eq!(linux("foo/../../bar/../baz.jpg"), "../baz.jpg");
        assert_eq!(windows(r"C:\Photos\..\IMG.JPG"), "c:/img.jpg");
    }

    #[test]
    fn non_ascii_names_are_distinct_and_the_cache_file_is_ascii() {
        let id = identity(5, 9);
        let left = relative_cache_path(&linux("/撮影A/写真.JPG"), id);
        let right = relative_cache_path(&linux("/撮影B/写真.JPG"), id);
        assert_ne!(left, right);
        let name = left.file_name().unwrap().to_str().unwrap();
        assert!(name.is_ascii());
        assert!(!name.contains('写'));
        assert_eq!(left, relative_cache_path(&linux("/撮影A/写真.JPG"), id));
    }

    #[test]
    fn hash_domain_includes_the_cache_version() {
        let domain = hash_domain();
        assert!(domain.contains(&format!("v{CACHE_VERSION}")));
        assert_eq!(path_key("x").len(), CACHE_KEY_HEX_LEN);
    }

    #[test]
    fn strip_cache_buster_removes_only_the_ui_suffix() {
        assert_eq!(
            strip_cache_buster("shoots%2Fa%2FIMG_0001.JPG?v=1700000000_2048"),
            "shoots%2Fa%2FIMG_0001.JPG"
        );
        assert_eq!(
            strip_cache_buster("%E5%86%99%E7%9C%9F.JPG?v=10_20"),
            "%E5%86%99%E7%9C%9F.JPG"
        );
        assert_eq!(
            strip_cache_buster("file%3Fv%3D1_2.jpg?v=3_4"),
            "file%3Fv%3D1_2.jpg"
        );
        assert_eq!(strip_cache_buster("plain.jpg"), "plain.jpg");
        assert_eq!(
            strip_cache_buster("plain.jpg?v=10_20x"),
            "plain.jpg?v=10_20x"
        );
        assert_eq!(strip_cache_buster("plain.jpg?v=10_"), "plain.jpg?v=10_");
        assert_eq!(strip_cache_buster("?v=0_0"), "");
    }

    #[test]
    fn disk_path_matches_the_stored_thumbnail_predicate() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("shoot").join("IMG_0001.JPG");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, b"not-an-image").unwrap();
        let id = identity(1_700_000_000, 42);
        let path = thumbnail_disk_path(dir.path(), &source, id);
        let path_str = path.to_string_lossy();
        assert!(stored_thumbnail_is_current(
            &path_str,
            id.mtime_secs as i64,
            id.size as i64
        ));
        assert!(!stored_thumbnail_is_current(
            &path_str,
            id.mtime_secs as i64 + 1,
            id.size as i64
        ));
        assert!(path.starts_with(dir.path().join(CACHE_DIR_NAME)));
    }

    #[test]
    fn canonical_paths_and_symlinks_share_a_key_distinct_files_do_not() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        let shoot_a = dir.path().join("A");
        let shoot_b = dir.path().join("B");
        fs::create_dir_all(&shoot_a).unwrap();
        fs::create_dir_all(&shoot_b).unwrap();
        let left = shoot_a.join("IMG_0001.JPG");
        let right = shoot_b.join("IMG_0001.JPG");
        fs::write(&left, b"red").unwrap();
        fs::write(&right, b"blue").unwrap();
        let id = identity(8, 3);
        assert_ne!(
            thumbnail_disk_path(&cache, &left, id),
            thumbnail_disk_path(&cache, &right, id)
        );

        let via_dots = dir
            .path()
            .join("A")
            .join("..")
            .join("A")
            .join("IMG_0001.JPG");
        assert_eq!(
            thumbnail_disk_path(&cache, &left, id),
            thumbnail_disk_path(&cache, &via_dots, id)
        );

        #[cfg(unix)]
        {
            let link_dir = dir.path().join("link-dir");
            fs::create_dir_all(&link_dir).unwrap();
            let link = link_dir.join("IMG_0001.JPG");
            std::os::unix::fs::symlink(&left, &link).unwrap();
            assert_eq!(
                thumbnail_disk_path(&cache, &left, id),
                thumbnail_disk_path(&cache, &link, id)
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn linux_case_variants_and_non_utf8_names_stay_distinct() {
        use std::os::unix::ffi::OsStringExt;

        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        let upper = dir.path().join("IMG.JPG");
        let lower = dir.path().join("img.jpg");
        fs::write(&upper, b"A").unwrap();
        fs::write(&lower, b"B").unwrap();
        let id = identity(1, 1);
        assert_ne!(
            thumbnail_disk_path(&cache, &upper, id),
            thumbnail_disk_path(&cache, &lower, id)
        );

        let raw = dir
            .path()
            .join(std::ffi::OsString::from_vec(b"img_\xff.jpg".to_vec()));
        fs::write(&raw, b"x").unwrap();
        let other = dir
            .path()
            .join(std::ffi::OsString::from_vec(b"img_\xfe.jpg".to_vec()));
        fs::write(&other, b"y").unwrap();
        let left = thumbnail_disk_path(&cache, &raw, id);
        let right = thumbnail_disk_path(&cache, &other, id);
        assert_ne!(left, right);
        assert_eq!(left, thumbnail_disk_path(&cache, &raw, id));
        assert!(left.file_name().unwrap().to_str().unwrap().is_ascii());
    }

    #[test]
    fn purge_removes_legacy_files_and_keeps_the_current_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join(".thumbnails");
        let current = cache.join(CACHE_DIR_NAME).join("ab");
        fs::create_dir_all(&current).unwrap();
        let live = current.join(format!("{}_1_2.webp", "ab".repeat(32)));
        fs::write(&live, b"live-thumb").unwrap();
        let legacy = cache.join("IMG_0001.webp");
        fs::write(&legacy, b"legacy").unwrap();
        let legacy_upper = cache.join("OTHER.WEBP");
        fs::write(&legacy_upper, b"legacy-upper").unwrap();
        fs::write(cache.join("notes.txt"), b"keep").unwrap();

        let retired = cache.join("v1").join("ab");
        fs::create_dir_all(&retired).unwrap();
        fs::write(retired.join("old.webp"), vec![7u8; 16]).unwrap();

        let future = cache.join("v3").join("ab");
        fs::create_dir_all(&future).unwrap();
        let future_file = future.join("later.webp");
        fs::write(&future_file, b"future").unwrap();

        let outside = dir.path().join("vacation.webp");
        fs::write(&outside, b"do-not-delete").unwrap();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, cache.join("linked.webp")).unwrap();
            let outside_dir = dir.path().join("album");
            fs::create_dir_all(&outside_dir).unwrap();
            fs::write(outside_dir.join("keep.jpg"), b"keep-album").unwrap();
            std::os::unix::fs::symlink(&outside_dir, cache.join("v0")).unwrap();
        }

        let stats = purge_obsolete_thumbnails(&cache).unwrap();
        assert!(!legacy.exists());
        assert!(!legacy_upper.exists());
        assert!(!cache.join("v1").exists());
        assert!(live.exists());
        assert!(cache.join("notes.txt").exists());
        assert!(future_file.exists());
        assert!(outside.exists());
        assert!(stats.removed_entries >= 3);
        assert!(stats.removed_bytes >= 6);
        assert_eq!(stats.errors, 0);

        #[cfg(unix)]
        {
            assert!(!cache.join("linked.webp").exists());
            assert!(outside.exists());
            assert!(!cache.join("v0").exists());
            assert!(dir.path().join("album").join("keep.jpg").exists());
        }

        let again = purge_obsolete_thumbnails(&cache).unwrap();
        assert_eq!(again.removed_entries, 0);
        assert!(live.exists());
    }

    #[test]
    fn purge_of_a_missing_directory_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let stats = purge_obsolete_thumbnails(&dir.path().join("missing")).unwrap();
        assert_eq!(stats, PurgeStats::default());
    }

    #[test]
    fn writing_a_new_identity_removes_only_that_paths_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        let source = dir.path().join("IMG_0001.JPG");
        fs::write(&source, b"v1").unwrap();
        let old_path = thumbnail_disk_path(&cache, &source, identity(1, 2));
        let new_path = thumbnail_disk_path(&cache, &source, identity(3, 4));
        fs::create_dir_all(old_path.parent().unwrap()).unwrap();
        fs::write(&old_path, b"old").unwrap();
        fs::write(&new_path, b"new").unwrap();

        let neighbor_source = dir.path().join("other.JPG");
        fs::write(&neighbor_source, b"other").unwrap();
        let neighbor = thumbnail_disk_path(&cache, &neighbor_source, identity(1, 2));
        if neighbor.parent() == new_path.parent() {
            fs::write(&neighbor, b"neighbor").unwrap();
        } else {
            fs::create_dir_all(neighbor.parent().unwrap()).unwrap();
            fs::write(&neighbor, b"neighbor").unwrap();
        }

        let removed = remove_other_identity_files(&new_path).unwrap();
        assert_eq!(removed, 1);
        assert!(!old_path.exists());
        assert!(new_path.exists());
        assert!(neighbor.exists());
    }
}

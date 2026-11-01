//! Shared rules for recognizing a thumbnail written by the current cache.
//!
//! The bytes of the key live in `infra`. This module only records the version
//! and the filename shape so a stored path from an older scheme (the file
//! stem alone) is never treated as fresh.

/// Bump when the thumbnail encoding or the cache key changes.
///
/// * v1 stored `{file_stem}.webp` directly in the cache root.
/// * v2 hashed the source path but stored sensor pixels, so portrait photos
///   whose cameras set EXIF Orientation stayed sideways.
/// * v3 applies that orientation before the thumbnail is resized.
pub const CACHE_VERSION: u32 = 3;

/// Directory name under the cache root for [`CACHE_VERSION`].
pub const CACHE_DIR_NAME: &str = "v3";

/// Hex length of a blake3 path key. The writer and this check both use it.
pub const CACHE_KEY_HEX_LEN: usize = 64;

/// `_{mtime}_{size}.webp` appended to the path key.
pub fn identity_suffix(mtime_secs: u64, size: u64) -> String {
    format!("_{mtime_secs}_{size}.webp")
}

/// True when `thumb_path` is a file this version would write for `mtime`/`size`.
///
/// Legacy `{stem}.webp` paths return false even if that file is still on disk,
/// because the stem is not unique across folders.
pub fn stored_thumbnail_is_current(thumb_path: &str, mtime: i64, size: i64) -> bool {
    if mtime < 0 || size < 0 {
        return false;
    }
    let expected = identity_suffix(mtime as u64, size as u64);
    let name = thumb_path.rsplit(['/', '\\']).next().unwrap_or("");
    let Some(hash) = name.strip_suffix(&expected) else {
        return false;
    };
    hash.len() == CACHE_KEY_HEX_LEN
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        && thumb_path
            .split(['/', '\\'])
            .any(|part| part == CACHE_DIR_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_path(mtime: i64, size: i64) -> String {
        let hash = "ab".repeat(32);
        format!("/data/.thumbnails/{CACHE_DIR_NAME}/ab/{hash}_{mtime}_{size}.webp")
    }

    #[test]
    fn cache_dir_name_matches_version() {
        assert_eq!(
            CACHE_VERSION, 3,
            "v3 is the first cache that applies EXIF orientation"
        );
        assert_eq!(CACHE_DIR_NAME, format!("v{CACHE_VERSION}"));
        assert_eq!("ab".repeat(32).len(), CACHE_KEY_HEX_LEN);
    }

    #[test]
    fn previous_orientation_cache_is_not_current() {
        let hash = "ab".repeat(32);
        let sideways = format!("/data/.thumbnails/v2/ab/{hash}_10_20.webp");
        assert!(
            !stored_thumbnail_is_current(&sideways, 10, 20),
            "v2 thumbnails ignored EXIF orientation and must be regenerated"
        );
        let current = format!("/data/.thumbnails/{CACHE_DIR_NAME}/ab/{hash}_10_20.webp");
        assert!(stored_thumbnail_is_current(&current, 10, 20));
    }

    #[test]
    fn legacy_stem_path_is_not_current() {
        assert!(!stored_thumbnail_is_current(
            "/data/.thumbnails/IMG_0001.webp",
            1_700_000_000,
            2048,
        ));
    }

    #[test]
    fn current_path_matches_only_its_own_identity() {
        let mtime = 1_700_000_000;
        let size = 2048;
        let path = current_path(mtime, size);
        assert!(stored_thumbnail_is_current(&path, mtime, size));
        assert!(!stored_thumbnail_is_current(&path, mtime + 1, size));
        assert!(!stored_thumbnail_is_current(&path, mtime, size + 1));
    }

    #[test]
    fn windows_separators_still_match() {
        let hash = "cd".repeat(32);
        let path =
            format!(r"C:\Users\me\AppData\.thumbnails\{CACHE_DIR_NAME}\cd\{hash}_10_20.webp");
        assert!(stored_thumbnail_is_current(&path, 10, 20));
    }

    #[test]
    fn version_must_be_its_own_path_component() {
        let hash = "ab".repeat(32);
        let embedded = format!("/data/.thumbnails/{hash}_10_20.webp");
        assert!(!stored_thumbnail_is_current(&embedded, 10, 20));
        let prefix = format!("/data/.thumbnails/{CACHE_DIR_NAME}extra/{hash}_10_20.webp");
        assert!(!stored_thumbnail_is_current(&prefix, 10, 20));
    }

    #[test]
    fn short_or_non_hex_names_are_rejected() {
        let path = format!("/cache/{CACHE_DIR_NAME}/ab/IMG_0001_10_20.webp");
        assert!(!stored_thumbnail_is_current(&path, 10, 20));
        let hash = "AB".repeat(32);
        let upper_ok = format!("/cache/{CACHE_DIR_NAME}/ab/{hash}_10_20.webp");
        assert!(stored_thumbnail_is_current(&upper_ok, 10, 20));
    }
}

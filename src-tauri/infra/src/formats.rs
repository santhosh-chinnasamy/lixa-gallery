//! Which image extensions are listed, and which of those the system webview
//! can paint from the original bytes.
//!
//! HEIC and HEIF are listed only on macOS. WKWebView decodes them. WebView2
//! and WebKitGTK do not, and this app does not link libheif, so it cannot
//! build a substitute preview. Listing those files on Windows or Linux would
//! put a photo in the grid that nothing can show.
//!
//! TIFF is listed on every platform. The `image` crate decodes it, so the
//! grid thumbnail is a WebP everywhere. WKWebView can also paint the original
//! TIFF. WebView2 and WebKitGTK cannot, so the viewer there asks the thumbnail
//! protocol for a decoded preview instead of the original file.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
}

pub fn current_platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Linux
    }
}

pub fn is_scanned_image_extension(ext: &str) -> bool {
    is_scanned_image_extension_on(ext, current_platform())
}

pub fn webview_renders_original(ext: &str) -> bool {
    webview_renders_original_on(ext, current_platform())
}

pub fn is_scanned_image_extension_on(ext: &str, platform: Platform) -> bool {
    let ext = ext.to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif" | "tif" | "tiff" | "avif" | "svg"
    ) || (platform == Platform::MacOs && matches!(ext.as_str(), "heic" | "heif"))
}

/// True when the webview can paint this file without a decoded substitute.
pub fn webview_renders_original_on(ext: &str, platform: Platform) -> bool {
    let ext = ext.to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif" | "avif" | "svg" => true,
        "tif" | "tiff" | "heic" | "heif" => platform == Platform::MacOs,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_platform_lists_only_formats_it_can_show() {
        let shared = [
            "jpg", "jpeg", "png", "webp", "bmp", "gif", "tif", "tiff", "avif", "svg",
        ];
        for platform in [Platform::Windows, Platform::Linux, Platform::MacOs] {
            for ext in shared {
                assert!(
                    is_scanned_image_extension_on(ext, platform),
                    "{ext} should be scanned on {platform:?}"
                );
            }
            assert!(!is_scanned_image_extension_on("txt", platform));
            assert!(!is_scanned_image_extension_on("cr2", platform));

            let heic = is_scanned_image_extension_on("HEIC", platform);
            let heif = is_scanned_image_extension_on("heif", platform);
            assert_eq!(heic, platform == Platform::MacOs);
            assert_eq!(heif, platform == Platform::MacOs);

            assert!(webview_renders_original_on("jpg", platform));
            assert!(webview_renders_original_on("PNG", platform));
            assert_eq!(
                webview_renders_original_on("tiff", platform),
                platform == Platform::MacOs,
                "only WKWebView paints TIFF bytes"
            );
            assert_eq!(
                webview_renders_original_on("TIF", platform),
                platform == Platform::MacOs
            );
            assert_eq!(
                webview_renders_original_on("heic", platform),
                platform == Platform::MacOs
            );
            assert!(!webview_renders_original_on("heif", Platform::Windows));
            assert!(!webview_renders_original_on("heif", Platform::Linux));
        }
    }

    #[test]
    fn this_binary_uses_the_host_platform() {
        let platform = current_platform();
        if cfg!(windows) {
            assert_eq!(platform, Platform::Windows);
        } else if cfg!(target_os = "macos") {
            assert_eq!(platform, Platform::MacOs);
        } else {
            assert_eq!(platform, Platform::Linux);
        }
        assert_eq!(
            is_scanned_image_extension("heic"),
            platform == Platform::MacOs
        );
        assert!(is_scanned_image_extension("tiff"));
        assert_eq!(
            webview_renders_original("tiff"),
            platform == Platform::MacOs
        );
    }
}

import { convertFileSrc } from '@tauri-apps/api/core';
import type { PhotoMetadata } from '../types/photo';
import { webviewProtocolUrl } from './protocolImage';
import { previewUrl, thumbnailUrl } from './thumbnail';

/** Asset URL safe for `<img>` under the Vite http origin. */
export function assetUrl(filePath: string): string {
  return webviewProtocolUrl(convertFileSrc(filePath));
}

export const UNSUPPORTED_FORMAT_MESSAGE =
  'Format not supported on this platform';

type PhotoRef = Pick<PhotoMetadata, 'path' | 'metadata'>;

/**
 * macOS WKWebView paints TIFF, HEIC, and HEIF. WebView2 and WebKitGTK do not.
 * The check mirrors `infra::formats::webview_renders_original_on`.
 */
export function hostPaintsTiffAndHeic(
  platform: string,
  userAgent: string,
): boolean {
  return /macintosh|mac os x|macintel|macarm/i.test(`${platform} ${userAgent}`);
}

export function currentHostPaintsTiffAndHeic(): boolean {
  if (typeof navigator === 'undefined') return false;
  return hostPaintsTiffAndHeic(
    navigator.platform ?? '',
    navigator.userAgent ?? '',
  );
}

function extensionOf(path: string): string {
  const base = path.split(/[/\\]/).pop() ?? '';
  const dot = base.lastIndexOf('.');
  if (dot <= 0 || dot === base.length - 1) return '';
  return base.slice(dot + 1).toLowerCase();
}

function isHeic(ext: string): boolean {
  return ext === 'heic' || ext === 'heif';
}

function isTiff(ext: string): boolean {
  return ext === 'tif' || ext === 'tiff';
}

/**
 * Grid image. HEIC on macOS is the original file, because WKWebView can paint
 * it and we have no decoder. On other platforms the URL is null so the card
 * shows a message instead of a broken image. TIFF always uses the decoded
 * WebP thumbnail.
 */
export function gridImageUrl(
  photo: PhotoRef,
  paintsTiffAndHeic = currentHostPaintsTiffAndHeic(),
): string | null {
  const ext = extensionOf(photo.path);
  if (isHeic(ext)) {
    return paintsTiffAndHeic ? assetUrl(photo.path) : null;
  }
  return thumbnailUrl(photo);
}

/**
 * Viewer and slideshow. TIFF is the original on macOS and a decoded WebP
 * preview elsewhere. HEIC follows the same host rule as the grid.
 */
export function viewerImageUrl(
  photo: PhotoRef,
  paintsTiffAndHeic = currentHostPaintsTiffAndHeic(),
): string | null {
  const ext = extensionOf(photo.path);
  if (isHeic(ext)) {
    return paintsTiffAndHeic ? assetUrl(photo.path) : null;
  }
  if (isTiff(ext) && !paintsTiffAndHeic) {
    return previewUrl(photo);
  }
  return assetUrl(photo.path);
}

/** Originals above this decode faster through the cached 2048px WebP preview. */
const LARGE_ORIGINAL_BYTES = 10 * 1024 * 1024;

/**
 * Should the viewer paint a cached decoded preview before the original?
 *
 * Only large originals the webview can paint natively benefit: RAW, TIFF and
 * HEIC already route through previews (or are unsupported) elsewhere.
 */
export function prefersDecodedPreview(
  photo: Pick<PhotoMetadata, 'path' | 'metadata' | 'is_raw' | 'preview_path'>,
): boolean {
  if (photo.is_raw || photo.preview_path) return false;
  if (photo.metadata.size < LARGE_ORIGINAL_BYTES) return false;
  const ext = extensionOf(photo.path);
  return !isHeic(ext) && !isTiff(ext);
}

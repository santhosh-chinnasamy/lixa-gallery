import type { PhotoMetadata } from '../types/photo';

/**
 * Thumbnail URL for the `lixa-thumbnail` protocol.
 *
 * The filesystem path is a query parameter. Putting an absolute path in the
 * URI path segment lets WebKit decode `%2F` and turn `/home/...` into
 * `home/...`, which then 404s. `path=` keeps the bytes intact.
 */
export function thumbnailUrl(
  photo: Pick<PhotoMetadata, 'path' | 'metadata'>,
): string {
  const path = encodeURIComponent(photo.path);
  const version = `${photo.metadata.modified}_${photo.metadata.size}`;
  return `lixa-thumbnail://localhost/thumb?path=${path}&v=${version}`;
}

/**
 * Decoded viewer preview for a format the webview cannot paint, such as TIFF
 * on Windows and Linux. The protocol serves a WebP instead of the original.
 */
export function previewUrl(
  photo: Pick<PhotoMetadata, 'path' | 'metadata'>,
): string {
  const path = encodeURIComponent(photo.path);
  const version = `${photo.metadata.modified}_${photo.metadata.size}`;
  return `lixa-thumbnail://localhost/thumb?path=${path}&v=${version}&kind=preview`;
}

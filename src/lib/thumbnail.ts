import type { PhotoMetadata } from '../types/photo';

/**
 * Thumbnail URL for the `lixa-thumbnail` protocol.
 * `v` changes when the file is replaced in place so the webview does not
 * keep showing the previous response.
 */
export function thumbnailUrl(
  photo: Pick<PhotoMetadata, 'path' | 'metadata'>,
): string {
  const encodedPath = encodeURIComponent(photo.path);
  const version = `${photo.metadata.modified}_${photo.metadata.size}`;
  return `lixa-thumbnail://localhost/${encodedPath}?v=${version}`;
}

/**
 * Decoded viewer preview for a format the webview cannot paint, such as TIFF
 * on Windows and Linux. The protocol serves a WebP instead of the original.
 */
export function previewUrl(
  photo: Pick<PhotoMetadata, 'path' | 'metadata'>,
): string {
  const encodedPath = encodeURIComponent(photo.path);
  const version = `${photo.metadata.modified}_${photo.metadata.size}`;
  return `lixa-thumbnail://localhost/${encodedPath}?v=${version}&kind=preview`;
}

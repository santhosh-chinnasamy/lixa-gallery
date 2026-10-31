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

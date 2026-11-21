import type { PhotoMetadata } from '../types/photo';
import { gridImageUrl } from './imageSource';
import { pageIsHttpOrigin } from './protocolImage';

/**
 * Grid thumb URL that stays same-origin under Vite.
 *
 * WebKitGTK 2.54 blocks `lixa-thumbnail://` / `asset://` from
 * `http://localhost:1420` (tauri#16201). Under an http origin, only emit
 * `/__lixa_thumb__` for an absolute on-disk `.webp` path — never a custom
 * protocol URL (img can sit forever without onload). Empty/missing thumbs
 * return null so ImageCard can ensure once and stay on Loading.
 *
 * Production / non-http keeps `gridImageUrl` (protocol) as today.
 */
export function gridThumbUrl(
  photo: Pick<PhotoMetadata, 'path' | 'metadata' | 'thumbnail_path'>,
): string | null {
  const thumb = photo.thumbnail_path?.trim() ?? '';
  const absoluteWebp =
    thumb.length > 0 &&
    /\.webp$/i.test(thumb) &&
    (thumb.startsWith('/') || /^[A-Za-z]:[\\/]/.test(thumb));

  if (pageIsHttpOrigin()) {
    if (!absoluteWebp) return null;
    return `/__lixa_thumb__?p=${encodeURIComponent(thumb)}`;
  }
  return gridImageUrl(photo);
}

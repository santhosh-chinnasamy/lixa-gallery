import { invoke } from '@tauri-apps/api/core';
import { assetUrl, viewerImageUrl } from './imageSource';
import type { PhotoMetadata } from '../types/photo';

/** A URL the webview can paint, or null when a RAW preview still has to be extracted. */
export function immediateViewerSrc(photo: PhotoMetadata): string | null {
  if (photo.preview_path) return assetUrl(photo.preview_path);
  if (photo.is_raw) return null;
  return viewerImageUrl(photo);
}

export async function resolveViewerSrc(
  photo: PhotoMetadata,
): Promise<string | null> {
  const ready = immediateViewerSrc(photo);
  if (ready) return ready;
  try {
    const preview = await invoke<string | null>('ensure_raw_preview', {
      path: photo.path,
    });
    return preview ? assetUrl(preview) : null;
  } catch {
    return null;
  }
}

export function formatLabel(photo: PhotoMetadata): string {
  const own = photo.metadata.name.split('.').pop() || 'Unknown';
  if (!photo.sidecar_path) return own;
  const side = photo.sidecar_path.split(/[/\\]/).pop()?.split('.').pop() || '';
  return side ? `${own} + ${side}` : own;
}

export { dateSortKey } from './photoOrder';

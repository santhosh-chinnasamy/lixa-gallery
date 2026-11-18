/**
 * WebKitGTK under Vite (`http://localhost:1420`) blocks custom-scheme and
 * often `http://{scheme}.localhost` `<img>` loads (tauri-apps/tauri#16201).
 * Serve image bytes through `invoke` → blob: URL instead.
 */

import { invoke } from '@tauri-apps/api/core';

const CUSTOM_LOCALHOST = /^(lixa-thumbnail|asset):\/\/localhost\//i;
const THUMB_HOST = /lixa-thumbnail/i;
const ASSET_HOST = /(^asset:)|(^https?:\/\/asset\.localhost)/i;

export function pageIsHttpOrigin(): boolean {
  if (typeof location === 'undefined') return false;
  return location.protocol === 'http:' || location.protocol === 'https:';
}

/**
 * Rewrite `scheme://localhost/…` → `http://scheme.localhost/…` under Vite
 * http origins. Production `tauri://` pages keep the custom-scheme form.
 */
export function webviewProtocolUrl(url: string): string {
  if (!pageIsHttpOrigin()) return url;
  return url.replace(
    CUSTOM_LOCALHOST,
    (_match, scheme: string) => `http://${scheme.toLowerCase()}.localhost/`,
  );
}

export type SafeImageSrc = {
  src: string;
  /** Revoke blob URLs when the consumer unmounts or replaces the src. */
  revoke: () => void;
};

function parseThumbnailRequest(
  url: string,
): { path: string; preview: boolean } | null {
  try {
    const parsed = new URL(url);
    if (!THUMB_HOST.test(parsed.protocol) && !THUMB_HOST.test(parsed.hostname)) {
      return null;
    }
    const path = parsed.searchParams.get('path');
    if (!path) return null;
    return {
      path,
      preview: parsed.searchParams.get('kind') === 'preview',
    };
  } catch {
    return null;
  }
}

/** Decode Tauri `asset` / `http://asset.localhost` file path. */
function parseAssetPath(url: string): string | null {
  if (!ASSET_HOST.test(url)) return null;
  try {
    const normalized = url.replace(/^asset:\/\/localhost\//i, 'http://asset.localhost/');
    const parsed = new URL(normalized);
    const raw = parsed.pathname.replace(/^\//, '');
    if (!raw) return null;
    return decodeURIComponent(raw);
  } catch {
    return null;
  }
}

function sniffMime(bytes: Uint8Array): string {
  if (
    bytes.length >= 12 &&
    bytes[0] === 0x52 &&
    bytes[1] === 0x49 &&
    bytes[2] === 0x46 &&
    bytes[3] === 0x46 &&
    bytes[8] === 0x57 &&
    bytes[9] === 0x45 &&
    bytes[10] === 0x42 &&
    bytes[11] === 0x50
  ) {
    return 'image/webp';
  }
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return 'image/jpeg';
  }
  if (
    bytes.length >= 8 &&
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47
  ) {
    return 'image/png';
  }
  return 'application/octet-stream';
}

function base64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) out[i] = bin.charCodeAt(i);
  return out;
}

function blobFromBase64(b64: string): SafeImageSrc {
  const bytes = base64ToBytes(b64);
  const blob = new Blob([bytes], { type: sniffMime(bytes) });
  const objectUrl = URL.createObjectURL(blob);
  return {
    src: objectUrl,
    revoke: () => URL.revokeObjectURL(objectUrl),
  };
}

/** In-flight + resolved blob cache so virtualized grids do not re-invoke. */
const imageBlobCache = new Map<string, Promise<SafeImageSrc>>();

async function cachedBlob(
  cacheKey: string,
  loader: () => Promise<string>,
): Promise<SafeImageSrc> {
  const cached = imageBlobCache.get(cacheKey);
  if (cached) {
    const hit = await cached;
    return { src: hit.src, revoke: () => {} };
  }

  const pending = (async (): Promise<SafeImageSrc> => {
    const b64 = await loader();
    const ready = blobFromBase64(b64);
    return {
      src: ready.src,
      revoke: () => {
        ready.revoke();
        imageBlobCache.delete(cacheKey);
      },
    };
  })();

  imageBlobCache.set(cacheKey, pending);
  try {
    const ready = await pending;
    return { src: ready.src, revoke: () => {} };
  } catch (error) {
    imageBlobCache.delete(cacheKey);
    throw error;
  }
}

export async function webviewSafeImageSrc(url: string): Promise<SafeImageSrc> {
  if (!pageIsHttpOrigin()) {
    return { src: url, revoke: () => {} };
  }

  const thumb = parseThumbnailRequest(url);
  if (thumb) {
    return cachedBlob(url, () =>
      invoke<string>('get_thumbnail_bytes', {
        path: thumb.path,
        preview: thumb.preview,
      }),
    );
  }

  const assetPath = parseAssetPath(url);
  if (assetPath) {
    return cachedBlob(url, () =>
      invoke<string>('read_image_bytes', { path: assetPath }),
    );
  }

  return { src: webviewProtocolUrl(url), revoke: () => {} };
}

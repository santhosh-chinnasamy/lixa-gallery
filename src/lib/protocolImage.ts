/**
 * WebKitGTK 2.54.0/2.54.1 blocks custom-scheme `<img src>` when the page is
 * served from `http://localhost` (Vite `tauri dev`). `fetch()` of the same URL
 * still works. See tauri-apps/tauri#16201.
 *
 * When the document origin is http(s), paint custom-protocol bytes through a
 * blob: URL instead. Production `tauri://` / custom-scheme pages are unchanged.
 */

const CUSTOM_SCHEME = /^(lixa-thumbnail|asset):/i;

export function needsProtocolBlobBridge(url: string): boolean {
  if (typeof location === 'undefined') return false;
  if (location.protocol !== 'http:' && location.protocol !== 'https:') {
    return false;
  }
  return CUSTOM_SCHEME.test(url);
}

export type SafeImageSrc = {
  src: string;
  /** Revoke blob URLs when the consumer unmounts or replaces the src. */
  revoke: () => void;
};

export async function webviewSafeImageSrc(url: string): Promise<SafeImageSrc> {
  if (!needsProtocolBlobBridge(url)) {
    return { src: url, revoke: () => {} };
  }

  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`protocol fetch ${response.status}`);
  }
  const blob = await response.blob();
  const objectUrl = URL.createObjectURL(blob);
  return {
    src: objectUrl,
    revoke: () => URL.revokeObjectURL(objectUrl),
  };
}

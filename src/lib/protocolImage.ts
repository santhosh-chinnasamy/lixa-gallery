/**
 * WebKitGTK 2.54.0/2.54.1 (and some 2.54.2 builds under Vite) refuse
 * custom-scheme `<img>` and often `fetch()` when the page is
 * `http://localhost:1420`. Same failure mode as tauri-apps/tauri#16201.
 *
 * Tauri already serves custom protocols at `http://{scheme}.localhost/…`.
 * Prefer that form under http(s) page origins so `<img>` loads without a
 * blob bridge. Keep fetch→blob as a last resort for stubborn schemes.
 */

const CUSTOM_SCHEME = /^(lixa-thumbnail|asset):\/\//i;
const CUSTOM_LOCALHOST =
  /^(lixa-thumbnail|asset):\/\/localhost\//i;

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

export function needsProtocolBlobBridge(url: string): boolean {
  if (!pageIsHttpOrigin()) return false;
  return CUSTOM_SCHEME.test(url);
}

export type SafeImageSrc = {
  src: string;
  /** Revoke blob URLs when the consumer unmounts or replaces the src. */
  revoke: () => void;
};

export async function webviewSafeImageSrc(url: string): Promise<SafeImageSrc> {
  const rewritten = webviewProtocolUrl(url);
  if (!needsProtocolBlobBridge(rewritten)) {
    return { src: rewritten, revoke: () => {} };
  }

  const response = await fetch(rewritten);
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

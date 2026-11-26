/**
 * WebKitGTK under Vite (`http://localhost:1420`) blocks custom-scheme and
 * often `http://{scheme}.localhost` `<img>` loads (tauri-apps/tauri#16201).
 * Serve image bytes through `invoke` → blob: URL instead.
 *
 * Blob lifetime is refcounted. Callers must call `revoke()`. Idle entries are
 * dropped under a byte budget (thumbs) or entry cap (full-res) so WebKit RSS
 * cannot climb for the whole session.
 */

import { invoke } from '@tauri-apps/api/core';

const CUSTOM_LOCALHOST = /^(lixa-thumbnail|asset):\/\/localhost\//i;
const THUMB_HOST = /lixa-thumbnail/i;
const ASSET_HOST = /(^asset:)|(^https?:\/\/asset\.localhost)/i;

/** Idle thumbs kept warm after release (compressed bytes). Keep tiny. */
const THUMB_BUDGET_BYTES = 16 * 1024 * 1024;
/** Full-res blobs retained with zero holders (viewer neighbour warm set). */
const FULL_IDLE_CAP = 2;
/** Max concurrent get_thumbnail_bytes / read_image_bytes invokes. */
const INVOKE_CONCURRENCY = 3;

type ImageKind = 'thumb' | 'full';

type CacheEntry = {
  kind: ImageKind;
  src: string;
  bytes: number;
  refs: number;
  /** Insertion / last-acquire order for idle eviction. */
  seq: number;
  revokeUrl: () => void;
};

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
  /** Drop this holder's ref. Frees the blob when refs hit 0 and budgets say so. */
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
    const normalized = url.replace(
      /^asset:\/\/localhost\//i,
      'http://asset.localhost/',
    );
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
  if (
    bytes.length >= 3 &&
    bytes[0] === 0xff &&
    bytes[1] === 0xd8 &&
    bytes[2] === 0xff
  ) {
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

let invokeInFlight = 0;
const invokeWaiters: Array<() => void> = [];

async function withInvokeSlot<T>(fn: () => Promise<T>): Promise<T> {
  if (invokeInFlight >= INVOKE_CONCURRENCY) {
    await new Promise<void>((resolve) => invokeWaiters.push(resolve));
  }
  invokeInFlight += 1;
  try {
    return await fn();
  } finally {
    invokeInFlight -= 1;
    const next = invokeWaiters.shift();
    if (next) next();
  }
}

const entries = new Map<string, CacheEntry>();
const inflight = new Map<string, Promise<CacheEntry>>();
let seqCounter = 0;

function idleThumbBytes(): number {
  let total = 0;
  for (const entry of entries.values()) {
    if (entry.kind === 'thumb' && entry.refs === 0) total += entry.bytes;
  }
  return total;
}

function idleFullCount(): number {
  let n = 0;
  for (const entry of entries.values()) {
    if (entry.kind === 'full' && entry.refs === 0) n += 1;
  }
  return n;
}

function destroyEntry(key: string, entry: CacheEntry): void {
  entry.revokeUrl();
  entries.delete(key);
}

/** Drop oldest zero-ref entries until budgets fit. */
function evictIdle(): void {
  const idle = [...entries.entries()]
    .filter(([, e]) => e.refs === 0)
    .sort((a, b) => a[1].seq - b[1].seq);

  for (const [key, entry] of idle) {
    const thumbOver = idleThumbBytes() > THUMB_BUDGET_BYTES;
    const fullOver = idleFullCount() > FULL_IDLE_CAP;
    if (!thumbOver && !fullOver) break;
    if (entry.kind === 'thumb' && thumbOver) {
      destroyEntry(key, entry);
      continue;
    }
    if (entry.kind === 'full' && fullOver) {
      destroyEntry(key, entry);
    }
  }
}

function hold(key: string, entry: CacheEntry): SafeImageSrc {
  entry.refs += 1;
  entry.seq = ++seqCounter;
  let released = false;
  return {
    src: entry.src,
    revoke: () => {
      if (released) return;
      released = true;
      entry.refs = Math.max(0, entry.refs - 1);
      if (entry.refs === 0) {
        // Free immediately. WebKit was retaining decoded frames for warm
        // idle thumbs and climbing past 10+ GiB with a blank Favourites UI.
        destroyEntry(key, entry);
        evictIdle();
      }
    },
  };
}

async function loadEntry(
  key: string,
  kind: ImageKind,
  loader: () => Promise<string>,
): Promise<CacheEntry> {
  const existing = entries.get(key);
  if (existing) return existing;

  const pending = inflight.get(key);
  if (pending) return pending;

  const work = (async (): Promise<CacheEntry> => {
    const b64 = await withInvokeSlot(loader);
    const bytes = base64ToBytes(b64);
    const blob = new Blob([bytes], { type: sniffMime(bytes) });
    const objectUrl = URL.createObjectURL(blob);
    const entry: CacheEntry = {
      kind,
      src: objectUrl,
      bytes: bytes.byteLength,
      refs: 0,
      seq: ++seqCounter,
      revokeUrl: () => URL.revokeObjectURL(objectUrl),
    };
    entries.set(key, entry);
    return entry;
  })();

  inflight.set(key, work);
  try {
    return await work;
  } finally {
    inflight.delete(key);
  }
}

async function acquireBlob(
  key: string,
  kind: ImageKind,
  loader: () => Promise<string>,
): Promise<SafeImageSrc> {
  const entry = await loadEntry(key, kind, loader);
  // Hold before eviction so a brand-new entry is never dropped at refs=0.
  const safe = hold(key, entry);
  evictIdle();
  return safe;
}

export async function webviewSafeImageSrc(url: string): Promise<SafeImageSrc> {
  if (!pageIsHttpOrigin()) {
    return { src: url, revoke: () => {} };
  }

  const thumb = parseThumbnailRequest(url);
  if (thumb) {
    return acquireBlob(url, 'thumb', () =>
      invoke<string>('get_thumbnail_bytes', {
        path: thumb.path,
        preview: thumb.preview,
      }),
    );
  }

  const assetPath = parseAssetPath(url);
  if (assetPath) {
    return acquireBlob(url, 'full', () =>
      invoke<string>('read_image_bytes', { path: assetPath }),
    );
  }

  return { src: webviewProtocolUrl(url), revoke: () => {} };
}

/** Test / probe helpers. Not for product UI. */
export function blobCacheStats(): {
  entries: number;
  thumbIdleBytes: number;
  fullIdle: number;
  invokeInFlight: number;
} {
  return {
    entries: entries.size,
    thumbIdleBytes: idleThumbBytes(),
    fullIdle: idleFullCount(),
    invokeInFlight,
  };
}

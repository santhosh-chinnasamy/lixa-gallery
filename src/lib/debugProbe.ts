/**
 * Temporary Favourites/nav freeze probe. Writes to the native log and console
 * so a stuck UI still leaves a trail in the tauri terminal.
 *
 * Enable with localStorage.lixa_debug = '1' (or always on in DEV).
 */
import { debug as logDebug, info as logInfo, warn as logWarn } from './logger';
import { blobCacheStats } from './protocolImage';

const STORAGE_KEY = 'lixa_debug';

export function debugEnabled(): boolean {
  if (typeof window === 'undefined') return false;
  if (import.meta.env.DEV) return true;
  try {
    return window.localStorage.getItem(STORAGE_KEY) === '1';
  } catch {
    return false;
  }
}

function stamp(): string {
  return new Date().toISOString().slice(11, 23);
}

export function probe(tag: string, detail?: Record<string, unknown>): void {
  if (!debugEnabled()) return;
  const payload = detail ? ` ${JSON.stringify(detail)}` : '';
  const line = `[probe ${stamp()}] ${tag}${payload}`;
  console.info(line);
  void logInfo(line).catch(() => {});
}

export function probeWarn(tag: string, detail?: Record<string, unknown>): void {
  if (!debugEnabled()) return;
  const payload = detail ? ` ${JSON.stringify(detail)}` : '';
  const line = `[probe ${stamp()}] ${tag}${payload}`;
  console.warn(line);
  void logWarn(line).catch(() => {});
}

export function probeDebug(tag: string, detail?: Record<string, unknown>): void {
  if (!debugEnabled()) return;
  const payload = detail ? ` ${JSON.stringify(detail)}` : '';
  const line = `[probe ${stamp()}] ${tag}${payload}`;
  console.debug(line);
  void logDebug(line).catch(() => {});
}

/** Heartbeat + long-task observer while a page is mounted. */
export function startProbeSession(name: string): () => void {
  if (!debugEnabled()) return () => {};

  probe('session.start', { name });
  const t0 = performance.now();
  let beats = 0;

  const heartbeat = window.setInterval(() => {
    beats += 1;
    const cache = blobCacheStats();
    probe('session.heartbeat', {
      name,
      beats,
      aliveMs: Math.round(performance.now() - t0),
      blobEntries: cache.entries,
      thumbIdleBytes: cache.thumbIdleBytes,
      fullIdle: cache.fullIdle,
      invokeInFlight: cache.invokeInFlight,
    });
  }, 2000);

  let observer: PerformanceObserver | null = null;
  try {
    observer = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        if (entry.duration < 50) continue;
        probeWarn('session.longtask', {
          name,
          durationMs: Math.round(entry.duration),
          startMs: Math.round(entry.startTime),
        });
      }
    });
    observer.observe({ entryTypes: ['longtask'] as string[] });
  } catch {
    // longtask observer not available in this webview
  }

  const onPointer = (event: Event) => {
    const target = event.target;
    const label =
      target instanceof HTMLElement
        ? `${target.tagName.toLowerCase()}${target.id ? `#${target.id}` : ''}${
            typeof target.className === 'string' && target.className
              ? `.${target.className.split(/\s+/).slice(0, 2).join('.')}`
              : ''
          }`
        : 'unknown';
    probe('session.pointer', {
      name,
      type: event.type,
      label: label.slice(0, 120),
    });
  };

  window.addEventListener('pointerdown', onPointer, true);
  window.addEventListener('click', onPointer, true);

  return () => {
    window.clearInterval(heartbeat);
    observer?.disconnect();
    window.removeEventListener('pointerdown', onPointer, true);
    window.removeEventListener('click', onPointer, true);
    probe('session.end', {
      name,
      beats,
      aliveMs: Math.round(performance.now() - t0),
    });
  };
}

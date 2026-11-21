import { invoke } from '@tauri-apps/api/core';

/** Cap concurrent ensure_thumbnail_path calls from the grid (avoid invoke storms). */
const MAX_CONCURRENT = 4;
let active = 0;
const waiters: Array<() => void> = [];

function acquire(): Promise<void> {
  if (active < MAX_CONCURRENT) {
    active++;
    return Promise.resolve();
  }
  return new Promise((resolve) => {
    waiters.push(() => {
      active++;
      resolve();
    });
  });
}

function release() {
  active--;
  waiters.shift()?.();
}

/** One-shot materialize; queued so the grid stays responsive under Vite. */
export function ensureThumbnailPath(path: string): Promise<string> {
  return acquire()
    .then(() => invoke<string>('ensure_thumbnail_path', { path }))
    .finally(release);
}

/**
 * Small insertion-ordered cache. `has` counts as a use, so a key that is
 * still needed stays ahead of keys that fall out of the window.
 */
export class LruCache<T> {
  private readonly entries = new Map<string, T>();
  private readonly limit: number;
  private readonly onEvict?: (value: T) => void;

  constructor(limit: number, onEvict?: (value: T) => void) {
    if (!Number.isInteger(limit) || limit < 1) {
      throw new Error('LruCache limit must be a positive integer');
    }
    this.limit = limit;
    this.onEvict = onEvict;
  }

  get size(): number {
    return this.entries.size;
  }

  has(key: string): boolean {
    if (!this.entries.has(key)) return false;
    const value = this.entries.get(key) as T;
    this.entries.delete(key);
    this.entries.set(key, value);
    return true;
  }

  set(key: string, value: T): void {
    const previous = this.entries.get(key);
    if (this.entries.has(key)) {
      this.entries.delete(key);
      if (previous !== undefined && previous !== value) {
        this.onEvict?.(previous);
      }
    }
    this.entries.set(key, value);
    while (this.entries.size > this.limit) {
      const oldest = this.entries.keys().next().value;
      if (oldest === undefined) break;
      const evicted = this.entries.get(oldest);
      this.entries.delete(oldest);
      if (evicted !== undefined) this.onEvict?.(evicted);
    }
  }

  clear(): void {
    if (this.onEvict) {
      for (const value of this.entries.values()) {
        this.onEvict(value);
      }
    }
    this.entries.clear();
  }
}

/** ±2 neighbours, with room to step back without reloading the original. */
export const VIEWER_PRELOAD_LIMIT = 8;

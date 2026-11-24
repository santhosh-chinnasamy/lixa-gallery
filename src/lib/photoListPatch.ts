export type PathIndex = Map<string, number>;

export function rebuildPathIndex(
  list: ReadonlyArray<{ path: string }>,
  index: PathIndex,
): void {
  index.clear();
  for (let i = 0; i < list.length; i++) {
    index.set(list[i].path, i);
  }
}

/**
 * Replace updated rows by path. The list array is not copied; only the
 * slots that changed get a new object. A stale index is rebuilt once.
 */
export function patchByPath<T extends { path: string }>(
  list: T[],
  index: PathIndex,
  updates: readonly T[],
  merge: (existing: T, update: T) => T,
): boolean {
  let changed = false;
  let rebuilt = false;
  for (const update of updates) {
    let at = index.get(update.path);
    if (at === undefined || list[at]?.path !== update.path) {
      if (!rebuilt) {
        rebuildPathIndex(list, index);
        rebuilt = true;
      }
      at = index.get(update.path);
    }
    if (at === undefined) continue;
    const existing = list[at];
    list[at] = merge(existing, update);
    changed = true;
  }
  return changed;
}

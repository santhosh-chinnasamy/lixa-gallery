import { invoke } from '@tauri-apps/api/core';
import { derived, get, writable } from 'svelte/store';
import { errorText, favouriteToggleError } from '$lib/errorText';
import { patchByPath, rebuildPathIndex } from '$lib/photoListPatch';
import { reportError } from '$lib/logger';
import {
  flagAfterAction,
  type CullAction,
  type CullFlag,
  type PhotoCullState,
} from '../lib/cull';
import type { FavouriteFolderGroup, PhotoMetadata } from '../types/photo';
import { toasts } from './toastStore';

export interface FolderNode {
  name: string;
  path: string;
  children: FolderNode[];
  /** Subfolders exist. `children` is empty until that level is loaded. */
  has_children: boolean;
}

function createIndexedPhotoList() {
  const store = writable<PhotoMetadata[]>([]);
  const index = new Map<string, number>();
  return {
    subscribe: store.subscribe,
    set(list: PhotoMetadata[]) {
      rebuildPathIndex(list, index);
      store.set(list);
    },
    update(fn: (list: PhotoMetadata[]) => PhotoMetadata[]) {
      store.update((current) => {
        const next = fn(current);
        if (next !== current) rebuildPathIndex(next, index);
        return next;
      });
    },
    /** Patch by path. Does not copy the list; only changed slots are replaced. */
    patch(updates: PhotoMetadata[]): boolean {
      const list = get(store);
      const changed = patchByPath(list, index, updates, (existing, update) => ({
        ...existing,
        ...update,
      }));
      if (changed) store.set(list);
      return changed;
    },
  };
}

type GroupSlot = { group: number; photo: number };

function createIndexedGroupList() {
  const store = writable<FavouriteFolderGroup[]>([]);
  const index = new Map<string, GroupSlot>();

  function reindex(groups: FavouriteFolderGroup[]) {
    index.clear();
    for (let groupIndex = 0; groupIndex < groups.length; groupIndex++) {
      const groupPhotos = groups[groupIndex].photos;
      for (let photoIndex = 0; photoIndex < groupPhotos.length; photoIndex++) {
        index.set(groupPhotos[photoIndex].path, {
          group: groupIndex,
          photo: photoIndex,
        });
      }
    }
  }

  return {
    subscribe: store.subscribe,
    set(groups: FavouriteFolderGroup[]) {
      reindex(groups);
      store.set(groups);
    },
    update(fn: (groups: FavouriteFolderGroup[]) => FavouriteFolderGroup[]) {
      store.update((current) => {
        const next = fn(current);
        if (next !== current) reindex(next);
        return next;
      });
    },
    patch(updates: PhotoMetadata[]): boolean {
      const groups = get(store);
      let changed = false;
      let rebuilt = false;
      for (const update of updates) {
        let slot = index.get(update.path);
        const current = slot
          ? groups[slot.group]?.photos[slot.photo]
          : undefined;
        if (!current || current.path !== update.path) {
          if (!rebuilt) {
            reindex(groups);
            rebuilt = true;
          }
          slot = index.get(update.path);
        }
        if (!slot) continue;
        const existing = groups[slot.group].photos[slot.photo];
        groups[slot.group].photos[slot.photo] = { ...existing, ...update };
        changed = true;
      }
      if (changed) store.set(groups);
      return changed;
    },
  };
}

export const photos = createIndexedPhotoList();
export const isLoading = writable(false);
export const currentFolder = writable<string | null>(null);
/** Folder session from the last successful `scan_folder`. */
export const folderGeneration = writable(0);
/**
 * Bumps when thumbnail patches mutate `photos` in place. The array identity
 * stays put, so the grid reads this to skip a full re-sort.
 */
export const photosRevision = writable(0);
export const folderTree = writable<FolderNode | null>(null);

export type SortOption = 'name' | 'date' | 'size';
export type SortOrder = 'asc' | 'desc';
export type LoadingMode = 'sync' | 'lazy';

export const searchQuery = writable<string>('');
export const sortBy = writable<SortOption>('name');
export const sortOrder = writable<SortOrder>('asc');

export type CullFilter = 'all' | 'picked' | 'rejected' | 'unflagged';

function createCullFilterStore() {
  const STORAGE_KEY = 'cull_filter';
  const stored =
    typeof localStorage !== 'undefined'
      ? localStorage.getItem(STORAGE_KEY)
      : null;
  const initial: CullFilter =
    stored === 'picked' ||
    stored === 'rejected' ||
    stored === 'unflagged' ||
    stored === 'all'
      ? stored
      : 'all';
  const { subscribe, set } = writable<CullFilter>(initial);
  return {
    subscribe,
    set: (filter: CullFilter) => {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, filter);
      }
      set(filter);
    },
  };
}

export const cullFilter = createCullFilterStore();

function createAutoAdvanceStore() {
  const STORAGE_KEY = 'cull_auto_advance';
  const initial =
    typeof localStorage === 'undefined'
      ? true
      : localStorage.getItem(STORAGE_KEY) !== 'off';
  const { subscribe, set } = writable<boolean>(initial);
  return {
    subscribe,
    set: (enabled: boolean) => {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, enabled ? 'on' : 'off');
      }
      set(enabled);
    },
  };
}

export const autoAdvance = createAutoAdvanceStore();

type StoredFlag = Exclude<CullFlag, 'unflagged'>;

function createCullFlagStore() {
  const flags = writable<Map<string, StoredFlag>>(new Map());
  const pending = new Set<string>();

  function current(): Map<string, StoredFlag> {
    return get(flags);
  }

  function flagOf(path: string): CullFlag {
    return current().get(path) ?? 'unflagged';
  }

  function writeLocal(path: string, flag: CullFlag) {
    flags.update((map) => {
      const next = new Map(map);
      if (flag === 'unflagged') next.delete(path);
      else next.set(path, flag);
      return next;
    });
  }

  return {
    subscribe: flags.subscribe,
    flagOf,
    apply: async (path: string, action: CullAction) => {
      if (pending.has(path)) return;
      pending.add(path);
      const previous = flagOf(path);
      const next = flagAfterAction(previous, action);
      writeLocal(path, next);
      try {
        await invoke('set_cull_flag', { path, flag: next });
      } catch (error) {
        writeLocal(path, previous);
        const message = favouriteToggleError(error);
        reportError(message);
        toasts.show(message);
      } finally {
        pending.delete(path);
      }
    },
    initialize: async () => {
      try {
        const rows = await invoke<PhotoCullState[]>('get_cull_flags');
        const next = new Map<string, StoredFlag>();
        for (const row of rows) {
          if (row.flag === 'picked' || row.flag === 'rejected') {
            next.set(row.path, row.flag);
          }
        }
        flags.set(next);
      } catch (error) {
        const message = `Couldn't load favourites. ${errorText(error)}`;
        reportError(message);
        toasts.show(message);
      }
    },
    dropPicked: () => {
      flags.update((map) => {
        const next = new Map(map);
        for (const [path, flag] of next) {
          if (flag === 'picked') next.delete(path);
        }
        return next;
      });
    },
    dropPickedPrefix: (prefix: string) => {
      flags.update((map) => {
        const next = new Map(map);
        for (const [path, flag] of next) {
          if (flag === 'picked' && path.startsWith(prefix)) next.delete(path);
        }
        return next;
      });
    },
  };
}

export const cullFlags = createCullFlagStore();

function createLoadingModeStore() {
  const STORAGE_KEY = 'loading_mode';
  const initialValue = (
    typeof localStorage !== 'undefined'
      ? localStorage.getItem(STORAGE_KEY) || 'lazy'
      : 'lazy'
  ) as LoadingMode;
  const { subscribe, set } = writable<LoadingMode>(initialValue);

  return {
    subscribe,
    set: (mode: LoadingMode) => {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, mode);
      }
      set(mode);
    },
  };
}

export const loadingMode = createLoadingModeStore();

function createGridSizeStore() {
  const STORAGE_KEY = 'grid_size';
  const initialValue = Number(
    typeof localStorage !== 'undefined'
      ? localStorage.getItem(STORAGE_KEY) || '180'
      : '180',
  );
  const { subscribe, set } = writable<number>(initialValue);

  return {
    subscribe,
    set: (size: number) => {
      const clampedSize = Math.max(100, Math.min(400, size));
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, clampedSize.toString());
      }
      set(clampedSize);
    },
  };
}

export const gridSize = createGridSizeStore();

export type GridFit = 'fill' | 'fit';

function createGridFitStore() {
  const STORAGE_KEY = 'grid_fit';
  const stored =
    typeof localStorage !== 'undefined'
      ? localStorage.getItem(STORAGE_KEY)
      : null;
  const initialValue: GridFit = stored === 'fit' ? 'fit' : 'fill';
  const { subscribe, set } = writable<GridFit>(initialValue);

  return {
    subscribe,
    set: (fit: GridFit) => {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, fit);
      }
      set(fit);
    },
  };
}

export const gridFit = createGridFitStore();

function createExpandedFoldersStore() {
  const { subscribe, update, set } = writable<Set<string>>(new Set());
  return {
    subscribe,
    toggle: (path: string) =>
      update((s) => {
        const newSet = new Set(s);
        if (newSet.has(path)) newSet.delete(path);
        else newSet.add(path);
        return newSet;
      }),
    expand: (path: string) =>
      update((s) => {
        const newSet = new Set(s);
        newSet.add(path);
        return newSet;
      }),
    clear: () => set(new Set()),
  };
}

export const expandedFolders = createExpandedFoldersStore();

function createExplorerWidthStore() {
  const STORAGE_KEY = 'explorer_width';
  const initialValue = Number(
    typeof localStorage !== 'undefined'
      ? localStorage.getItem(STORAGE_KEY) || '250'
      : '250',
  );
  const { subscribe, set } = writable<number>(initialValue);

  return {
    subscribe,
    set: (width: number) => {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, width.toString());
      }
      set(width);
    },
  };
}

export const explorerWidth = createExplorerWidthStore();

function createRecentFoldersStore() {
  const STORAGE_KEY = 'recent_folders';
  const initialValue = JSON.parse(
    typeof localStorage !== 'undefined'
      ? localStorage.getItem(STORAGE_KEY) || '[]'
      : '[]',
  );
  const { subscribe, set, update } = writable<string[]>(initialValue);

  return {
    subscribe,
    add: (path: string) =>
      update((folders) => {
        const newFolders = [path, ...folders.filter((f) => f !== path)].slice(
          0,
          5,
        );
        if (typeof localStorage !== 'undefined') {
          localStorage.setItem(STORAGE_KEY, JSON.stringify(newFolders));
        }
        return newFolders;
      }),
    clear: () => {
      if (typeof localStorage !== 'undefined') {
        localStorage.removeItem(STORAGE_KEY);
      }
      set([]);
    },
  };
}

export const recentFolders = createRecentFoldersStore();

const pickedPaths = derived(cullFlags, ($flags) => {
  const picked = new Set<string>();
  for (const [path, flag] of $flags) {
    if (flag === 'picked') picked.add(path);
  }
  return picked;
});

export const favorites = {
  subscribe: pickedPaths.subscribe,
  initialize: () => cullFlags.initialize(),
  has: (path: string) => get(pickedPaths).has(path),
  toggle: (path: string) => cullFlags.apply(path, 'pick'),
  clear: async () => {
    try {
      await invoke('clear_favourites');
      cullFlags.dropPicked();
      favouritePhotos.set([]);
      favouriteFolderGroups.set([]);
    } catch (error) {
      const message = `Couldn't clear favourites. ${errorText(error)}`;
      reportError(message);
      toasts.show(message);
    }
  },
  clearPrefix: async (prefix: string) => {
    try {
      await invoke('clear_favourites_by_prefix', { prefix });
      cullFlags.dropPickedPrefix(prefix);
      await refreshFavourites();
    } catch (error) {
      const message = `Couldn't clear favourites. ${errorText(error)}`;
      reportError(message);
      toasts.show(message);
    }
  },
};

export type FavouriteScope = 'current' | 'all';
export const favouriteScope = writable<FavouriteScope>('current');
export const favouritePhotos = createIndexedPhotoList();
export const favouriteFolderGroups = createIndexedGroupList();
export const isLoadingFavourites = writable<boolean>(false);

export const currentFolderFavoritesCount = derived(
  [favorites, currentFolder],
  ([$favorites, $currentFolder]) => {
    if (!$currentFolder) return 0;
    let count = 0;
    for (const path of $favorites) {
      if (path.startsWith($currentFolder)) {
        count++;
      }
    }
    return count;
  },
);

export async function refreshFavourites() {
  isLoadingFavourites.set(true);
  try {
    await favorites.initialize();
    const groups = await invoke<FavouriteFolderGroup[]>(
      'get_favourite_folder_groups',
    );
    favouriteFolderGroups.set(groups);

    const curr = get(currentFolder);
    const scope = get(favouriteScope);
    const folderScope = scope === 'current' && curr ? curr : null;
    const photosList = await invoke<PhotoMetadata[]>('get_favourite_photos', {
      folderScope,
    });
    favouritePhotos.set(photosList);
  } catch (error) {
    const message = `Couldn't refresh favourites. ${errorText(error)}`;
    reportError(message);
    toasts.show(message);
  } finally {
    isLoadingFavourites.set(false);
  }
}

export function clearPhotos() {
  photos.set([]);
  currentFolder.set(null);
  folderGeneration.set(0);
  folderTree.set(null);
  expandedFolders.clear();
}

/**
 * Bump the materialization revision at most once per frame.
 *
 * The backend emits a `photos-updated` batch every ~10–64 rows, so a cold
 * 2,000-photo folder produced hundreds of grid re-derives per second. The
 * patches are already in the array by the next frame; one revision bump per
 * frame is enough to make the grid read them.
 */
let revisionFrame = 0;

function schedulePhotosRevision() {
  if (revisionFrame) return;
  const schedule = (callback: () => void): number => {
    if (typeof requestAnimationFrame === 'function') {
      return requestAnimationFrame(callback);
    }
    return setTimeout(callback, 16) as unknown as number;
  };
  revisionFrame = schedule(() => {
    revisionFrame = 0;
    photosRevision.update((revision) => revision + 1);
  });
}

/** True while the Favourites page owns the lists that `mergePhotoUpdates` patches. */
function favouritesPageIsOpen(): boolean {
  if (typeof location === 'undefined') return false;
  return location.pathname.replace(/\/+$/, '') === '/favourites';
}

/**
 * Merge backend materialize/EXIF results into the open folder list.
 * Drops patches from a different folder session; patches by path only.
 * Preserves list order so thumb-only patches do not reshuffle the grid.
 */
export function mergePhotoUpdates(
  generation: number,
  updates: PhotoMetadata[],
) {
  if (!Array.isArray(updates) || updates.length === 0) return;
  const liveGeneration = get(folderGeneration);
  // Drop patches from a different folder session. generation 0 means "use live".
  if (generation !== 0 && generation !== liveGeneration) return;

  if (photos.patch(updates)) schedulePhotosRevision();

  // Favourites rebuilds both lists from `refreshFavourites()` when the page
  // mounts, so patching them on every library batch is wasted work for a grid
  // nobody is looking at. Patch them only while that page is open.
  if (favouritesPageIsOpen()) {
    favouritePhotos.patch(updates);
    // All Folders favourites render from group.photos; patch those too or
    // thumbs stay empty after background materialize.
    favouriteFolderGroups.patch(updates);
  }
}

export async function loadFolderTree(path: string) {
  try {
    const tree = await invoke<FolderNode>('get_folder_tree', { path });
    folderTree.set(tree);
  } catch (error) {
    const message = `Couldn't list subfolders in ${path}. ${errorText(error)}`;
    reportError(message);
    toasts.show(message);
  }
}

function findFolder(node: FolderNode, path: string): FolderNode | null {
  if (node.path === path) return node;
  for (const child of node.children) {
    const found = findFolder(child, path);
    if (found) return found;
  }
  return null;
}

function attachFolderChildren(
  node: FolderNode,
  path: string,
  children: FolderNode[],
): FolderNode {
  if (node.path === path) {
    return { ...node, children, has_children: children.length > 0 };
  }
  let changed = false;
  const nextChildren = node.children.map((child) => {
    const next = attachFolderChildren(child, path, children);
    if (next !== child) changed = true;
    return next;
  });
  if (!changed) return node;
  return { ...node, children: nextChildren };
}

/** Loads the next level of the sidebar tree the first time a node is expanded. */
export async function ensureFolderChildren(path: string) {
  const tree = get(folderTree);
  const node = tree ? findFolder(tree, path) : null;
  if (!node || !node.has_children || node.children.length > 0) return;

  try {
    const loaded = await invoke<FolderNode>('get_folder_tree', { path });
    folderTree.update((current) =>
      current ? attachFolderChildren(current, path, loaded.children) : current,
    );
  } catch (error) {
    console.error('Error loading folder children:', error);
  }
}

// Initialize favorites when the store is created
favorites.initialize();

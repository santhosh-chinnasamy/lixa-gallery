import { invoke } from '@tauri-apps/api/core';
import { derived, get, writable } from 'svelte/store';
import {
  flagAfterAction,
  type CullAction,
  type CullFlag,
  type PhotoCullState,
} from '../lib/cull';
import type { FavouriteFolderGroup, PhotoMetadata } from '../types/photo';

export interface FolderNode {
  name: string;
  path: string;
  children: FolderNode[];
}

export const photos = writable<PhotoMetadata[]>([]);
export const isLoading = writable(false);
export const currentFolder = writable<string | null>(null);
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

  function current(): Map<string, StoredFlag> {
    return get(flags);
  }

  function flagOf(path: string): CullFlag {
    return current().get(path) ?? 'unflagged';
  }

  async function persist(path: string, flag: CullFlag) {
    await invoke('set_cull_flag', { path, flag });
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
    persist,
    apply: async (path: string, action: CullAction) => {
      await persist(path, flagAfterAction(flagOf(path), action));
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
        console.error('Error loading cull flags:', error);
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
    await invoke('clear_favourites');
    cullFlags.dropPicked();
    favouritePhotos.set([]);
    favouriteFolderGroups.set([]);
  },
  clearPrefix: async (prefix: string) => {
    await invoke('clear_favourites_by_prefix', { prefix });
    cullFlags.dropPickedPrefix(prefix);
    await refreshFavourites();
  },
};

export type FavouriteScope = 'current' | 'all';
export const favouriteScope = writable<FavouriteScope>('current');
export const favouritePhotos = writable<PhotoMetadata[]>([]);
export const favouriteFolderGroups = writable<FavouriteFolderGroup[]>([]);
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
    console.error('Error refreshing favourites:', error);
  } finally {
    isLoadingFavourites.set(false);
  }
}

export function clearPhotos() {
  photos.set([]);
  currentFolder.set(null);
  folderTree.set(null);
  expandedFolders.clear();
}

export async function loadFolderTree(path: string) {
  try {
    const tree = await invoke<FolderNode>('get_folder_tree', { path });
    folderTree.set(tree);
  } catch (error) {
    console.error('Error loading folder tree:', error);
  }
}

// Initialize favorites when the store is created
favorites.initialize();

import { invoke } from '@tauri-apps/api/core';
import { derived, get, writable } from 'svelte/store';
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

type FavoriteStore = Set<string>;

function createFavoritesStore() {
  const { subscribe, set, update } = writable<FavoriteStore>(new Set());

  return {
    subscribe,
    initialize: async () => {
      try {
        const favs = await invoke<Array<{ path: string }>>('get_favourites');
        const paths = new Set(favs.map((f) => f.path));
        set(paths);
      } catch (error) {
        console.error('Error loading favorites:', error);
      }
    },
    add: (path: string) =>
      update((favorites) => {
        favorites.add(path);
        return favorites;
      }),
    remove: (path: string) =>
      update((favorites) => {
        favorites.delete(path);
        return favorites;
      }),
    has: (path: string) => {
      const favoriteSet = get({ subscribe });
      return favoriteSet.has(path);
    },
    toggle: async (path: string) => {
      const favoriteSet = get({ subscribe });
      if (favoriteSet.has(path)) {
        await invoke('remove_favourite', { path });
        favorites.remove(path);
      } else {
        await invoke('add_favourite', { path });
        favorites.add(path);
      }
    },
    clear: async () => {
      await invoke('clear_favourites');
      set(new Set());
      favouritePhotos.set([]);
      favouriteFolderGroups.set([]);
    },
    clearPrefix: async (prefix: string) => {
      await invoke('clear_favourites_by_prefix', { prefix });
      await favorites.initialize();
      await refreshFavourites();
    },
  };
}

export const favorites = createFavoritesStore();

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

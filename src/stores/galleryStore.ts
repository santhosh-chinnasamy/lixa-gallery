import { invoke } from '@tauri-apps/api/core';
import { get, writable } from 'svelte/store';
import type { PhotoMetadata } from '../types/photo';

export const photos = writable<PhotoMetadata[]>([]);
export const isLoading = writable(false);
export const currentFolder = writable<string | null>(null);

function createRecentFoldersStore() {
  const STORAGE_KEY = 'recent_folders';
  const initialValue = JSON.parse(
    typeof localStorage !== 'undefined'
      ? localStorage.getItem(STORAGE_KEY) || '[]'
      : '[]'
  );
  const { subscribe, set, update } = writable<string[]>(initialValue);

  return {
    subscribe,
    add: (path: string) =>
      update((folders) => {
        const newFolders = [path, ...folders.filter((f) => f !== path)].slice(
          0,
          5
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
        const favorites =
          await invoke<Array<{ path: string }>>('get_favourites');
        const paths = new Set(favorites.map((f) => f.path));
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
    },
  };
}

export const favorites = createFavoritesStore();

// Initialize favorites when the store is created
favorites.initialize();

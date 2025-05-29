import { invoke } from '@tauri-apps/api/core';
import { get, writable } from 'svelte/store';

export const photos = writable<string[]>([]);
export const isLoading = writable(false);

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
  };
}

export const favorites = createFavoritesStore();

// Initialize favorites when the store is created
favorites.initialize();

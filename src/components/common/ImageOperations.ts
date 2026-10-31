import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { error as logError } from '$lib/logger';
import {
  photos,
  isLoading,
  favorites,
  currentFolder,
  recentFolders,
  loadFolderTree,
  loadingMode,
} from '../../stores/galleryStore';
import { get } from 'svelte/store';
import type { PhotoMetadata } from '../../types/photo';

export async function loadPath(folderPath: string, isRoot = true) {
  try {
    isLoading.set(true);
    const mode = get(loadingMode);
    const loadedPhotos: PhotoMetadata[] = await invoke('scan_folder', {
      path: folderPath,
      mode: mode,
    });
    photos.set(loadedPhotos);
    currentFolder.set(folderPath);
    if (isRoot) {
      recentFolders.add(folderPath);
      await loadFolderTree(folderPath);
      invoke('save_workspace', { path: folderPath }).catch((e) =>
        logError(`Failed to record workspace: ${e}`),
      );
    }
  } catch (error) {
    logError(`Failed to load photos from path: ${error}`);
  } finally {
    isLoading.set(false);
  }
}

export async function loadSubfolder(path: string) {
  await loadPath(path, false);
}

export async function loadPhotos() {
  try {
    const folder = await open({
      multiple: false,
      directory: true,
    });

    if (!folder) {
      return;
    }

    isLoading.set(true);
    await loadPath(folder);
  } catch (error) {
    logError(`Failed to load photos: ${error}`);
    isLoading.set(false);
  }
}

export async function exportFavorites(
  mode: 'copy' | 'move' = 'copy',
  paths?: string[],
  preserveFolderStructure = false,
) {
  try {
    const destination = await open({
      multiple: false,
      directory: true,
    });
    if (!destination) return;

    await invoke('export_favourites', {
      destination,
      mode,
      paths: paths && paths.length > 0 ? paths : null,
      preserveFolderStructure,
    });

    return destination;
  } catch (error) {
    logError(`Failed to export files: ${error}`);
    throw error;
  }
}

export async function clearFavorites() {
  const confirmed = await confirm(
    'Are you sure you want to clear all favourites? This action cannot be undone.',
  );

  if (confirmed) {
    await favorites.clear();
  }
}

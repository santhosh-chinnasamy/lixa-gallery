import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { photos, isLoading, favorites, currentFolder, recentFolders } from '../../stores/galleryStore';
import type { PhotoMetadata } from '../../types/photo';

export async function loadPath(folderPath: string) {
  try {
    isLoading.set(true);
    const loadedPhotos: PhotoMetadata[] = await invoke('scan_folder', {
      path: folderPath,
    });
    photos.set(loadedPhotos);
    currentFolder.set(folderPath);
    recentFolders.add(folderPath);
  } catch (error) {
    console.error('Failed to load photos from path:', error);
  } finally {
    isLoading.set(false);
  }
}

export async function loadPhotos() {
  try {
    isLoading.set(true);
    const folder = await open({
      multiple: false,
      directory: true,
    });

    if (!folder) {
      isLoading.set(false);
      return;
    }

    await loadPath(folder);
  } catch (error) {
    console.error('Failed to load photos:', error);
    isLoading.set(false);
  }
}

export async function loadPhotosWithModal(onModalOpen: () => void, onModalClose: () => void) {
  try {
    onModalOpen();
    isLoading.set(true);

    const folder = await open({
      multiple: false,
      directory: true,
    });

    if (!folder) {
      isLoading.set(false);
      onModalClose();
      return;
    }

    await loadPath(folder);
    onModalClose();
  } catch (error) {
    console.error('Failed to load photos:', error);
    onModalClose();
    isLoading.set(false);
  }
}

export async function exportFavorites() {
  try {
    const destination = await open({
      multiple: false,
      directory: true,
    });
    if (!destination) return;

    await invoke('export_favourites', {
      destination,
    });

    return destination;
  } catch (error) {
    console.error('Failed to export files:', error);
    throw error;
  }
}

export async function clearFavorites() {
  const confirmed = await confirm(
    'Are you sure you want to clear all favourites? This action cannot be undone.'
  );

  if (confirmed) {
    await favorites.clear();
  }
}
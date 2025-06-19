import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { photos, isLoading, favorites } from '../../stores/galleryStore';

export async function loadPhotos() {
  try {
    isLoading.set(true);
    const folder = await open({
      multiple: false,
      directory: true,
    });
    
    if (!folder) {
      isLoading.set(false);
      photos.set([]);
      return;
    }
    
    const loadedPhotos: string[] = await invoke('scan_folder', {
      path: folder,
    });
    photos.set(loadedPhotos);
  } catch (error) {
    console.error('Failed to load photos:', error);
  } finally {
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
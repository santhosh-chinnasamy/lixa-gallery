import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { errorText, folderOpenError } from '$lib/errorText';
import { reportError } from '$lib/logger';
import {
  photos,
  isLoading,
  favorites,
  currentFolder,
  recentFolders,
  loadFolderTree,
  loadingMode,
} from '../../stores/galleryStore';
import { toasts } from '../../stores/toastStore';
import { get } from 'svelte/store';
import type { ExportReport, PhotoMetadata } from '../../types/photo';

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
    toasts.dismissAll();
    if (isRoot) {
      recentFolders.add(folderPath);
      await loadFolderTree(folderPath);
      invoke('save_workspace', { path: folderPath }).catch((e) =>
        reportError(
          `Failed to record workspace for ${folderPath}. ${errorText(e)}`,
        ),
      );
    }
  } catch (error) {
    const message = folderOpenError(folderPath, error);
    reportError(message);
    toasts.show(message, {
      label: 'Retry',
      run: () => {
        void loadPath(folderPath, isRoot);
      },
    });
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

    await loadPath(folder);
  } catch (error) {
    const message = `Couldn't open a folder. ${errorText(error)}`;
    reportError(message);
    toasts.show(message);
    isLoading.set(false);
  }
}

export async function exportFavorites(
  mode: 'copy' | 'move' = 'copy',
  paths?: string[],
  preserveFolderStructure = false,
): Promise<{ destination: string; report: ExportReport } | undefined> {
  try {
    const destination = await open({
      multiple: false,
      directory: true,
    });
    if (!destination) return;

    const report = await invoke<ExportReport>('export_favourites', {
      destination,
      mode,
      paths: paths && paths.length > 0 ? paths : null,
      preserveFolderStructure,
    });

    return { destination, report };
  } catch (error) {
    reportError(`Couldn't export favourites. ${errorText(error)}`);
    throw error;
  }
}

export function formatExportSuccess(
  mode: 'copy' | 'move',
  destination: string,
  report: ExportReport,
): string {
  const verb = mode === 'copy' ? 'copied' : 'moved';
  let message = `Favourites successfully ${verb} to ${destination}`;
  const notes: string[] = [];
  if (report.renamed > 0) {
    const noun = report.renamed === 1 ? 'file was' : 'files were';
    const whose = report.renamed === 1 ? 'that name was' : 'their names were';
    notes.push(
      `${report.renamed} ${noun} renamed because ${whose} already used`,
    );
  }
  if (report.skipped > 0) {
    const noun = report.skipped === 1 ? 'file was' : 'files were';
    const pronoun = report.skipped === 1 ? 'it was' : 'they were';
    notes.push(
      `${report.skipped} ${noun} skipped because ${pronoun} already in the destination`,
    );
  }
  if (notes.length > 0) {
    message += `. ${notes.join('. ')}.`;
  }
  return message;
}

export async function clearFavorites() {
  const confirmed = await confirm(
    'Are you sure you want to clear all favourites? This action cannot be undone.',
  );

  if (confirmed) {
    await favorites.clear();
  }
}

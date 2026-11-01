import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
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
import type {
  ExportFailure,
  ExportReport,
  PhotoMetadata,
} from '../../types/photo';

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
        console.error('Failed to record workspace:', e),
      );
    }
  } catch (error) {
    console.error('Failed to load photos from path:', error);
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
    console.error('Failed to load photos:', error);
    isLoading.set(false);
  }
}

export async function exportFavorites(
  mode: 'copy' | 'move' = 'copy',
  paths?: string[],
  preserveFolderStructure = false,
  isCancelled: () => boolean = () => false,
): Promise<{ destination: string; report: ExportReport } | undefined> {
  try {
    if (isCancelled()) return;

    const destination = await open({
      multiple: false,
      directory: true,
    });
    if (!destination || isCancelled()) return;

    const report = await invoke<ExportReport>('export_favourites', {
      destination,
      mode,
      paths: paths && paths.length > 0 ? paths : null,
      preserveFolderStructure,
    });

    return { destination, report };
  } catch (error) {
    console.error('Failed to export files:', error);
    throw error;
  }
}

export function describeExportError(error: unknown): string {
  if (typeof error === 'string' && error.trim()) {
    return error.trim();
  }
  if (error instanceof Error && error.message.trim()) {
    return error.message.trim();
  }
  return 'The export could not be completed.';
}

export function formatExportSuccess(
  mode: 'copy' | 'move',
  destination: string,
  report: ExportReport,
): string {
  const transferred = report.copied + report.renamed;
  const verbPast = mode === 'move' ? 'moved' : 'copied';
  const verbGerund = mode === 'move' ? 'moving' : 'copying';

  let message: string;
  if (report.cancelled) {
    message = `Cancelled after ${verbGerund} ${transferred} of ${report.total}`;
  } else if (report.failed > 0) {
    message = `Exported ${transferred} of ${report.total} to ${destination}`;
  } else {
    message = `Favourites successfully ${verbPast} to ${destination}`;
  }

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
  if (report.failures.length > 0) {
    notes.push(formatFailureNote(report.failures));
  }
  if (notes.length > 0) {
    message += `. ${notes.join('. ')}.`;
  }
  return message;
}

function formatFailureNote(failures: ExportFailure[]): string {
  const shown = failures.slice(0, 8);
  const detail = shown
    .map((failure) => `${failure.path}: ${failure.reason}`)
    .join('; ');
  const extra = failures.length - shown.length;
  const more = extra > 0 ? `; and ${extra} more` : '';
  const noun = failures.length === 1 ? 'file failed' : 'files failed';
  return `${failures.length} ${noun} (${detail}${more})`;
}

export async function clearFavorites() {
  const confirmed = await confirm(
    'Are you sure you want to clear all favourites? This action cannot be undone.',
  );

  if (confirmed) {
    await favorites.clear();
  }
}

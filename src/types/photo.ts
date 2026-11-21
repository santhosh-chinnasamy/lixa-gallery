export type { CullFlag, PhotoCullState } from '../lib/cull';

export interface ExifInfo {
  camera?: string;
  lens?: string;
  focal_length?: string;
  aperture?: string;
  shutter?: string;
  iso?: number;
  date?: string;
  captured_at?: number;
  captured_subsec?: number;
  width?: number;
  height?: number;
  orientation?: string;
}

export interface PhotoMetadata {
  metadata: FileMetadata;
  thumbnail_path: string;
  path: string;
  sidecar_path?: string;
  preview_path?: string;
  exif?: ExifInfo;
  is_raw?: boolean;
}

/** Return value of `scan_folder`. */
export interface FolderScanResult {
  generation: number;
  photos: PhotoMetadata[];
}

/** Payload for the `photos-updated` event. */
export interface PhotosUpdatedEvent {
  generation: number;
  photos: PhotoMetadata[];
}

export interface FileMetadata {
  name: string;
  modified: number;
  created: number;
  size: number;
}

export interface FavouriteFolderGroup {
  folder_path: string;
  folder_name: string;
  count: number;
  photos: PhotoMetadata[];
}

export interface Workspace {
  id: string;
  name: string;
  root_path: string;
  created_at: number;
  last_opened_at: number;
}

export interface ExportOptions {
  destination: string;
  mode: 'copy' | 'move';
  paths?: string[];
  preserve_folder_structure?: boolean;
}

export interface ExportFailure {
  path: string;
  reason: string;
}

export interface ExportReport {
  copied: number;
  renamed: number;
  skipped: number;
  failed: number;
  failures: ExportFailure[];
  cancelled: boolean;
  total: number;
}

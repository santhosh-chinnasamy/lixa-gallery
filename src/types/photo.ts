export interface PhotoMetadata {
  metadata: FileMetadata;
  thumbnail_path: string;
  path: string;
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

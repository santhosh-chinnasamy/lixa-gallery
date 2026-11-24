import type { PhotoMetadata } from '../types/photo';

export type PhotoSortBy = 'name' | 'date' | 'size';
export type PhotoSortOrder = 'asc' | 'desc';

/** Capture time when the file has it, otherwise the file's own timestamp. */
export function dateSortKey(photo: PhotoMetadata): number {
  const captured = photo.exif?.captured_at;
  if (captured != null) {
    return captured * 1_000_000 + (photo.exif?.captured_subsec ?? 0);
  }
  return (photo.metadata.created || photo.metadata.modified) * 1_000_000;
}

export function photoSortKey(
  photo: PhotoMetadata,
  sortBy: PhotoSortBy,
): string | number {
  switch (sortBy) {
    case 'name':
      return photo.metadata.name;
    case 'date':
      return dateSortKey(photo);
    case 'size':
      return photo.metadata.size;
  }
}

export function comparePhotos(
  a: PhotoMetadata,
  b: PhotoMetadata,
  sortBy: PhotoSortBy,
  sortOrder: PhotoSortOrder,
): number {
  let comparison = 0;
  switch (sortBy) {
    case 'name':
      comparison = a.metadata.name.localeCompare(b.metadata.name);
      break;
    case 'date':
      comparison = dateSortKey(a) - dateSortKey(b);
      break;
    case 'size':
      comparison = a.metadata.size - b.metadata.size;
      break;
  }
  return sortOrder === 'asc' ? comparison : -comparison;
}

export interface AppliedSort {
  sortBy: PhotoSortBy;
  sortOrder: PhotoSortOrder;
}

function samePathsAndSortKeys(
  previous: PhotoMetadata[],
  incoming: PhotoMetadata[],
  sortBy: PhotoSortBy,
): boolean {
  if (previous.length !== incoming.length) return false;
  const keys = new Map<string, string | number>();
  for (const photo of previous) {
    keys.set(photo.path, photoSortKey(photo, sortBy));
  }
  if (keys.size !== previous.length) return false;
  for (const photo of incoming) {
    const key = keys.get(photo.path);
    if (key === undefined || key !== photoSortKey(photo, sortBy)) return false;
  }
  return true;
}

/**
 * Keep `previous` order when every path is still present and its sort key is
 * unchanged. Thumbnail updates replace the object at that path and do not
 * reorder. Returns `previous` itself when no object identity changed.
 */
export function projectPhotoOrder(
  previous: PhotoMetadata[] | null,
  incoming: PhotoMetadata[],
  sortBy: PhotoSortBy,
  sortOrder: PhotoSortOrder,
  applied: AppliedSort | null,
): PhotoMetadata[] {
  const sortUnchanged =
    applied !== null &&
    applied.sortBy === sortBy &&
    applied.sortOrder === sortOrder;
  if (
    previous &&
    sortUnchanged &&
    samePathsAndSortKeys(previous, incoming, sortBy)
  ) {
    const byPath = new Map(incoming.map((photo) => [photo.path, photo]));
    let changed = false;
    const next = new Array<PhotoMetadata>(previous.length);
    for (let i = 0; i < previous.length; i++) {
      const photo = byPath.get(previous[i].path) ?? previous[i];
      if (photo !== previous[i]) changed = true;
      next[i] = photo;
    }
    return changed ? next : previous;
  }
  const sorted = incoming.slice();
  sorted.sort((a, b) => comparePhotos(a, b, sortBy, sortOrder));
  return sorted;
}

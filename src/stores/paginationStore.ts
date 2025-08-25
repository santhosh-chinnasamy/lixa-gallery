import { writable, derived, get } from 'svelte/store';
import type { PhotoMetadata } from '../types/photo';

// Configuration
export const PAGE_SIZE = 50; // Configurable page size
export const PRELOAD_PAGES = 1; // Number of pages to preload ahead/behind

// Core pagination state
export const currentPage = writable(0);
export const allPhotos = writable<PhotoMetadata[]>([]);
export const totalPages = derived(allPhotos, ($allPhotos) => 
  Math.ceil($allPhotos.length / PAGE_SIZE)
);

// Current page photos
export const currentPagePhotos = derived(
  [allPhotos, currentPage],
  ([$allPhotos, $currentPage]) => {
    const startIndex = $currentPage * PAGE_SIZE;
    const endIndex = Math.min(startIndex + PAGE_SIZE, $allPhotos.length);
    return $allPhotos.slice(startIndex, endIndex);
  }
);

// Preloaded photos (current + adjacent pages)
export const visiblePhotos = derived(
  [allPhotos, currentPage],
  ([$allPhotos, $currentPage]) => {
    const startPage = Math.max(0, $currentPage - PRELOAD_PAGES);
    const endPage = Math.min(
      Math.ceil($allPhotos.length / PAGE_SIZE) - 1,
      $currentPage + PRELOAD_PAGES
    );
    
    const startIndex = startPage * PAGE_SIZE;
    const endIndex = Math.min((endPage + 1) * PAGE_SIZE, $allPhotos.length);
    
    return {
      photos: $allPhotos.slice(startIndex, endIndex),
      startIndex,
      currentPageStart: $currentPage * PAGE_SIZE
    };
  }
);

// Pagination controls
export const paginationControls = {
  goToPage: (page: number) => {
    currentPage.update(current => {
      const maxPage = Math.ceil(get(allPhotos).length / PAGE_SIZE) - 1;
      return Math.max(0, Math.min(page, maxPage));
    });
  },
  
  nextPage: () => {
    currentPage.update(current => {
      const maxPage = Math.ceil(get(allPhotos).length / PAGE_SIZE) - 1;
      return Math.min(current + 1, maxPage);
    });
  },
  
  prevPage: () => {
    currentPage.update(current => Math.max(current - 1, 0));
  },
  
  reset: () => {
    currentPage.set(0);
  }
};

// Memory management
export const memoryStats = writable({
  totalImages: 0,
  loadedImages: 0,
  currentPage: 0,
  totalPages: 0
});

// Update memory stats when photos change
allPhotos.subscribe($photos => {
  memoryStats.update(stats => ({
    ...stats,
    totalImages: $photos.length,
    totalPages: Math.ceil($photos.length / PAGE_SIZE)
  }));
});

currentPage.subscribe($page => {
  memoryStats.update(stats => ({
    ...stats,
    currentPage: $page + 1
  }));
});

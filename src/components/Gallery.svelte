<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  import PaginationControls from './PaginationControls.svelte';
  import type { PhotoMetadata } from '../types/photo';
  import { onMount, onDestroy } from 'svelte';
  import { visiblePhotos, currentPage, memoryStats } from '../stores/paginationStore';
  
  export let photos: PhotoMetadata[];

  let selectedImage: PhotoMetadata | null = null;
  let galleryContainer: HTMLElement;
  let visibleImages = new Set<number>();
  let observer: IntersectionObserver | null = null;
  let loadedImageElements = new Map<string, HTMLImageElement>();

  $: displayPhotos = $visiblePhotos.photos;
  $: startIndex = $visiblePhotos.startIndex;
  $: currentPageStart = $visiblePhotos.currentPageStart;

  const handleImageClick = (photo: PhotoMetadata) => {
    selectedImage = photo;
  };

  const handleCloseModal = () => {
    selectedImage = null;
  };

  // Memory cleanup function
  const cleanupOffscreenImages = () => {
    const currentVisiblePaths = new Set(displayPhotos.map(p => p.path));
    
    for (const [path, imgElement] of loadedImageElements) {
      if (!currentVisiblePaths.has(path)) {
        // Force garbage collection by clearing src
        imgElement.src = '';
        loadedImageElements.delete(path);
      }
    }
  };

  // Track loaded images for cleanup
  const trackImageElement = (path: string, element: HTMLImageElement) => {
    loadedImageElements.set(path, element);
  };

  onMount(() => {
    if (!galleryContainer) return;

    // Optimized Intersection Observer with reduced preloading
    observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          const index = parseInt(entry.target.getAttribute('data-index') || '0');
          if (entry.isIntersecting) {
            visibleImages.add(index);
            // Minimal preloading - only 2 images ahead
            for (let i = index; i < Math.min(index + 2, displayPhotos.length); i++) {
              visibleImages.add(i);
            }
            visibleImages = visibleImages;
          } else {
            visibleImages.delete(index);
            visibleImages = visibleImages;
          }
        });
        
        // Update memory stats
        memoryStats.update(stats => ({
          ...stats,
          loadedImages: visibleImages.size
        }));
      },
      {
        root: galleryContainer,
        rootMargin: '50px', // Reduced from 100px
        threshold: 0.1
      }
    );

    return () => {
      if (observer) {
        observer.disconnect();
      }
    };
  });

  // Cleanup when page changes
  $: if ($currentPage !== undefined) {
    cleanupOffscreenImages();
    visibleImages.clear();
    visibleImages = visibleImages;
  }

  onDestroy(() => {
    if (observer) {
      observer.disconnect();
    }
    // Final cleanup
    loadedImageElements.clear();
  });

  // Re-observe when displayPhotos changes
  $: if (observer && galleryContainer && displayPhotos) {
    // Small delay to ensure DOM is updated
    setTimeout(() => {
      const imageCards = galleryContainer.querySelectorAll('[data-index]');
      imageCards.forEach((card) => observer?.observe(card));
    }, 10);
  }
</script>

<div class="flex flex-col h-full">
  <main class="flex-1 overflow-auto scroll-smooth" bind:this={galleryContainer}>
    <div class="h-full p-2 sm:p-3 md:p-4 lg:p-6 xl:p-8">
      {#if photos.length === 0}
        <div class="flex h-full items-center justify-center">
          <div class="text-center text-gray-500">
            <div class="mb-2 text-lg font-medium">No photos found</div>
            <div class="text-sm">Choose a folder to load images</div>
          </div>
        </div>
      {:else}
        <!-- Optimized grid for laptop/desktop with mobile support -->
        <div class="
          grid w-full
          gap-2 grid-cols-2
          xs:gap-3 xs:grid-cols-3
          sm:gap-4 sm:grid-cols-4
          md:gap-4 md:grid-cols-5
          lg:gap-5 lg:grid-cols-6
          xl:gap-6 xl:grid-cols-7
          2xl:gap-6 2xl:grid-cols-8
          auto-rows-fr
          justify-items-center
        ">
          {#each displayPhotos as photo, index}
            <div data-index={index} class="
              w-full max-w-[180px]
              xs:max-w-[160px]
              sm:max-w-[170px]
              md:max-w-[180px]
              lg:max-w-[190px]
              xl:max-w-[200px]
              2xl:max-w-[220px]
            ">
              <ImageCard 
                {photo} 
                tabindex={startIndex + index + 1} 
                {handleImageClick}
                {trackImageElement}
                isVisible={visibleImages.has(index) || index < 6}
                globalIndex={startIndex + index}
              />
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </main>
  
  <!-- Pagination Controls -->
  <PaginationControls />
</div>

<ImageModal {selectedImage} onClose={handleCloseModal} />

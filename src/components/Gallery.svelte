<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  import ControlBar from './ControlBar.svelte';
  import type { PhotoMetadata } from '../types/photo';
  import { onMount } from 'svelte';
  import {
    searchQuery,
    showImagesOnly,
    sortBy,
    sortOrder,
    gridSize,
  } from '../stores/galleryStore';

  let {
    photos = [],
    onLoadPhotos,
  }: { photos: PhotoMetadata[]; onLoadPhotos?: () => void } = $props();

  let selectedImage = $state<PhotoMetadata | null>(null);
  let galleryContainer = $state<HTMLElement | null>(null);
  let visibleImages = $state(new Set<number>());

  let processedPhotos = $derived.by(() => {
    let result = [...photos];

    if ($showImagesOnly) {
      result = result.filter(
        (p) =>
          !p.metadata.name.toLowerCase().endsWith('.xmp') &&
          !p.metadata.name.toLowerCase().endsWith('.txt') &&
          !p.metadata.name.toLowerCase().endsWith('.json') &&
          !p.metadata.name.toLowerCase().endsWith('.dng'),
      ); // Just rudimentary extension checks for "images only"
    }

    if ($searchQuery.trim()) {
      const q = $searchQuery.toLowerCase();
      result = result.filter((p) => p.metadata.name.toLowerCase().includes(q));
    }

    result.sort((a, b) => {
      let comparison = 0;
      switch ($sortBy) {
        case 'name':
          comparison = a.metadata.name.localeCompare(b.metadata.name);
          break;
        case 'date':
          comparison =
            (a.metadata.created || a.metadata.modified) -
            (b.metadata.created || b.metadata.modified);
          break;
        case 'size':
          comparison = a.metadata.size - b.metadata.size;
          break;
      }
      return $sortOrder === 'asc' ? comparison : -comparison;
    });

    return result;
  });

  const handleImageClick = (photo: PhotoMetadata) => {
    selectedImage = photo;
  };

  const handleCloseModal = () => {
    selectedImage = null;
  };

  // Re-run observer logic whenever photos change or container is ready
  $effect(() => {
    if (!galleryContainer || photos.length === 0) return;

    // Clear visible images when photos change to avoid showing old indexes
    visibleImages = new Set();

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          const indexAttr = entry.target.getAttribute('data-index');
          if (indexAttr === null) return;
          const index = parseInt(indexAttr);

          if (entry.isIntersecting) {
            visibleImages.add(index);
            // Preload next few images
            for (let i = index; i < Math.min(index + 8, photos.length); i++) {
              visibleImages.add(i);
            }
          } else {
            visibleImages.delete(index);
          }
        });
      },
      {
        root: galleryContainer,
        rootMargin: '200px', // Increased margin for smoother loading
        threshold: 0.1,
      },
    );

    // Observe all image cards - using a tiny delay to ensure DOM has updated
    const timeout = setTimeout(() => {
      if (!galleryContainer) return;
      const imageCards = galleryContainer.querySelectorAll('[data-index]');
      imageCards.forEach((card) => observer.observe(card));
    }, 0);

    return () => {
      clearTimeout(timeout);
      observer.disconnect();
    };
  });
</script>

<div
  class="relative flex h-full flex-col overflow-hidden"
  style="--grid-item-size: {$gridSize}px"
>
  <ControlBar {onLoadPhotos} />
  <main
    class="custom-scrollbar flex-1 overflow-y-auto scroll-smooth"
    bind:this={galleryContainer}
  >
    <div class="h-full p-6 lg:p-10">
      {#if processedPhotos.length === 0}
        <div class="flex h-full min-h-[400px] items-center justify-center">
          <div
            class="animate-in fade-in slide-in-from-bottom-4 text-center text-muted-foreground duration-700"
          >
            <div class="mb-2 text-lg font-medium">No results found</div>
            <p class="text-sm">
              {#if photos.length === 0}
                This directory doesn't appear to contain any supported files.
              {:else}
                No files match your current search and filter criteria.
              {/if}
            </p>
          </div>
        </div>
      {:else}
        <div
          class="grid w-full auto-rows-fr justify-items-center gap-3 sm:gap-4 md:gap-5"
          style="grid-template-columns: repeat(auto-fill, minmax(var(--grid-item-size), 1fr));"
        >
          {#each processedPhotos as photo, index (photo.path)}
            <div
              data-index={index}
              class="flex w-full justify-center"
              style="max-width: calc(var(--grid-item-size) * 1.5);"
            >
              <div class="w-full max-w-full">
                <ImageCard
                  {photo}
                  tabindex={index + 1}
                  {handleImageClick}
                  isVisible={visibleImages.has(index) || index < 12}
                />
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </main>
</div>

<ImageModal {selectedImage} onClose={handleCloseModal} />

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 6px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(0, 0, 0, 0.1);
    border-radius: 10px;
  }
  :global(.dark) .custom-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
  }
</style>

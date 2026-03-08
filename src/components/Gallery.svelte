<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  import ControlBar from './ControlBar.svelte';
  import type { PhotoMetadata } from '../types/photo';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import {
    searchQuery,
    showImagesOnly,
    sortBy,
    sortOrder,
    gridSize,
  } from '../stores/galleryStore';

  let { photos = [] }: { photos: PhotoMetadata[] } = $props();

  let selectedImage = $state<PhotoMetadata | null>(null);
  let galleryContainer = $state<HTMLElement | null>(null);
  let containerWidth = $state(0);

  // Calculate dynamic columns based on container width and grid size
  let columns = $derived(
    containerWidth > 0
      ? Math.max(1, Math.floor(containerWidth / ($gridSize + 16)))
      : 1,
  );

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

  // Initialize the head-less virtualizer instance
  let virtualizer = createVirtualizer({
    count: 0,
    getScrollElement: () => null,
    estimateSize: () => 100,
  });

  // Keep it reactive
  $effect(() => {
    // When the photo list changes (e.g navigating to a new folder), reset the scroll
    // to prevent the virtualizer from rendering items out of bounds or caching old heights.
    const count = processedPhotos.length;

    $virtualizer.setOptions({
      count,
      getScrollElement: () => galleryContainer,
      estimateSize: () => $gridSize + 48,
      overscan: 10,
      lanes: columns,
      onChange: () => {
        // Optional reactive sync hook
      },
    });

    if (galleryContainer && count > 0) {
      // Small tick to ensure the new virtualizer bounds are respected
      setTimeout(() => {
        $virtualizer.measure();
      }, 0);
    }
  });

  // Explicitly watch for photo array changes to force scroll to top
  $effect(() => {
    // Adding photos as a dependency triggers this block
    if (photos && galleryContainer) {
      galleryContainer.scrollTo({ top: 0 });
      setTimeout(() => {
        $virtualizer.measure();
      }, 10);
    }
  });
</script>

<div
  class="relative flex h-full flex-col overflow-hidden"
  style="--grid-item-size: {$gridSize}px"
>
  <ControlBar />
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
          bind:clientWidth={containerWidth}
          class="relative w-full"
          style="height: {$virtualizer.getTotalSize()}px;"
        >
          {#each $virtualizer.getVirtualItems() as virtualItem (processedPhotos[virtualItem.index]?.path ?? virtualItem.index)}
            {@const photo = processedPhotos[virtualItem.index]}
            {@const columnWidth = columns > 0 ? containerWidth / columns : 0}
            <div
              style="position: absolute; top: 0; left: 0; transform: translateY({virtualItem.start}px) translateX({virtualItem.lane *
                columnWidth}px); width: {columnWidth}px; height: {virtualItem.size}px; padding: 8px; display: flex; justify-content: center;"
            >
              <div
                style="width: 100%; max-width: calc(var(--grid-item-size) * 1.5);"
              >
                {#if photo}
                  <ImageCard
                    {photo}
                    tabindex={virtualItem.index + 1}
                    {handleImageClick}
                    isVisible={true}
                  />
                {/if}
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

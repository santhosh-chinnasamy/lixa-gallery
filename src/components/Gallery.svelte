<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  import SlideshowModal from './SlideshowModal.svelte';
  import ControlBar from './ControlBar.svelte';
  import type { PhotoMetadata } from '../types/photo';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { immediateSubfolders } from '$lib/folderEmptyState';
  import {
    searchQuery,
    sortBy,
    sortOrder,
    gridSize,
    folderTree,
    currentFolder,
  } from '../stores/galleryStore';
  import { loadSubfolder } from './common/ImageOperations';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

  let { photos = [] }: { photos: PhotoMetadata[] } = $props();

  let selectedImage = $state<PhotoMetadata | null>(null);
  let isSlideshowOpen = $state(false);
  let galleryContainer = $state<HTMLElement | null>(null);
  let containerWidth = $state(0);

  // Calculate dynamic columns based on container width and grid size
  let columns = $derived(
    containerWidth > 0
      ? Math.max(1, Math.floor(containerWidth / ($gridSize + 16)))
      : 1,
  );

  let subfolders = $derived(immediateSubfolders($folderTree, $currentFolder));

  let processedPhotos = $derived.by(() => {
    let result = [...photos];

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
  <ControlBar
    onStartSlideshow={() => (isSlideshowOpen = true)}
    canStartSlideshow={processedPhotos.length > 0}
  />
  <main
    class="custom-scrollbar flex-1 overflow-y-auto scroll-smooth"
    bind:this={galleryContainer}
  >
    <div class="h-full p-6 lg:p-10">
      {#if processedPhotos.length === 0}
        <div class="flex h-full min-h-[400px] items-center justify-center">
          {#if photos.length === 0 && subfolders.length > 0}
            <div
              class="animate-in fade-in slide-in-from-bottom-4 w-full max-w-lg duration-700"
            >
              <div class="mb-6 text-center">
                <div class="mb-2 text-lg font-medium text-foreground">
                  No photos in this folder
                </div>
                <p class="text-sm text-muted-foreground">
                  There are no supported images here. Open a subfolder to find
                  them.
                </p>
              </div>
              <div class="space-y-2">
                {#each subfolders as folder (folder.path)}
                  <button
                    type="button"
                    class="group flex w-full items-center justify-between rounded-xl border bg-background p-4 text-left transition-all hover:border-border hover:bg-accent"
                    onclick={() => loadSubfolder(folder.path)}
                  >
                    <span class="flex min-w-0 items-center gap-3">
                      <FolderIcon
                        size={18}
                        class="shrink-0 text-muted-foreground"
                      />
                      <span
                        class="truncate text-sm font-medium text-foreground"
                      >
                        {folder.name}
                      </span>
                    </span>
                    <ChevronRightIcon
                      size={18}
                      class="shrink-0 text-muted-foreground transition-transform group-hover:translate-x-1"
                    />
                  </button>
                {/each}
              </div>
            </div>
          {:else}
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
          {/if}
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

<ImageModal
  bind:selectedImage
  photos={processedPhotos}
  onClose={handleCloseModal}
/>

<SlideshowModal
  bind:open={isSlideshowOpen}
  photos={processedPhotos}
  onClose={() => (isSlideshowOpen = false)}
/>

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

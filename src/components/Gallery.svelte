<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  import SlideshowModal from './SlideshowModal.svelte';
  import ControlBar from './ControlBar.svelte';
  import type { PhotoMetadata } from '../types/photo';
  import type { CullAction } from '../lib/cull';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { tick } from 'svelte';
  import { get } from 'svelte/store';
  import {
    searchQuery,
    sortBy,
    sortOrder,
    gridSize,
    cullFilter,
    cullFlags,
    autoAdvance,
  } from '../stores/galleryStore';
  import {
    isTextEntry,
    setShortcutLayer,
    shortcutHelpOpen,
    shortcutMatches,
  } from '$lib/shortcuts';

  let { photos = [] }: { photos: PhotoMetadata[] } = $props();

  let selectedImage = $state<PhotoMetadata | null>(null);
  let isSlideshowOpen = $state(false);
  let galleryContainer = $state<HTMLElement | null>(null);
  let containerWidth = $state(0);
  let activeIndex = $state(0);
  let focusCard = $state(false);

  const pickedCount = $derived(
    photos.filter((photo) => $cullFlags.get(photo.path) === 'picked').length,
  );

  // Calculate dynamic columns based on container width and grid size
  let columns = $derived(
    containerWidth > 0
      ? Math.max(1, Math.floor(containerWidth / ($gridSize + 16)))
      : 1,
  );

  let processedPhotos = $derived.by(() => {
    let result = [...photos];

    if ($cullFilter !== 'all') {
      result = result.filter((photo) => {
        const flag = $cullFlags.get(photo.path) ?? 'unflagged';
        return flag === $cullFilter;
      });
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
    const index = processedPhotos.findIndex((item) => item.path === photo.path);
    if (index >= 0) activeIndex = index;
    selectedImage = photo;
  };

  const handleCloseModal = () => {
    selectedImage = null;
    focusCard = true;
  };

  function moveTo(index: number) {
    if (processedPhotos.length === 0) return;
    const next = Math.max(0, Math.min(processedPhotos.length - 1, index));
    activeIndex = next;
    focusCard = true;
    $virtualizer.scrollToIndex(next, { align: 'auto' });
  }

  function pageStride() {
    const rowHeight = $gridSize + 48;
    const rows = Math.max(
      1,
      Math.floor((galleryContainer?.clientHeight ?? rowHeight) / rowHeight),
    );
    return rows * columns;
  }

  async function markActive(action: CullAction) {
    const photo = processedPhotos[activeIndex];
    if (!photo) return;
    const index = activeIndex;
    const path = photo.path;
    await cullFlags.apply(path, action);
    if (!get(autoAdvance)) return;
    await tick();
    if (processedPhotos[index]?.path === path) {
      if (index < processedPhotos.length - 1) moveTo(index + 1);
    } else if (processedPhotos.length > 0) {
      moveTo(Math.min(index, processedPhotos.length - 1));
    }
  }

  function onGridKeydown(event: KeyboardEvent) {
    if (isTextEntry(event.target)) return;
    if (get(shortcutHelpOpen)) return;
    if (selectedImage || isSlideshowOpen) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (processedPhotos.length === 0) return;

    if (shortcutMatches(event, 'move-right')) {
      event.preventDefault();
      moveTo(activeIndex + 1);
    } else if (shortcutMatches(event, 'move-left')) {
      event.preventDefault();
      moveTo(activeIndex - 1);
    } else if (shortcutMatches(event, 'move-down')) {
      event.preventDefault();
      moveTo(activeIndex + columns);
    } else if (shortcutMatches(event, 'move-up')) {
      event.preventDefault();
      moveTo(activeIndex - columns);
    } else if (shortcutMatches(event, 'home')) {
      event.preventDefault();
      moveTo(0);
    } else if (shortcutMatches(event, 'end')) {
      event.preventDefault();
      moveTo(processedPhotos.length - 1);
    } else if (shortcutMatches(event, 'page-down')) {
      event.preventDefault();
      moveTo(activeIndex + pageStride());
    } else if (shortcutMatches(event, 'page-up')) {
      event.preventDefault();
      moveTo(activeIndex - pageStride());
    } else if (shortcutMatches(event, 'open-viewer')) {
      event.preventDefault();
      const photo = processedPhotos[activeIndex];
      if (photo) selectedImage = photo;
    } else if (shortcutMatches(event, 'pick')) {
      event.preventDefault();
      markActive('pick');
    } else if (shortcutMatches(event, 'reject')) {
      event.preventDefault();
      markActive('reject');
    } else if (shortcutMatches(event, 'unflag')) {
      event.preventDefault();
      markActive('unflag');
    } else if (shortcutMatches(event, 'start-slideshow')) {
      event.preventDefault();
      isSlideshowOpen = true;
    }
  }

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
  let listKey = $derived(photos.map((photo) => photo.path).join('\n'));
  let seenListKey = '';

  $effect(() => {
    if (listKey === seenListKey) return;
    seenListKey = listKey;
    activeIndex = 0;
    if (photos && galleryContainer) {
      galleryContainer.scrollTo({ top: 0 });
      setTimeout(() => {
        $virtualizer.measure();
      }, 10);
    }
  });

  $effect(() => {
    const length = processedPhotos.length;
    if (length === 0) {
      activeIndex = 0;
      return;
    }
    if (activeIndex > length - 1) activeIndex = length - 1;
  });

  $effect(() => {
    if (!focusCard || selectedImage || isSlideshowOpen) return;
    if (processedPhotos.length === 0) {
      focusCard = false;
      return;
    }
    const index = activeIndex;
    const rendered = $virtualizer
      .getVirtualItems()
      .some((item) => item.index === index);
    if (!rendered) {
      $virtualizer.scrollToIndex(index, { align: 'auto' });
      return;
    }
    tick().then(() => {
      const el = galleryContainer?.querySelector<HTMLElement>(
        `[data-gallery-index="${index}"]`,
      );
      el?.focus({ preventScroll: true });
      focusCard = false;
    });
  });

  $effect(() => {
    if (!selectedImage) return;
    const index = processedPhotos.findIndex(
      (photo) => photo.path === selectedImage?.path,
    );
    if (index >= 0) activeIndex = index;
  });

  $effect(() => {
    setShortcutLayer('grid', true);
    return () => setShortcutLayer('grid', false);
  });
</script>

<svelte:window onkeydown={onGridKeydown} />

<div
  class="relative flex h-full flex-col overflow-hidden"
  style="--grid-item-size: {$gridSize}px"
>
  <ControlBar
    onStartSlideshow={() => (isSlideshowOpen = true)}
    canStartSlideshow={processedPhotos.length > 0}
    photoCount={photos.length}
    {pickedCount}
  />
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
                    tabindex={virtualItem.index === activeIndex ? 0 : -1}
                    isActive={virtualItem.index === activeIndex}
                    galleryIndex={virtualItem.index}
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
  startIndex={activeIndex}
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

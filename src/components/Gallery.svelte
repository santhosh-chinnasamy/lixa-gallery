<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  import SlideshowModal from './SlideshowModal.svelte';
  import ControlBar from './ControlBar.svelte';
  import type { PhotoMetadata } from '../types/photo';
  import type { CullAction } from '../lib/cull';
  import { dateSortKey } from '$lib/photoView';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { immediateSubfolders } from '$lib/folderEmptyState';
  import { tick, untrack } from 'svelte';
  import { get } from 'svelte/store';
  import {
    searchQuery,
    sortBy,
    sortOrder,
    gridSize,
    folderTree,
    currentFolder,
    cullFilter,
    cullFlags,
    autoAdvance,
  } from '../stores/galleryStore';
  import { loadSubfolder } from './common/ImageOperations';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import {
    isTextEntry,
    setShortcutLayer,
    shortcutHelpOpen,
    shortcutMatches,
  } from '$lib/shortcuts';

  let { photos = [] }: { photos: PhotoMetadata[] } = $props();
  // Guard against a bad scan payload so the grid never receives a non-array.
  const photoList = $derived(Array.isArray(photos) ? photos : []);

  let selectedImage = $state<PhotoMetadata | null>(null);
  let isSlideshowOpen = $state(false);
  let galleryContainer = $state<HTMLElement | null>(null);
  let containerWidth = $state(0);
  let activeIndex = $state(0);
  let focusCard = $state(false);

  const pickedCount = $derived(
    photoList.filter((photo) => $cullFlags.get(photo.path) === 'picked')
      .length,
  );

  // Calculate dynamic columns based on container width and grid size
  let columns = $derived(
    containerWidth > 0
      ? Math.max(1, Math.floor(containerWidth / ($gridSize + 16)))
      : 1,
  );

  let subfolders = $derived(immediateSubfolders($folderTree, $currentFolder));

  let processedPhotos = $derived.by(() => {
    let result = [...photoList];

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
          comparison = dateSortKey(a) - dateSortKey(b);
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

  // Keep options in sync without subscribing to the virtualizer store.
  // `$virtualizer.setOptions` notifies the store (onChange → writable.set),
  // which would re-run this effect forever and freeze the UI.
  $effect(() => {
    const count = processedPhotos.length;
    const el = galleryContainer;
    const size = $gridSize;
    const lanes = columns;

    untrack(() => {
      const api = get(virtualizer);
      api.setOptions({
        count,
        getScrollElement: () => el,
        estimateSize: () => size + 48,
        overscan: 10,
        lanes,
      });
      if (el && count > 0) {
        queueMicrotask(() => api.measure());
      }
    });
  });

  // Explicitly watch for photo array changes to force scroll to top
  let listKey = $derived(photoList.map((photo) => photo.path).join('\n'));
  let seenListKey = '';

  $effect(() => {
    if (listKey === seenListKey) return;
    seenListKey = listKey;
    activeIndex = 0;
    const el = galleryContainer;
    if (el) {
      el.scrollTo({ top: 0 });
      untrack(() => {
        const api = get(virtualizer);
        queueMicrotask(() => api.measure());
      });
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
    const api = untrack(() => get(virtualizer));
    const rendered = api
      .getVirtualItems()
      .some((item) => item.index === index);
    if (!rendered) {
      api.scrollToIndex(index, { align: 'auto' });
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
    photoCount={photoList.length}
    {pickedCount}
  />
  <main
    class="custom-scrollbar flex-1 overflow-y-auto scroll-smooth"
    bind:this={galleryContainer}
  >
    <div class="h-full p-6 lg:p-10">
      {#if processedPhotos.length === 0}
        <div class="flex h-full min-h-[400px] items-center justify-center">
          {#if photoList.length === 0 && subfolders.length > 0}
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
                {#if photoList.length === 0}
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

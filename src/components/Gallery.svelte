<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  import SlideshowModal from './SlideshowModal.svelte';
  import ControlBar from './ControlBar.svelte';
  import type { PhotoMetadata } from '../types/photo';
  import type { CullAction } from '../lib/cull';
  import { projectPhotoOrder, type AppliedSort } from '$lib/photoOrder';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { immediateSubfolders } from '$lib/folderEmptyState';
  import { tick, untrack } from 'svelte';
  import { get } from 'svelte/store';
  import {
    searchQuery,
    sortBy,
    sortOrder,
    photosRevision,
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

  let {
    photos = [],
    embedded = false,
    source = 'library',
  }: {
    photos: PhotoMetadata[];
    embedded?: boolean;
    source?: 'library' | 'favourites';
  } = $props();
  // Guard against a bad scan payload so the grid never receives a non-array.
  const photoList = $derived(Array.isArray(photos) ? photos : []);
  const fromFavourites = $derived(source === 'favourites');

  let selectedImage = $state<PhotoMetadata | null>(null);
  let isSlideshowOpen = $state(false);
  let galleryContainer = $state<HTMLElement | null>(null);
  /** Stable grid width; ignore transient 0/tiny values that unmount the virtual row. */
  let containerWidth = $state(0);
  let activeIndex = $state(0);
  let focusCard = $state(false);
  let widthProbe = $state<HTMLElement | null>(null);

  const pickedCount = $derived(
    photoList.filter((photo) => $cullFlags.get(photo.path) === 'picked').length,
  );

  // Bucket width so scrollbar / sub-pixel jitter cannot flip column count.
  let widthBucket = $derived(Math.floor(containerWidth / 16) * 16);
  let columns = $derived(
    widthBucket > 0
      ? Math.max(1, Math.floor(widthBucket / ($gridSize + 16)))
      : 1,
  );

  let subfolders = $derived(immediateSubfolders($folderTree, $currentFolder));

  // Last displayed order. Thumbnail patches keep it so the grid does not
  // localeCompare the whole folder on every batch.
  let orderedPhotos: PhotoMetadata[] | null = null;
  let appliedSort: AppliedSort | null = null;

  let processedPhotos = $derived.by(() => {
    // In-place thumbnail patches keep the photos array identity. This counter
    // is what makes the derived re-read those slots.
    void $photosRevision;
    let incoming = photoList;

    // Favourites already passes a curated list. Library cull/search must not
    // re-filter it (empty cullFlags + cull_filter=picked blanks the grid).
    if (!fromFavourites) {
      if ($cullFilter !== 'all') {
        incoming = incoming.filter((photo) => {
          const flag = $cullFlags.get(photo.path) ?? 'unflagged';
          return flag === $cullFilter;
        });
      }

      if ($searchQuery.trim()) {
        const q = $searchQuery.toLowerCase();
        incoming = incoming.filter((p) =>
          p.metadata.name.toLowerCase().includes(q),
        );
      }
    }

    const sortByNow = $sortBy;
    const sortOrderNow = $sortOrder;
    const next = projectPhotoOrder(
      orderedPhotos,
      incoming,
      sortByNow,
      sortOrderNow,
      appliedSort,
    );
    orderedPhotos = next;
    appliedSort = { sortBy: sortByNow, sortOrder: sortOrderNow };
    return next;
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

  // Width from a short probe (not the tall virtual content). Rapid scroll on
  // WebKit used to flicker bind:clientWidth → containerWidth < 48 → unmount
  // every card, leaving the virtualizer range stuck on one empty row.
  $effect(() => {
    const el = widthProbe;
    if (!el || typeof ResizeObserver === 'undefined') return;

    const applyWidth = (next: number) => {
      untrack(() => {
        if (next > 48) {
          if (Math.abs(next - containerWidth) >= 1) containerWidth = next;
          return;
        }
        // Ignore transient collapse; keep last good width for columns/layout.
      });
    };

    applyWidth(el.clientWidth);
    const ro = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width ?? el.clientWidth;
      applyWidth(w);
    });
    ro.observe(el);
    return () => ro.disconnect();
  });

  // Keep options in sync without subscribing to the virtualizer store.
  // `$virtualizer.setOptions` notifies the store (onChange → writable.set),
  // which would re-run this effect forever and freeze the UI.
  // Also skip no-op updates: measure() can nudge width → columns → effect,
  // which pegged WebKit when Favourites rendered ~100+ cards.
  let lastVirtualOpts = { count: -1, size: -1, lanes: -1, hasEl: false };
  let measureRaf = 0;

  function scheduleMeasure() {
    if (measureRaf) cancelAnimationFrame(measureRaf);
    measureRaf = requestAnimationFrame(() => {
      measureRaf = 0;
      get(virtualizer).measure();
    });
  }

  $effect(() => {
    const count = processedPhotos.length;
    const el = galleryContainer;
    const size = $gridSize;
    const lanes = columns;
    const hasEl = !!el;
    const overscan = embedded ? 5 : 12;

    untrack(() => {
      const prev = lastVirtualOpts;
      if (
        prev.count === count &&
        prev.size === size &&
        prev.lanes === lanes &&
        prev.hasEl === hasEl
      ) {
        return;
      }

      const lanesOnly =
        prev.count === count &&
        prev.size === size &&
        prev.hasEl === hasEl &&
        prev.lanes !== lanes;

      lastVirtualOpts = { count, size, lanes, hasEl };

      const api = get(virtualizer);
      api.setOptions({
        count,
        getScrollElement: () => el,
        estimateSize: () => size + 48,
        overscan,
        lanes,
      });
      if (!el || count === 0) return;

      if (lanesOnly) {
        scheduleMeasure();
      } else {
        queueMicrotask(() => api.measure());
      }
    });

    return () => {
      if (measureRaf) {
        cancelAnimationFrame(measureRaf);
        measureRaf = 0;
      }
    };
  });

  // After fast front/back scroll, WebKit can leave getVirtualItems() empty or
  // stuck on a single row while total size stays huge. Remeasure on settle.
  $effect(() => {
    const el = galleryContainer;
    if (!el) return;

    let settleTimer = 0;
    const recover = () => {
      const api = get(virtualizer);
      const count = untrack(() => processedPhotos.length);
      if (count === 0) return;
      const visible = api.getVirtualItems().length;
      const rowH = untrack(() => $gridSize) + 48;
      const viewportRows = Math.max(
        1,
        Math.ceil(el.clientHeight / Math.max(rowH, 1)),
      );
      const lanes = untrack(() => columns);
      // One stuck row (screenshot symptom) is ~lanes items; a healthy
      // viewport should paint roughly viewportRows * lanes (+ overscan).
      const expectedMin = Math.min(
        count,
        Math.max(lanes + 1, viewportRows * lanes),
      );
      if (visible === 0 || visible < expectedMin) {
        // Remeasure + resync scroll offset (stale offset → blank grid).
        const top = el.scrollTop;
        api.measure();
        api.scrollToOffset(top);
      }
    };

    const onScroll = () => {
      if (settleTimer) clearTimeout(settleTimer);
      settleTimer = window.setTimeout(recover, 80);
    };
    const onScrollEnd = () => recover();

    el.addEventListener('scroll', onScroll, { passive: true });
    el.addEventListener('scrollend', onScrollEnd);
    return () => {
      el.removeEventListener('scroll', onScroll);
      el.removeEventListener('scrollend', onScrollEnd);
      if (settleTimer) clearTimeout(settleTimer);
    };
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
    const rendered = api.getVirtualItems().some((item) => item.index === index);
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
    if (embedded || fromFavourites) return;
    setShortcutLayer('grid', true);
    return () => setShortcutLayer('grid', false);
  });
</script>

<svelte:window onkeydown={onGridKeydown} />

<div
  class="relative flex h-full flex-col overflow-hidden"
  style="--grid-item-size: {$gridSize}px"
>
  {#if !embedded}
    <ControlBar
      onStartSlideshow={() => (isSlideshowOpen = true)}
      canStartSlideshow={processedPhotos.length > 0}
      photoCount={photoList.length}
      {pickedCount}
    />
  {/if}
  <main
    class="custom-scrollbar flex-1 overflow-y-auto"
    bind:this={galleryContainer}
  >
    <div class={embedded ? 'h-full p-2' : 'h-full p-6 lg:p-10'}>
      <!-- Width probe: short element so scroll cannot zero clientWidth. -->
      <div
        bind:this={widthProbe}
        class="h-0 w-full overflow-hidden"
        aria-hidden="true"
      ></div>
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
          class="relative w-full"
          style="height: {$virtualizer.getTotalSize()}px;"
        >
          {#each $virtualizer.getVirtualItems() as virtualItem (virtualItem.key)}
            {@const photo = processedPhotos[virtualItem.index]}
            {@const columnWidth = columns > 0 ? containerWidth / columns : 0}
            <div
              style="position: absolute; top: 0; left: 0; transform: translateY({virtualItem.start}px) translateX({virtualItem.lane *
                columnWidth}px); width: {columnWidth}px; height: {virtualItem.size}px; padding: 8px; display: flex; justify-content: center;"
            >
              <div
                style="width: 100%; max-width: calc(var(--grid-item-size) * 1.5);"
              >
                {#if photo && columnWidth > 0}
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

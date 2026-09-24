<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { cn } from '$lib/utils';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, scale as scaleTransition } from 'svelte/transition';
  import { favorites, photos } from '../stores/galleryStore';
  import Filename from './Filename.svelte';
  import Heart from './icons/Heart.svelte';
  import ArrowRight from './icons/ArrowRight.svelte';
  import { page } from '$app/state';
  import ArrowLeft from './icons/ArrowLeft.svelte';
  import Close from './icons/Close.svelte';
  import Info from '@lucide/svelte/icons/info';
  import ZoomIn from '@lucide/svelte/icons/zoom-in';
  import ZoomOut from '@lucide/svelte/icons/zoom-out';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import type { PhotoMetadata } from '../types/photo';

  // Props using Svelte 5 props rune
  let {
    selectedImage = $bindable(),
    onClose,
    photos: photosProp,
    source = page.url.pathname === '/' || page.url.pathname === '/index.html'
      ? 'all'
      : 'favorites',
  }: {
    selectedImage: PhotoMetadata | null;
    onClose: () => void;
    photos?: PhotoMetadata[];
    source?: 'all' | 'favorites';
  } = $props();

  // Reactive derived values
  const photoSource = $derived(
    photosProp ??
      (source === 'favorites'
        ? $photos.filter((photo) => $favorites.has(photo.path))
        : $photos),
  );

  const currentIndex = $derived.by(() => {
    if (!selectedImage) return -1;
    const currentPath = selectedImage.path;
    return photoSource.findIndex((photo) => photo.path === currentPath);
  });

  const canShowPrevious = $derived(currentIndex > 0);
  const canShowNext = $derived(currentIndex < photoSource.length - 1);
  const isFavourite = $derived(
    selectedImage ? $favorites.has(selectedImage.path) : false,
  );
  const fileName = $derived(selectedImage?.metadata.name);

  // Modal and Image states
  let imageLoaded = $state(false);
  let imageError = $state(false);
  const preloadedImages = new Map<string, HTMLImageElement>();

  let showInfo = $state(false);
  let imageWidth = $state(0);
  let imageHeight = $state(0);

  let showUI = $state(true);
  let uiTimeoutRef: ReturnType<typeof setTimeout>;

  // Zoom, Pan, and Filter states
  let zoomScale = $state(1.0);
  let offsetX = $state(0);
  let offsetY = $state(0);
  let isDragging = $state(false);
  let activeFilter = $state<'none' | 'grayscale' | 'sepia' | 'invert'>('none');

  let imageEl = $state<HTMLImageElement | null>(null);
  let containerWidth = $state(0);
  let containerHeight = $state(0);

  // Mouse drag coordinates
  let startX = 0;
  let startY = 0;

  // Touch gesture state
  let isTouchDragging = $state(false);
  let touchStartX = 0;
  let touchStartY = 0;
  let initialPinchDistance = 0;
  let initialScale = 1.0;

  const keyboardActions = {
    Escape: onClose,
    ArrowLeft: showPrevious,
    ArrowRight: showNext,
    l: toggleFavorite,
  } as const;

  function handleKeydown(event: KeyboardEvent) {
    if (!selectedImage) return;

    const action = keyboardActions[event.key as keyof typeof keyboardActions];
    if (action) {
      event.preventDefault();
      action();
    }
  }

  function toggleFavorite() {
    if (selectedImage) favorites.toggle(selectedImage.path);
  }

  function showPrevious() {
    if (canShowPrevious) {
      selectedImage = photoSource[currentIndex - 1];
    }
  }

  function showNext() {
    if (canShowNext) {
      selectedImage = photoSource[currentIndex + 1];
    }
  }

  function handleBackdropClick(event: Event) {
    if (event.target === event.currentTarget) {
      onClose();
    }
  }

  function handleImageLoad(e: Event) {
    const img = e.target as HTMLImageElement;
    imageWidth = img.naturalWidth;
    imageHeight = img.naturalHeight;
    imageLoaded = true;
    imageError = false;
  }

  function handleImageError() {
    imageError = true;
    imageLoaded = false;
  }

  // Preload adjacent images
  function preloadAdjacentImages() {
    if (!selectedImage || currentIndex === -1) return;

    const indicesToPreload = [];
    if (canShowPrevious) indicesToPreload.push(currentIndex - 1);
    if (canShowNext) indicesToPreload.push(currentIndex + 1);

    // Preload next 2 images in each direction for smoother navigation
    if (currentIndex - 2 >= 0) indicesToPreload.push(currentIndex - 2);
    if (currentIndex + 2 < photoSource.length)
      indicesToPreload.push(currentIndex + 2);

    indicesToPreload.forEach((index) => {
      const photo = photoSource[index];
      if (photo && !preloadedImages.has(photo.path)) {
        const img = new Image();
        img.src = convertFileSrc(photo.path);
        preloadedImages.set(photo.path, img);
      }
    });
  }

  // Zoom / Pan helper functions
  function clamp(val: number, min: number, max: number) {
    return Math.max(min, Math.min(max, val));
  }

  function getBounds() {
    if (!imageEl || !containerWidth || !containerHeight) {
      return { minX: 0, maxX: 0, minY: 0, maxY: 0 };
    }
    const renderedWidth = imageEl.clientWidth;
    const renderedHeight = imageEl.clientHeight;

    // Bounds check to prevent panning empty space when zoomed image fits or exceeds screen
    const maxOffsetX = Math.max(
      0,
      (renderedWidth * zoomScale - containerWidth) / 2,
    );
    const maxOffsetY = Math.max(
      0,
      (renderedHeight * zoomScale - containerHeight) / 2,
    );
    return {
      minX: -maxOffsetX,
      maxX: maxOffsetX,
      minY: -maxOffsetY,
      maxY: maxOffsetY,
    };
  }

  function clampOffsets() {
    const bounds = getBounds();
    offsetX = clamp(offsetX, bounds.minX, bounds.maxX);
    offsetY = clamp(offsetY, bounds.minY, bounds.maxY);
  }

  function zoomIn() {
    zoomScale = Math.min(5.0, zoomScale + 0.5);
  }

  function zoomOut() {
    zoomScale = Math.max(1.0, zoomScale - 0.5);
  }

  function resetAll() {
    zoomScale = 1.0;
    offsetX = 0;
    offsetY = 0;
    activeFilter = 'none';
  }

  // Mouse Dragging handlers
  function handleMouseDown(e: MouseEvent) {
    if (zoomScale <= 1.0) return;
    e.preventDefault();
    isDragging = true;
    startX = e.clientX - offsetX;
    startY = e.clientY - offsetY;
  }

  function handleWindowMouseMove(e: MouseEvent) {
    resetUITimer();
    if (!isDragging) return;
    const bounds = getBounds();
    offsetX = clamp(e.clientX - startX, bounds.minX, bounds.maxX);
    offsetY = clamp(e.clientY - startY, bounds.minY, bounds.maxY);
  }

  function handleWindowMouseUp() {
    isDragging = false;
    isTouchDragging = false;
  }

  function handleDoubleClick(e: MouseEvent) {
    e.preventDefault();
    if (zoomScale > 1.0) {
      zoomScale = 1.0;
      offsetX = 0;
      offsetY = 0;
    } else {
      zoomScale = 2.5;
      offsetX = 0;
      offsetY = 0;
    }
  }

  function handleWheel(e: WheelEvent) {
    if (!imageLoaded) return;
    e.preventDefault();
    const zoomFactor = 1.1;
    let newScale = zoomScale;
    if (e.deltaY < 0) {
      newScale = Math.min(5.0, zoomScale * zoomFactor);
    } else {
      newScale = Math.max(1.0, zoomScale / zoomFactor);
    }
    zoomScale = newScale;
  }

  // Touch gesture handlers
  function handleTouchStart(e: TouchEvent) {
    resetUITimer();
    if (!imageLoaded) return;

    if (e.touches.length === 1) {
      if (zoomScale > 1.0) {
        isTouchDragging = true;
        touchStartX = e.touches[0].clientX - offsetX;
        touchStartY = e.touches[0].clientY - offsetY;
      }
    } else if (e.touches.length === 2) {
      isTouchDragging = false;
      const dx = e.touches[0].clientX - e.touches[1].clientX;
      const dy = e.touches[0].clientY - e.touches[1].clientY;
      initialPinchDistance = Math.sqrt(dx * dx + dy * dy);
      initialScale = zoomScale;
    }
  }

  function handleTouchMove(e: TouchEvent) {
    resetUITimer();
    if (!imageLoaded) return;

    if (e.touches.length === 1 && isTouchDragging) {
      e.preventDefault();
      const bounds = getBounds();
      offsetX = clamp(
        e.touches[0].clientX - touchStartX,
        bounds.minX,
        bounds.maxX,
      );
      offsetY = clamp(
        e.touches[0].clientY - touchStartY,
        bounds.minY,
        bounds.maxY,
      );
    } else if (e.touches.length === 2 && initialPinchDistance > 0) {
      e.preventDefault();
      const dx = e.touches[0].clientX - e.touches[1].clientX;
      const dy = e.touches[0].clientY - e.touches[1].clientY;
      const currentDistance = Math.sqrt(dx * dx + dy * dy);
      const factor = currentDistance / initialPinchDistance;
      zoomScale = clamp(initialScale * factor, 1.0, 5.0);
    }
  }

  function handleTouchEnd(e: TouchEvent) {
    if (e.touches.length === 0) {
      isTouchDragging = false;
      initialPinchDistance = 0;
    } else if (e.touches.length === 1) {
      isTouchDragging = true;
      touchStartX = e.touches[0].clientX - offsetX;
      touchStartY = e.touches[0].clientY - offsetY;
      initialPinchDistance = 0;
    }
  }

  // Info / Utility functions
  function formatBytes(bytes: number, decimals = 2) {
    if (!+bytes) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
  }

  function toggleInfo() {
    showInfo = !showInfo;
  }

  function resetUITimer() {
    showUI = true;
    clearTimeout(uiTimeoutRef);
    uiTimeoutRef = setTimeout(() => {
      // Don't auto-hide UI if zoomed in or dragging
      if (zoomScale === 1.0 && !isDragging && !isTouchDragging) {
        showUI = false;
      }
    }, 2500);
  }

  function handleMouseLeave() {
    clearTimeout(uiTimeoutRef);
    // Don't auto-hide UI if zoomed in
    if (zoomScale === 1.0) {
      showUI = false;
    }
  }

  // Filter style string builder
  const filterStyle = $derived.by(() => {
    switch (activeFilter) {
      case 'grayscale':
        return 'grayscale(100%)';
      case 'sepia':
        return 'sepia(100%)';
      case 'invert':
        return 'invert(100%)';
      default:
        return '';
    }
  });

  // Effects for reactivity
  $effect(() => {
    if (selectedImage) {
      imageLoaded = false;
      imageError = false;
      resetAll();
      preloadAdjacentImages();
      resetUITimer();
    }
    return () => clearTimeout(uiTimeoutRef);
  });

  $effect(() => {
    // Automatically runs when zoomScale, containerWidth, or containerHeight changes
    if (zoomScale || containerWidth || containerHeight) {
      clampOffsets();
      showUI = true;
    }
  });
</script>

<svelte:window
  onkeydown={handleKeydown}
  onmousemove={handleWindowMouseMove}
  onmouseup={handleWindowMouseUp}
/>

{#if selectedImage}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/85 p-4 backdrop-blur-3xl transition-colors duration-500"
    role="dialog"
    aria-modal="true"
    aria-label="Image preview"
    tabindex="-1"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    onmousemove={resetUITimer}
    onmouseleave={handleMouseLeave}
    in:fade={{ duration: 200 }}
    out:fade={{ duration: 150 }}
  >
    <!-- Close button & top controls -->
    <div
      class={`absolute right-4 top-4 z-20 transition-opacity duration-300 ${
        showUI ? 'opacity-100' : 'pointer-events-none opacity-0'
      }`}
    >
      <Button
        variant="ghost"
        size="icon"
        class="h-12 w-12 rounded-full bg-black/50 text-white hover:bg-black/70 focus:ring-2 focus:ring-white/50"
        onclick={onClose}
        aria-label="Close preview"
      >
        <Close />
      </Button>
    </div>

    <!-- Main modal content -->
    <div
      class="relative flex h-full max-h-[95vh] w-full max-w-7xl flex-col bg-white/10"
      in:scaleTransition={{ duration: 200, start: 0.95 }}
      out:scaleTransition={{ duration: 150, start: 0.95 }}
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeydown}
      role="button"
      tabindex="0"
    >
      <!-- Image container -->
      <div
        bind:clientWidth={containerWidth}
        bind:clientHeight={containerHeight}
        onwheel={handleWheel}
        ontouchstart={handleTouchStart}
        ontouchmove={handleTouchMove}
        ontouchend={handleTouchEnd}
        class="relative flex flex-1 cursor-default select-none items-center justify-center overflow-hidden rounded-lg"
      >
        <!-- Loading spinner -->
        {#if !imageLoaded && !imageError}
          <div
            class="absolute inset-0 flex items-center justify-center bg-black/20 backdrop-blur-sm"
          >
            <div
              class="h-12 w-12 animate-spin rounded-full border-b-2 border-white"
            ></div>
          </div>
        {/if}

        <!-- Error state -->
        {#if imageError}
          <div
            class="absolute inset-0 flex items-center justify-center bg-black/20 backdrop-blur-sm"
          >
            <div class="text-center text-white">
              <div class="mb-2 text-lg">Failed to load image</div>
              <div class="text-sm text-white/70">{fileName}</div>
            </div>
          </div>
        {/if}

        <!-- Thumbnail placeholder/fallback -->
        {#if !imageLoaded || imageError}
          <img
            src={`lixa-thumbnail://localhost/${encodeURIComponent(selectedImage.path)}`}
            alt=""
            class={`absolute inset-0 h-full w-full rounded-lg object-contain blur-md transition-opacity duration-500 ${
              imageError ? 'opacity-50' : 'opacity-20'
            }`}
          />
        {/if}

        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- Main image -->
        <img
          bind:this={imageEl}
          src={convertFileSrc(selectedImage.path)}
          alt={fileName}
          draggable="false"
          class={`relative z-10 max-h-full max-w-full rounded-lg object-contain shadow-2xl transition-opacity duration-300 ${
            imageLoaded ? 'opacity-100' : 'opacity-0'
          } ${zoomScale > 1.0 ? 'cursor-grab' : 'cursor-default'} ${isDragging || isTouchDragging ? 'cursor-grabbing' : ''}`}
          style="filter: drop-shadow(0 25px 25px rgb(0 0 0 / 0.5)) {filterStyle
            ? ' ' + filterStyle
            : ''}; transform: translate({offsetX}px, {offsetY}px) scale({zoomScale}); transition: {isDragging ||
          isTouchDragging
            ? 'none'
            : 'transform 0.15s ease-out, filter 0.2s ease-in-out'}; transform-origin: center center;"
          onload={handleImageLoad}
          onerror={handleImageError}
          onmousedown={handleMouseDown}
          ondblclick={handleDoubleClick}
        />

        <!-- Info Panel Overlay -->
        <div
          class={`absolute bottom-0 right-0 top-0 z-30 w-72 transform bg-black/60 p-6 text-sm text-white shadow-2xl backdrop-blur-xl transition-transform duration-300 ease-in-out ${
            showInfo ? 'translate-x-0' : 'translate-x-full'
          }`}
        >
          <div
            class="mb-4 flex items-center justify-between border-b border-white/20 pb-2"
          >
            <h3 class="flex items-center gap-2 text-lg font-semibold">
              <Info size={18} /> Info
            </h3>
            <Button
              variant="ghost"
              size="icon"
              class="h-8 w-8 text-white/70 hover:text-white"
              onclick={toggleInfo}
            >
              <Close />
            </Button>
          </div>

          <div class="space-y-4">
            <div>
              <p
                class="text-xs font-medium uppercase tracking-wider text-white/50"
              >
                File Name
              </p>
              <p class="mt-1 break-words">{fileName}</p>
            </div>

            <div class="grid grid-cols-2 gap-4">
              <div>
                <p
                  class="text-xs font-medium uppercase tracking-wider text-white/50"
                >
                  Size
                </p>
                <p class="mt-1">{formatBytes(selectedImage.metadata.size)}</p>
              </div>
              <div>
                <p
                  class="text-xs font-medium uppercase tracking-wider text-white/50"
                >
                  Format
                </p>
                <p class="mt-1 capitalize">
                  {fileName?.split('.').pop() || 'Unknown'}
                </p>
              </div>
            </div>

            <div>
              <p
                class="text-xs font-medium uppercase tracking-wider text-white/50"
              >
                Resolution
              </p>
              <p class="mt-1">
                {#if imageLoaded && imageWidth > 0}
                  {imageWidth} × {imageHeight}
                {:else}
                  Loading...
                {/if}
              </p>
            </div>

            <div>
              <p
                class="text-xs font-medium uppercase tracking-wider text-white/50"
              >
                Date Modified
              </p>
              <p class="mt-1">
                {new Intl.DateTimeFormat(undefined, {
                  dateStyle: 'medium',
                  timeStyle: 'short',
                }).format(new Date(selectedImage.metadata.modified * 1000))}
              </p>
            </div>
          </div>
        </div>
      </div>

      <!-- Zoom & Filter Controls -->
      <div
        class={`mt-2 flex shrink-0 flex-col items-center justify-center transition-opacity duration-300 ${
          showUI ? 'opacity-100' : 'pointer-events-none opacity-0'
        }`}
      >
        <div
          class="flex flex-wrap items-center justify-center gap-3 rounded-2xl border border-white/10 bg-black/60 p-2.5 text-white shadow-xl backdrop-blur-md"
        >
          <!-- Zoom Out Button -->
          <Button
            variant="ghost"
            size="icon"
            disabled={zoomScale <= 1.0}
            onclick={zoomOut}
            class="h-12 w-12 rounded-xl text-white transition-all hover:bg-white/10 disabled:opacity-30"
            aria-label="Zoom out"
          >
            <ZoomOut size={20} />
          </Button>

          <!-- Zoom Percentage -->
          <span
            class="min-w-[50px] text-center text-sm font-semibold text-white/90"
          >
            {Math.round(zoomScale * 100)}%
          </span>

          <!-- Zoom In Button -->
          <Button
            variant="ghost"
            size="icon"
            disabled={zoomScale >= 5.0}
            onclick={zoomIn}
            class="h-12 w-12 rounded-xl text-white transition-all hover:bg-white/10 disabled:opacity-30"
            aria-label="Zoom in"
          >
            <ZoomIn size={20} />
          </Button>

          <!-- Divider -->
          <div class="h-8 w-px bg-white/20"></div>

          <!-- Filters Toggle Group -->
          <div class="flex items-center gap-1">
            {#each ['none', 'grayscale', 'sepia', 'invert'] as filterOption}
              <Button
                variant={activeFilter === filterOption ? 'secondary' : 'ghost'}
                onclick={() =>
                  (activeFilter = filterOption as typeof activeFilter)}
                class={`h-12 rounded-xl px-4 text-sm font-medium capitalize transition-all duration-200 ${
                  activeFilter === filterOption
                    ? 'bg-white font-semibold text-black shadow-md'
                    : 'text-white/80 hover:bg-white/10 hover:text-white'
                }`}
                aria-label={`Apply ${filterOption === 'none' ? 'no' : filterOption} filter`}
                aria-pressed={activeFilter === filterOption}
              >
                {filterOption === 'none' ? 'Normal' : filterOption}
              </Button>
            {/each}
          </div>

          <!-- Reset Button -->
          {#if zoomScale > 1.0 || activeFilter !== 'none'}
            <div class="flex items-center pl-1" in:fade={{ duration: 150 }}>
              <Button
                variant="secondary"
                onclick={resetAll}
                class="flex h-12 items-center gap-2 rounded-xl border-0 bg-amber-500 px-4 font-semibold text-black shadow-md transition-all duration-200 hover:bg-amber-400"
                aria-label="Reset zoom and filters"
              >
                <RotateCcw size={16} />
                <span>Reset</span>
              </Button>
            </div>
          {/if}
        </div>
      </div>

      <!-- Navigation & Info Control Bar -->
      <div
        class={`mt-2 flex shrink-0 flex-col items-center justify-center transition-opacity duration-300 ${
          showUI ? 'opacity-100' : 'pointer-events-none opacity-0'
        }`}
      >
        <div
          class="flex items-center gap-2 rounded-full bg-white/20 p-2 shadow-lg backdrop-blur-md"
        >
          <!-- Previous button -->
          <Button
            variant="ghost"
            size="icon"
            disabled={!canShowPrevious}
            onclick={showPrevious}
            class={cn(
              'h-12 w-12 rounded-full text-white transition-all duration-200 hover:bg-white/20 disabled:opacity-30',
              !canShowPrevious && 'cursor-not-allowed',
            )}
            aria-label="Previous image"
          >
            <ArrowLeft />
          </Button>

          <!-- Image counter -->
          <div class="px-3 text-sm font-medium text-white/90">
            {currentIndex + 1} / {photoSource.length}
          </div>

          <!-- Favorite button -->
          <Button
            variant="ghost"
            size="icon"
            onclick={toggleFavorite}
            class={cn(
              'h-12 w-12 rounded-full transition-all duration-200',
              isFavourite
                ? 'bg-red-500/20 text-red-400 hover:bg-red-500/30'
                : 'text-white hover:bg-white/20 hover:text-red-400',
            )}
            aria-label={isFavourite
              ? 'Remove from favorites'
              : 'Add to favorites'}
            aria-pressed={isFavourite}
          >
            <Heart {isFavourite} />
          </Button>

          <!-- Next button -->
          <Button
            variant="ghost"
            size="icon"
            disabled={!canShowNext}
            onclick={showNext}
            class={cn(
              'h-12 w-12 rounded-full text-white transition-all duration-200 hover:bg-white/20 disabled:opacity-30',
              !canShowNext && 'cursor-not-allowed',
            )}
            aria-label="Next image"
          >
            <ArrowRight />
          </Button>

          <div class="mx-1 h-6 w-px bg-white/20"></div>

          <!-- Info toggle button -->
          <Button
            variant="ghost"
            size="icon"
            onclick={toggleInfo}
            class={cn(
              'h-12 w-12 rounded-full transition-all duration-200',
              showInfo
                ? 'bg-white/20 text-white'
                : 'text-white/80 hover:bg-white/20 hover:text-white',
            )}
            aria-label="Toggle info panel"
          >
            <Info size={18} />
          </Button>
        </div>
      </div>

      <div
        class={`transition-opacity duration-300 ${
          showUI ? 'opacity-100' : 'pointer-events-none opacity-0'
        }`}
      >
        <Filename name={fileName} />
      </div>
    </div>
  </div>
{/if}

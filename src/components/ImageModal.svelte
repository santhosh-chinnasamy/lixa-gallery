<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { cn } from '$lib/utils';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, scale } from 'svelte/transition';
  import { favorites, photos } from '../stores/galleryStore';
  import Filename from './Filename.svelte';
  import Heart from './icons/Heart.svelte';
  import ArrowRight from './icons/ArrowRight.svelte';
  import { page } from '$app/state';
  import ArrowLeft from './icons/ArrowLeft.svelte';
  import Close from './icons/Close.svelte';
  import Info from '@lucide/svelte/icons/info';
  import type { PhotoMetadata } from '../types/photo';
  export let selectedImage: PhotoMetadata | null;
  export let onClose: () => void;
  export let source = page.url.pathname === '/' ? 'all' : 'favorites';

  // For favorites, we need to find photos by path since favorites store paths
  $: photoSource =
    source === 'favorites'
      ? $photos.filter((photo) => $favorites.has(photo.path))
      : $photos;
  $: currentIndex = selectedImage
    ? photoSource.findIndex((photo) => photo.path === selectedImage!.path)
    : -1;
  $: canShowPrevious = currentIndex > 0;
  $: canShowNext = currentIndex < photoSource.length - 1;
  $: isFavourite = selectedImage ? $favorites.has(selectedImage.path) : false;

  // Image preloading and loading states
  let imageLoaded = false;
  let imageError = false;
  let preloadedImages = new Map<string, HTMLImageElement>();

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
      imageLoaded = false;
      imageError = false;
    }
  }

  function showNext() {
    if (canShowNext) {
      selectedImage = photoSource[currentIndex + 1];
      imageLoaded = false;
      imageError = false;
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

  // Reactive statement to preload when selectedImage changes
  $: if (selectedImage) {
    imageLoaded = false;
    imageError = false;
    preloadAdjacentImages();
  }

  let showInfo = false;
  let imageWidth = 0;
  let imageHeight = 0;

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

  $: fileName = selectedImage?.metadata.name;

  let showUI = true;
  let uiTimeoutRef: ReturnType<typeof setTimeout>;

  function resetUITimer() {
    showUI = true;
    clearTimeout(uiTimeoutRef);
    uiTimeoutRef = setTimeout(() => {
      showUI = false;
    }, 2000);
  }

  function handleMouseMove() {
    resetUITimer();
  }

  function handleMouseLeave() {
    clearTimeout(uiTimeoutRef);
    showUI = false;
  }

  // Reactive block to initialize timer when UI first shows
  $: {
    if (selectedImage) {
      resetUITimer();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if selectedImage}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4 backdrop-blur-3xl transition-colors duration-500"
    role="dialog"
    aria-modal="true"
    aria-label="Image preview"
    tabindex="-1"
    on:click={handleBackdropClick}
    on:keydown={handleKeydown}
    on:mousemove={handleMouseMove}
    on:mouseleave={handleMouseLeave}
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
        class="h-10 w-10 rounded-full bg-black/50 text-white hover:bg-black/70 focus:ring-2 focus:ring-white/50"
        onclick={onClose}
        aria-label="Close preview"
      >
        <Close />
      </Button>
    </div>

    <!-- Main modal content -->
    <div
      class="relative flex h-full max-h-[95vh] w-full max-w-7xl flex-col bg-white/10"
      in:scale={{ duration: 200, start: 0.95 }}
      out:scale={{ duration: 150, start: 0.95 }}
      on:click|stopPropagation
      on:keydown={handleKeydown}
      role="button"
      tabindex="0"
    >
      <!-- Image container -->
      <div
        class="relative flex flex-1 items-center justify-center overflow-hidden rounded-lg"
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

        <!-- Main image -->
        <img
          src={convertFileSrc(selectedImage.path)}
          alt={fileName}
          class={`max-h-full max-w-full rounded-lg object-contain shadow-2xl transition-opacity duration-300 ${
            imageLoaded ? 'opacity-100' : 'opacity-0'
          }`}
          style="filter: drop-shadow(0 25px 25px rgb(0 0 0 / 0.5))"
          on:load={handleImageLoad}
          on:error={handleImageError}
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

      <!-- Control bar -->
      <div
        class={`mt-4 flex shrink-0 flex-col items-center justify-center transition-opacity duration-300 ${
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
              'h-10 w-10 rounded-full text-white transition-all duration-200 hover:bg-white/20 disabled:opacity-30',
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
              'h-10 w-10 rounded-full transition-all duration-200',
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
              'h-10 w-10 rounded-full text-white transition-all duration-200 hover:bg-white/20 disabled:opacity-30',
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
              'h-10 w-10 rounded-full transition-all duration-200',
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

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
  import type { PhotoMetadata } from '../types/photo';

  export let selectedImage: PhotoMetadata | null;
  export let onClose: () => void;
  export let source = page.url.pathname === '/' ? 'all' : 'favorites';

  // For favorites, we need to find photos by path since favorites store paths
  $: photoSource = source === 'favorites' 
    ? $photos.filter(photo => $favorites.has(photo.path))
    : $photos;
  $: currentIndex = selectedImage ? photoSource.findIndex(photo => photo.path === selectedImage!.path) : -1;
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

  function handleImageLoad() {
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
    if (currentIndex + 2 < photoSource.length) indicesToPreload.push(currentIndex + 2);

    indicesToPreload.forEach(index => {
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

  $: fileName = selectedImage?.metadata.name;
</script>

<svelte:window on:keydown={handleKeydown} />

{#if selectedImage}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/95 p-4"
    role="dialog"
    aria-modal="true"
    aria-label="Image preview"
    tabindex="-1"
    on:click={handleBackdropClick}
    on:keydown={handleKeydown}
    in:fade={{ duration: 200 }}
    out:fade={{ duration: 150 }}
  >
    <!-- Close button -->
    <Button
      variant="ghost"
      size="icon"
      class="absolute right-4 top-4 z-10 h-10 w-10 rounded-full bg-black/50 text-white hover:bg-black/70 focus:ring-2 focus:ring-white/50"
      onclick={onClose}
      aria-label="Close preview"
    >
      <Close />
    </Button>

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
        class="flex flex-1 items-center justify-center overflow-hidden rounded-lg relative"
      >
        <!-- Loading spinner -->
        {#if !imageLoaded && !imageError}
          <div class="absolute inset-0 flex items-center justify-center bg-black/20 backdrop-blur-sm">
            <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-white"></div>
          </div>
        {/if}

        <!-- Error state -->
        {#if imageError}
          <div class="absolute inset-0 flex items-center justify-center bg-black/20 backdrop-blur-sm">
            <div class="text-white text-center">
              <div class="text-lg mb-2">Failed to load image</div>
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
      </div>

      <!-- Control bar -->
      <div class="mt-4 flex shrink-0 items-center justify-center">
        <div
          class="flex items-center gap-2 rounded-full bg-white/20 p-2 backdrop-blur-sm"
        >
          <!-- Previous button -->
          <Button
            variant="ghost"
            size="icon"
            disabled={!canShowPrevious}
            onclick={showPrevious}
            class={cn(
              'h-10 w-10 rounded-full text-white hover:bg-white/20 disabled:opacity-30 transition-all duration-200',
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
              'h-10 w-10 rounded-full text-white hover:bg-white/20 disabled:opacity-30 transition-all duration-200',
              !canShowNext && 'cursor-not-allowed',
            )}
            aria-label="Next image"
          >
            <ArrowRight />
          </Button>
        </div>
      </div>

      <Filename name={fileName} />
    </div>
  </div>
{/if}

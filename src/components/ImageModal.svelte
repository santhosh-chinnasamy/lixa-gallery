<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { cn } from '$lib/utils';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, scale } from 'svelte/transition';
  import { favorites, photos } from '../stores/galleryStore';

  export let selectedImage: string | null;
  export let onClose: () => void;
  export let source: 'all' | 'favorites' = 'all';

  $: photoSource = source === 'favorites' ? Array.from($favorites) : $photos;
  $: currentIndex = selectedImage ? photoSource.indexOf(selectedImage) : -1;
  $: canShowPrevious = currentIndex > 0;
  $: canShowNext = currentIndex < photoSource.length - 1;
  $: isFavorite = selectedImage ? $favorites.has(selectedImage) : false;

  const keyboardActions = {
    Escape: onClose,
    ArrowLeft: showPrevious,
    ArrowRight: showNext,
    h: toggleFavorite,
  } as const;

  function handleKeydown(event: KeyboardEvent) {
    const action = keyboardActions[event.key as keyof typeof keyboardActions];
    if (action) {
      event.preventDefault();
      action();
    }
  }

  function toggleFavorite() {
    if (selectedImage) favorites.toggle(selectedImage);
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
</script>

// ImageModal.svelte
<svelte:window on:keydown={handleKeydown} />

{#if selectedImage}
  <!-- Modal backdrop -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/95 p-4"
    role="dialog"
    aria-modal="true"
    aria-label="Image preview"
    on:click={handleBackdropClick}
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
      <svg
        class="h-5 w-5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M6 18L18 6M6 6l12 12"
        />
      </svg>
    </Button>

    <!-- Main modal content -->
    <div
      class="relative flex h-full max-h-[95vh] w-full max-w-7xl flex-col"
      in:scale={{ duration: 200, start: 0.95 }}
      out:scale={{ duration: 150, start: 0.95 }}
      on:click|stopPropagation
    >
      <!-- Image container -->
      <div
        class="flex flex-1 items-center justify-center overflow-hidden rounded-lg"
      >
        <img
          src={convertFileSrc(selectedImage)}
          alt="Selected image preview"
          class="max-h-full max-w-full rounded-lg object-contain shadow-2xl"
          style="filter: drop-shadow(0 25px 25px rgb(0 0 0 / 0.5))"
        />
      </div>

      <!-- Control bar -->
      <div class="mt-4 flex shrink-0 items-center justify-center">
        <div
          class="flex items-center gap-2 rounded-full bg-black/60 p-2 backdrop-blur-sm"
        >
          <!-- Previous button -->
          <Button
            variant="ghost"
            size="icon"
            disabled={!canShowPrevious}
            onclick={showPrevious}
            class={cn(
              'h-10 w-10 rounded-full text-white hover:bg-white/20 disabled:opacity-30',
              !canShowPrevious && 'cursor-not-allowed',
            )}
            aria-label="Previous image"
          >
            <svg
              class="h-5 w-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M15 19l-7-7 7-7"
              />
            </svg>
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
              'h-10 w-10 rounded-full transition-colors',
              isFavorite
                ? 'bg-red-500/20 text-red-400 hover:bg-red-500/30'
                : 'text-white hover:bg-white/20 hover:text-red-400',
            )}
            aria-label={isFavorite
              ? 'Remove from favorites'
              : 'Add to favorites'}
            aria-pressed={isFavorite}
          >
            <svg class="h-5 w-5 fill-current" viewBox="0 0 24 24">
              <path
                d={isFavorite
                  ? 'M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z'
                  : 'M16.5 3c-1.74 0-3.41.81-4.5 2.09C10.91 3.81 9.24 3 7.5 3 4.42 3 2 5.42 2 8.5c0 3.78 3.4 6.86 8.55 11.54L12 21.35l1.45-1.32C18.6 15.36 22 12.28 22 8.5 22 5.42 19.58 3 16.5 3zm-4.4 15.55l-.1.1-.1-.1C7.14 14.24 4 11.39 4 8.5 4 6.5 5.5 5 7.5 5c1.54 0 3.04.99 3.57 2.36h1.87C13.46 5.99 14.96 5 16.5 5c2 0 3.5 1.5 3.5 3.5 0 2.89-3.14 5.74-7.9 10.05z'}
              />
            </svg>
          </Button>

          <!-- Next button -->
          <Button
            variant="ghost"
            size="icon"
            disabled={!canShowNext}
            onclick={showNext}
            class={cn(
              'h-10 w-10 rounded-full text-white hover:bg-white/20 disabled:opacity-30',
              !canShowNext && 'cursor-not-allowed',
            )}
            aria-label="Next image"
          >
            <svg
              class="h-5 w-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 5l7 7-7 7"
              />
            </svg>
          </Button>
        </div>
      </div>

      <!-- Image info (filename) -->
      <div class="mt-2 text-center">
        <div class="text-sm text-white/70">
          {selectedImage?.split('/').pop() || 'Unknown'}
        </div>
      </div>
    </div>
  </div>
{/if}

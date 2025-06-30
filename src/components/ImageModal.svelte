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

  export let selectedImage: string | null;
  export let onClose: () => void;
  export let source = page.url.pathname === '/' ? 'all' : 'favorites';

  $: photoSource = source === 'favorites' ? Array.from($favorites) : $photos;
  $: currentIndex = selectedImage ? photoSource.indexOf(selectedImage) : -1;
  $: canShowPrevious = currentIndex > 0;
  $: canShowNext = currentIndex < photoSource.length - 1;
  $: isFavorite = selectedImage ? $favorites.has(selectedImage) : false;

  const keyboardActions = {
    Escape: onClose,
    ArrowLeft: showPrevious,
    ArrowRight: showNext,
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

<svelte:window on:keydown={handleKeydown} />

{#if selectedImage}
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
      <Close />
    </Button>

    <!-- Main modal content -->
    <div
      class="relative flex h-full max-h-[95vh] w-full max-w-7xl flex-col bg-white/10"
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
          loading="lazy"
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
              'h-10 w-10 rounded-full text-white hover:bg-white/20 disabled:opacity-30',
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
            <Heart {isFavorite} />
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
            <ArrowRight />
          </Button>
        </div>
      </div>

      <Filename {selectedImage} />
    </div>
  </div>
{/if}

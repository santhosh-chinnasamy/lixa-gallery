<script lang="ts">
  import { page } from '$app/state';
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import { cn } from '$lib/utils';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { favorites, photos } from '../stores/galleryStore';
  import type { KeyboardActions } from '../types/events';

  export let selectedImage: string | null;
  export let onClose: () => void;
  const sourceForPhotos =
    page.url.pathname === '/' ? $photos : Array.from($favorites);

  $: currentIndex = selectedImage ? sourceForPhotos.indexOf(selectedImage) : -1;
  $: canShowPrevious = currentIndex > 0;
  $: canShowNext = currentIndex < sourceForPhotos.length - 1;
  $: isFavorite = selectedImage ? $favorites.has(selectedImage) : false;

  const keyboardActions: KeyboardActions = {
    Escape: onClose,
    ArrowLeft: showPrevious,
    ArrowRight: showNext,
    h: toggleFavorite,
  };

  async function toggleFavorite() {
    if (!selectedImage) return;
    try {
      favorites.toggle(selectedImage);
    } catch (error) {
      console.error(
        `Error ${isFavorite ? 'removing' : 'adding'} favorite:`,
        error,
      );
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    const action = keyboardActions[event.key];
    if (action) {
      event.preventDefault();
      action();
    }
  }

  function showPrevious() {
    if (canShowPrevious && selectedImage) {
      selectedImage = sourceForPhotos[currentIndex - 1];
    }
  }

  function showNext() {
    if (canShowNext && selectedImage) {
      selectedImage = sourceForPhotos[currentIndex + 1];
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />
{#if selectedImage}
  <div
    class="fixed inset-0 z-50 grid place-items-center bg-black/90"
    on:click={onClose}
    role="dialog"
    aria-modal="true"
    aria-label="Image preview"
  >
    <Button
      variant="outline"
      size="icon"
      title="Close Preview"
      onclick={onClose}
      class="absolute right-4 top-4"
    >
      <span aria-hidden="true">×</span>
    </Button>
    <Card.Root
      class="relative max-h-[90vh] max-w-[90vw] rounded-lg bg-background p-4"
      onclick={(event: Event) => event.stopPropagation()}
    >
      <Card.Content>
        <img
          src={convertFileSrc(selectedImage)}
          alt="Selected image preview"
          class="max-h-[calc(90vh-6rem)] max-w-full rounded object-contain"
        />
      </Card.Content>
      <div
        class="mt-4 flex h-16 items-center justify-center space-x-4 rounded-md bg-gray-500 px-4 py-2 shadow-md"
        role="toolbar"
        aria-label="Image controls"
      >
        <Button
          variant="outline"
          size="icon"
          disabled={!canShowPrevious}
          title="Previous image"
          onclick={showPrevious}
          class={cn(!canShowPrevious && 'cursor-not-allowed opacity-50')}
        >
          <span aria-hidden="true">‹</span>
        </Button>

        <Button
          variant={isFavorite ? 'destructive' : 'outline'}
          size="icon"
          onclick={toggleFavorite}
          title={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
          aria-pressed={isFavorite}
          class="transition-colors hover:bg-red-500 hover:text-white"
        >
          <span aria-hidden="true">♥</span>
        </Button>

        <Button
          variant="outline"
          size="icon"
          disabled={!canShowNext}
          title="Next image"
          onclick={showNext}
          class={cn(!canShowNext && 'cursor-not-allowed opacity-50')}
        >
          <span aria-hidden="true">›</span>
        </Button>
      </div>
    </Card.Root>
  </div>
{/if}

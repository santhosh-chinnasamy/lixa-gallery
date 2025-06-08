<script lang="ts">
  import { page } from '$app/state';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { favorites, photos } from '../stores/galleryStore';
  import type { KeyboardActions } from '../types/events';
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';

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
    class="modal-overlay"
    on:click={onClose}
    role="dialog"
    aria-modal="true"
    aria-label="Image preview"
  >
    <Card.Root onclick={(event: Event) => event.stopPropagation()}>
      <div class="modal-content">
        <Card.Content>
          <img
            src={convertFileSrc(selectedImage)}
            alt="Selected image preview"
            class="modal-image"
          />
        </Card.Content>
      </div>
      <div
        class="flex h-[4rem] justify-center space-x-4"
        role="toolbar"
        aria-label="Image controls"
      >
        <Button
          variant="outline"
          size="icon"
          disabled={!canShowPrevious}
          title="Previous image"
          onclick={showPrevious}
        >
          <span aria-hidden="true">‹</span>
        </Button>

        <Button
          variant={isFavorite ? 'destructive' : 'outline'}
          size="icon"
          onclick={toggleFavorite}
          title={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
          aria-pressed={isFavorite}
          class="hover:bg-red-500 hover:text-white"
        >
          <span aria-hidden="true">♥</span>
        </Button>
        <Button
          variant="outline"
          size="icon"
          disabled={!canShowNext}
          title="Next image"
          onclick={showNext}
        >
          <span aria-hidden="true">›</span>
        </Button>
        <Button
          class="close-button"
          variant="outline"
          size="icon"
          title="Close Preview"
          onclick={onClose}
        >
          <span aria-hidden="true">×</span>
        </Button>
      </div>
    </Card.Root>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.9);
    display: grid;
    place-items: center;
    z-index: var(--z-modal);
  }

  .modal-content {
    position: relative;
    max-width: min(90vw, 1200px);
    max-height: 90vh;
    border-radius: 0.5rem;
    background-color: var(--color-background);
    padding: 1rem;
  }

  .modal-image {
    max-width: 100%;
    max-height: calc(90vh - 6rem);
    object-fit: contain;
    border-radius: 0.25rem;
  }
</style>

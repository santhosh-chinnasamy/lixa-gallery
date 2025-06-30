<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import { cn } from '$lib/utils';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, scale } from 'svelte/transition';
  import { favorites, photos } from '../stores/galleryStore';

  export let selectedImage: string | null;
  export let onClose: () => void;
  export let source: 'all' | 'favorites' = 'all';

  const photoSource = source === 'favorites' ? Array.from($favorites) : $photos;

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
</script>

<svelte:window on:keydown={handleKeydown} />

{#if selectedImage}
  <div
    class="fixed inset-0 z-50 grid place-items-center bg-black/90"
    role="dialog"
    aria-modal="true"
    aria-label="Image preview"
    on:click={onClose}
    in:scale={{ duration: 200 }}
    out:fade={{ duration: 200 }}
  >
    <Button
      variant="outline"
      size="icon"
      title="Close preview"
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
          class="max-h-[calc(80vh-6rem)] max-w-full rounded object-contain"
        />
      </Card.Content>

      <div
        class="mt-4 flex h-16 items-center justify-center space-x-4 rounded-md bg-gray-500/80 px-4 py-2 shadow-md"
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
          ‹
        </Button>

        <Button
          variant={isFavorite ? 'destructive' : 'outline'}
          size="icon"
          onclick={toggleFavorite}
          title={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
          aria-pressed={isFavorite}
          class="transition-colors hover:bg-red-500 hover:text-white"
        >
          ♥
        </Button>

        <Button
          variant="outline"
          size="icon"
          disabled={!canShowNext}
          title="Next image"
          onclick={showNext}
          class={cn(!canShowNext && 'cursor-not-allowed opacity-50')}
        >
          ›
        </Button>
      </div>
    </Card.Root>
  </div>
{/if}

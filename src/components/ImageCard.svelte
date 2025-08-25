<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { favorites } from '../stores/galleryStore';
  import Filename from './Filename.svelte';
  import Heart from './icons/Heart.svelte';
  import type { PhotoMetadata } from '../types/photo';
  import { onMount } from 'svelte';

  export let photo: PhotoMetadata;
  export let tabindex: number | undefined = 0;
  export let handleImageClick: (photo: PhotoMetadata) => void;
  export let trackImageElement: ((path: string, element: HTMLImageElement) => void) | undefined = undefined;
  export let isVisible = true; // For intersection observer
  export let globalIndex: number | undefined = undefined;
  
  // Use globalIndex for accessibility or other purposes if needed
  $: ariaLabel = globalIndex !== undefined ? `Image ${globalIndex + 1}` : fileName;

  $: isFavourite = $favorites.has(photo.path);
  $: fileName = photo.metadata.name;
  $: thumbnailSrc = photo.thumbnail_path;

  let imageElement: HTMLImageElement;
  let imageLoaded = false;
  let imageError = false;

  const handleClick = () => handleImageClick(photo);

  const toggleFavorite = (event: Event) => {
    event.stopPropagation();
    favorites.toggle(photo.path);
  };

  const keyboardActions = {
    Enter: handleClick,
    l: toggleFavorite,
  } as const;

  function handleKeyPress(event: KeyboardEvent) {
    const action = keyboardActions[event.key as keyof typeof keyboardActions];
    if (action) {
      event.preventDefault();
      action(event);
    }
  }

  function handleImageLoad() {
    imageLoaded = true;
    imageError = false;
    if (trackImageElement && imageElement) {
      trackImageElement(photo.path, imageElement);
    }
  }

  function handleImageError() {
    imageError = true;
    imageLoaded = false;
  }
</script>

<Card.Root
  class={`group relative cursor-pointer transition-all duration-200 ease-in-out hover:scale-[1.02] hover:shadow-lg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2`}
  {tabindex}
  onclick={handleClick}
  onkeypress={handleKeyPress}
>
  <Card.Content class="relative overflow-hidden rounded-lg p-0">
    <!-- Image container with consistent aspect ratio -->
    <div class="aspect-[1/1] h-auto w-full overflow-hidden bg-gray-100">
      <!-- Loading placeholder -->
      {#if !imageLoaded && !imageError}
        <div class="flex h-full w-full items-center justify-center bg-gray-200 animate-pulse">
          <div class="text-gray-400 text-sm">Loading...</div>
        </div>
      {/if}

      <!-- Error placeholder -->
      {#if imageError}
        <div class="flex h-full w-full items-center justify-center bg-gray-100">
          <div class="text-gray-500 text-xs text-center p-2">
            <div>Failed to load</div>
            <div class="text-gray-400">{fileName}</div>
          </div>
        </div>
      {/if}

      <!-- Actual image -->
      <img
        bind:this={imageElement}
        src={convertFileSrc(thumbnailSrc)}
        alt={ariaLabel}
        loading={isVisible ? "eager" : "lazy"}
        class={`h-full w-full object-cover transition-all duration-300 group-hover:scale-105 ${
          imageLoaded ? 'opacity-100' : 'opacity-0'
        }`}
        onload={handleImageLoad}
        onerror={handleImageError}
      />
    </div>

    <button
      class={`absolute right-2 top-2 rounded-full p-2 shadow-md transition-all duration-200 ${
        isFavourite
          ? 'bg-red-50 text-red-500 hover:bg-red-100'
          : 'bg-white/80 text-gray-400 opacity-0 hover:bg-white hover:text-red-500 group-hover:opacity-100'
      } backdrop-blur-sm`}
      onclick={toggleFavorite}
      aria-label={isFavourite ? 'Remove from favorites' : 'Add to favorites'}
    >
      <Heart {isFavourite} />
    </button>

    <div
      class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/60 to-transparent p-3 opacity-0 transition-opacity duration-200 group-hover:opacity-100"
    >
      <Filename name={fileName} />
    </div>
  </Card.Content>
</Card.Root>

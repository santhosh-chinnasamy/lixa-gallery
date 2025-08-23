<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { favorites } from '../stores/galleryStore';
  import Filename from './Filename.svelte';
  import Heart from './icons/Heart.svelte';
  import type { PhotoMetadata } from '../types/photo';

  export let photo: PhotoMetadata;
  export let tabindex: number | undefined = 0;
  export let handleImageClick: (photo: PhotoMetadata) => void;

  $: isFavourite = $favorites.has(photo.path);
  $: fileName = photo.metadata.name;
  $: thumbnailSrc = photo.thumbnail_path;

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
      <img
        src={convertFileSrc(thumbnailSrc)}
        alt={fileName}
        loading="lazy"
        class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
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

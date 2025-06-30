<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { favorites } from '../stores/galleryStore';
  import Filename from './Filename.svelte';
  import Heart from './icons/Heart.svelte';

  export let path: string;
  export let tabindex: number | undefined = 0;
  export let handleImageClick: (path: string) => void;

  const handleClick = () => handleImageClick(path);

  const toggleFavorite = (event: Event) => {
    event.stopPropagation();
    favorites.toggle(path);
  };

  const handleKeyPress = (event: KeyboardEvent) => {
    if (event.key === 'Enter') handleClick();
    if (event.key.toLowerCase() === 'l') toggleFavorite(event);
  };

  $: isFavorite = $favorites.has(path);
</script>

<Card.Root
  class={`group relative cursor-pointer transition-all duration-200 ease-in-out hover:scale-[1.02] hover:shadow-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 ${
    isFavorite
      ? 'ring-2 ring-red-400 ring-offset-2'
      : 'hover:ring-1 hover:ring-gray-200'
  }`}
  {tabindex}
  onclick={handleClick}
  onkeypress={handleKeyPress}
>
  <Card.Content class="relative overflow-hidden rounded-lg p-0">
    <!-- Image container with consistent aspect ratio -->
    <div class="aspect-[3/5] w-full overflow-hidden bg-gray-100">
      <img
        src={convertFileSrc(path)}
        alt="Gallery image"
        loading="lazy"
        class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
      />
    </div>

    <button
      class={`absolute right-2 top-2 rounded-full p-2 shadow-md transition-all duration-200 ${
        isFavorite
          ? 'bg-red-50 text-red-500 hover:bg-red-100'
          : 'bg-white/80 text-gray-400 opacity-0 hover:bg-white hover:text-red-500 group-hover:opacity-100'
      } backdrop-blur-sm`}
      onclick={toggleFavorite}
      aria-label={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
    >
      <Heart {isFavorite} />
    </button>

    <div
      class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/60 to-transparent p-3 opacity-0 transition-opacity duration-200 group-hover:opacity-100"
    >
      <Filename selectedImage={path} />
    </div>
  </Card.Content>
</Card.Root>

<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { favorites } from '../stores/galleryStore';

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
    <div class="aspect-[4/5] w-full overflow-hidden bg-gray-100">
      <img
        src={convertFileSrc(path)}
        alt="Gallery image"
        loading="lazy"
        class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
      />
    </div>

    <!-- Favorite button overlay -->
    <button
      class={`absolute right-2 top-2 rounded-full p-2 shadow-md transition-all duration-200 ${
        isFavorite
          ? 'bg-red-50 text-red-500 hover:bg-red-100'
          : 'bg-white/80 text-gray-400 opacity-0 hover:bg-white hover:text-red-500 group-hover:opacity-100'
      } backdrop-blur-sm`}
      onclick={toggleFavorite}
      aria-label={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
    >
      <svg class="h-4 w-4 fill-current" viewBox="0 0 24 24">
        <path
          d={isFavorite
            ? 'M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z'
            : 'M16.5 3c-1.74 0-3.41.81-4.5 2.09C10.91 3.81 9.24 3 7.5 3 4.42 3 2 5.42 2 8.5c0 3.78 3.4 6.86 8.55 11.54L12 21.35l1.45-1.32C18.6 15.36 22 12.28 22 8.5 22 5.42 19.58 3 16.5 3zm-4.4 15.55l-.1.1-.1-.1C7.14 14.24 4 11.39 4 8.5 4 6.5 5.5 5 7.5 5c1.54 0 3.04.99 3.57 2.36h1.87C13.46 5.99 14.96 5 16.5 5c2 0 3.5 1.5 3.5 3.5 0 2.89-3.14 5.74-7.9 10.05z'}
        />
      </svg>
    </button>

    <!-- Image info overlay (optional) -->
    <div
      class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/60 to-transparent p-3 opacity-0 transition-opacity duration-200 group-hover:opacity-100"
    >
      <div class="text-xs text-white/90">
        {path.split('/').pop()?.split('.')[0] || 'Image'}
      </div>
    </div>
  </Card.Content>
</Card.Root>

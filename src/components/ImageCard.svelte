<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { favorites } from '../stores/galleryStore';

  export let path: string;
  export let tabindex: number | undefined = 0;
  export let handleImageClick: (path: string) => void;

  const handleClick = () => handleImageClick(path);

  const toggleFavorite = () => favorites.toggle(path);

  const handleKeyPress = (event: KeyboardEvent) => {
    if (event.key === 'Enter') handleClick();
    if (event.key.toLowerCase() === 'l') toggleFavorite();
  };

  const isFavorite = $favorites.has(path);
</script>

<Card.Root
  class={`relative cursor-pointer border transition-all ${
    isFavorite ? 'border-red-500' : 'border-transparent'
  } rounded-xl hover:shadow-lg focus:outline-none focus:ring-2 focus:ring-blue-500`}
  {tabindex}
  onclick={handleClick}
  onkeypress={handleKeyPress}
>
  <Card.Content class="aspect-[4/5] w-full">
    <img
      src={convertFileSrc(path)}
      alt="Image"
      loading="lazy"
      class="h-full w-full rounded-xl object-cover"
    />
  </Card.Content>

  {#if isFavorite}
    <div class="absolute right-2 top-2 rounded-full bg-white p-1 shadow">
      ❤️
    </div>
  {/if}
</Card.Root>

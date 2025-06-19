<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { favorites } from '../stores/galleryStore';

  let { path, tabindex, handleImageClick } = $props();

  const handleClick = () => {
    handleImageClick(path);
  };

  const toggleFavorite = () => favorites.toggle(path);

  const handleKeyPress = (event: KeyboardEvent) => {
    switch (event.key) {
      case 'Enter':
        handleImageClick(path);
        break;
      case 'l':
        toggleFavorite();
        break;
    }
  };
</script>

<Card.Root
  class={`h-96 w-96 ${!!$favorites.has(path) ? 'border-[0.2rem] border-red-500' : ''}`}
  {tabindex}
  onclick={handleClick}
  onkeypress={handleKeyPress}
>
  <Card.Content>
    <img
      src={convertFileSrc(path)}
      alt={path}
      class="h-80 w-80 object-cover"
      loading="lazy"
    />
  </Card.Content>
</Card.Root>

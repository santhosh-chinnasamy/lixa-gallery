<script lang="ts">
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
      case 'h':
        toggleFavorite();
        break;
    }
  };
</script>

<main
  class="image-card"
  {tabindex}
  onclick={handleClick}
  onkeypress={handleKeyPress}
>
  <div class="image-container">
    <img src={convertFileSrc(path)} alt="" class="image" loading="lazy" />
  </div>
</main>

<style>
  .image-card {
    display: flex;
    justify-content: center;
    align-items: center;
    margin: auto;
    border-radius: 0.25rem;
    background-color: var(--color-gray-300);
    color: #fff;
  }

  .image-container {
    width: 300px;
    height: 200px;
    display: flex;
    justify-content: center;
    align-items: center;
    overflow: hidden;
  }

  .image-container .image {
    max-width: 100%;
    height: auto;
    object-fit: cover;
  }

  .image:hover {
    transform: scale(1.1);
    transition: transform 0.2s ease-in-out;
    cursor: pointer;
  }

  @media (max-width: 800px) {
    .image-container {
      width: 200px;
      height: 200px;
    }
  }
</style>

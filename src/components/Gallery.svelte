<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';

  let { photos } = $props();
  let selectedImage: string | null = $state(null);

  function handleImageClick(path: string) {
    selectedImage = path;
  }

  function handleCloseModal() {
    selectedImage = null;
  }
</script>

<main class="gallery">
  {#if photos.length === 0}
    <p class="no-photos">No photos found. Please select a folder with photos</p>
  {:else}
    <div class="images">
      {#each photos as path, index}
        <ImageCard {path} tabindex={index + 1} {handleImageClick} />
      {/each}
    </div>
  {/if}
</main>

<ImageModal {selectedImage} onClose={handleCloseModal} />

<style>
  .no-photos {
    display: flex;
    justify-content: center;
    align-items: center;
    text-align: center;
    font-size: 1.5rem;
    font-weight: bold;
    color: var(--color-gray-600);
    height: calc(100vh - 100px);
  }

  .gallery {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 1rem;
    overflow: scroll;
    height: calc(100vh - 100px);
  }

  .images {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
  }

  @media (max-width: 800px) {
    .images {
      grid-template-columns: repeat(3, 1fr);
    }
  }

  @media (min-width: 1024px) {
    .images {
      grid-template-columns: repeat(4, 1fr);
    }
  }
</style>

<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  import type { PhotoMetadata } from '../types/photo';
  
  export let photos: PhotoMetadata[];

  let selectedImage: PhotoMetadata | null = null;

  const handleImageClick = (photo: PhotoMetadata) => {
    selectedImage = photo;
  };

  const handleCloseModal = () => {
    selectedImage = null;
  };
</script>

<main class="flex-1 overflow-auto">
  <div class="h-full p-3 sm:p-4 md:p-6">
    {#if photos.length === 0}
      <div class="flex h-full items-center justify-center">
        <div class="text-center text-gray-500">
          <div class="mb-2 text-lg font-medium">No photos found</div>
          <div class="text-sm">Choose a folder to load images</div>
        </div>
      </div>
    {:else}
      <!-- Responsive grid that adapts to container width -->
      <div class="auto-fill-grid grid gap-3 sm:gap-4 md:gap-6">
        {#each photos as photo, index}
          <ImageCard {photo} tabindex={index + 1} {handleImageClick} />
        {/each}
      </div>
    {/if}
  </div>
</main>

<ImageModal {selectedImage} onClose={handleCloseModal} />

<style>
  .auto-fill-grid {
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  }

  /* Responsive breakpoints for better control */
  @media (min-width: 640px) {
    .auto-fill-grid {
      grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    }
  }

/*   @media (min-width: 768px) {
    .auto-fill-grid {
      grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    }
  } */

  @media (min-width: 1024px) {
    .auto-fill-grid {
      grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    }
  }

  @media (min-width: 1280px) {
    .auto-fill-grid {
      grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    }
  }
</style>

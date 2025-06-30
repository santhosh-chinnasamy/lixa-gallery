<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  export let photos: string[];

  let selectedImage: string | null = null;

  const handleImageClick = (path: string) => {
    selectedImage = path;
  };

  const handleCloseModal = () => {
    selectedImage = null;
  };
</script>

<main
  class="flex h-[calc(100vh-4rem)] flex-col items-center overflow-auto p-4 pb-20"
>
  {#if photos.length === 0}
    <div class="text-center text-gray-500">
      No photos found. Choose a folder to load images.
    </div>
  {:else}
    <div class="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-4">
      {#each photos as path, index}
        <ImageCard {path} tabindex={index + 1} {handleImageClick} />
      {/each}
    </div>
  {/if}
</main>

<ImageModal {selectedImage} onClose={handleCloseModal} />

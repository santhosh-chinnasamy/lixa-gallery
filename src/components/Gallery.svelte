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

<main
  class="flex h-[calc(100vh-4rem)] flex-col items-center overflow-auto p-4 pb-20"
>
  <div
    class="xs:grid-cols-1 grid grid-cols-4 gap-4 sm:grid-cols-1 md:grid-cols-3 lg:grid-cols-4"
  >
    {#each photos as path, index}
      <ImageCard {path} tabindex={index + 1} {handleImageClick} />
    {/each}
  </div>
</main>

<ImageModal {selectedImage} onClose={handleCloseModal} />

<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  import type { PhotoMetadata } from '../types/photo';
  import { onMount } from 'svelte';

  let { photos = [] }: { photos: PhotoMetadata[] } = $props();

  let selectedImage = $state<PhotoMetadata | null>(null);
  let galleryContainer = $state<HTMLElement | null>(null);
  let visibleImages = $state(new Set<number>());

  const handleImageClick = (photo: PhotoMetadata) => {
    selectedImage = photo;
  };

  const handleCloseModal = () => {
    selectedImage = null;
  };

  // Re-run observer logic whenever photos change or container is ready
  $effect(() => {
    if (!galleryContainer || photos.length === 0) return;

    // Clear visible images when photos change to avoid showing old indexes
    visibleImages = new Set();

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          const indexAttr = entry.target.getAttribute('data-index');
          if (indexAttr === null) return;
          const index = parseInt(indexAttr);

          if (entry.isIntersecting) {
            visibleImages.add(index);
            // Preload next few images
            for (let i = index; i < Math.min(index + 8, photos.length); i++) {
              visibleImages.add(i);
            }
          } else {
            visibleImages.delete(index);
          }
        });
      },
      {
        root: galleryContainer,
        rootMargin: '200px', // Increased margin for smoother loading
        threshold: 0.1,
      },
    );

    // Observe all image cards - using a tiny delay to ensure DOM has updated
    const timeout = setTimeout(() => {
      if (!galleryContainer) return;
      const imageCards = galleryContainer.querySelectorAll('[data-index]');
      imageCards.forEach((card) => observer.observe(card));
    }, 0);

    return () => {
      clearTimeout(timeout);
      observer.disconnect();
    };
  });
</script>

<main
  class="custom-scrollbar flex-1 overflow-auto scroll-smooth"
  bind:this={galleryContainer}
>
  <div class="h-full p-2 sm:p-3 md:p-4 lg:p-6 xl:p-8">
    {#if photos.length === 0}
      <div class="flex h-full min-h-[400px] items-center justify-center">
        <div
          class="animate-in fade-in slide-in-from-bottom-4 text-center text-muted-foreground duration-700"
        >
          <div class="mb-2 text-lg font-medium">No photos in this folder</div>
          <p class="text-sm">
            This directory doesn't appear to contain any supported image files.
          </p>
        </div>
      </div>
    {:else}
      <div
        class="grid w-full auto-rows-fr grid-cols-2 justify-items-center gap-2 xs:grid-cols-3 xs:gap-3 sm:grid-cols-4 sm:gap-4 md:grid-cols-5 md:gap-4 lg:grid-cols-6 lg:gap-5 xl:grid-cols-7 xl:gap-6 2xl:grid-cols-8 2xl:gap-6"
      >
        {#each photos as photo, index (photo.path)}
          <div
            data-index={index}
            class="w-full max-w-[180px] xs:max-w-[160px] sm:max-w-[170px] md:max-w-[180px] lg:max-w-[190px] xl:max-w-[200px] 2xl:max-w-[220px]"
          >
            <ImageCard
              {photo}
              tabindex={index + 1}
              {handleImageClick}
              isVisible={visibleImages.has(index) || index < 12}
            />
          </div>
        {/each}
      </div>
    {/if}
  </div>
</main>

<ImageModal {selectedImage} onClose={handleCloseModal} />

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 6px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(0, 0, 0, 0.1);
    border-radius: 10px;
  }
  :global(.dark) .custom-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
  }
</style>

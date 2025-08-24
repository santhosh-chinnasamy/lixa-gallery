<script lang="ts">
  import ImageCard from './ImageCard.svelte';
  import ImageModal from './ImageModal.svelte';
  import type { PhotoMetadata } from '../types/photo';
  import { onMount } from 'svelte';
  
  export let photos: PhotoMetadata[];

  let selectedImage: PhotoMetadata | null = null;
  let galleryContainer: HTMLElement;
  let visibleImages = new Set<number>();

  const handleImageClick = (photo: PhotoMetadata) => {
    selectedImage = photo;
  };

  const handleCloseModal = () => {
    selectedImage = null;
  };

  onMount(() => {
    if (!galleryContainer) return;

    // Intersection Observer for lazy loading only
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          const index = parseInt(entry.target.getAttribute('data-index') || '0');
          if (entry.isIntersecting) {
            visibleImages.add(index);
            // Preload next few images
            for (let i = index; i < Math.min(index + 8, photos.length); i++) {
              visibleImages.add(i);
            }
            visibleImages = visibleImages; // Trigger reactivity
          } else {
            visibleImages.delete(index);
            visibleImages = visibleImages; // Trigger reactivity
          }
        });
      },
      {
        root: galleryContainer,
        rootMargin: '100px',
        threshold: 0.1
      }
    );

    // Observe all image cards
    const imageCards = galleryContainer.querySelectorAll('[data-index]');
    imageCards.forEach((card) => observer.observe(card));

    return () => {
      observer.disconnect();
    };
  });
</script>

<main class="flex-1 overflow-auto scroll-smooth" bind:this={galleryContainer}>
  <div class="h-full p-2 sm:p-3 md:p-4 lg:p-6 xl:p-8">
    {#if photos.length === 0}
      <div class="flex h-full items-center justify-center">
        <div class="text-center text-gray-500">
          <div class="mb-2 text-lg font-medium">No photos found</div>
          <div class="text-sm">Choose a folder to load images</div>
        </div>
      </div>
    {:else}
      <!-- Optimized grid for laptop/desktop with mobile support -->
      <div class="
        grid w-full
        gap-2 grid-cols-2
        xs:gap-3 xs:grid-cols-3
        sm:gap-4 sm:grid-cols-4
        md:gap-4 md:grid-cols-5
        lg:gap-5 lg:grid-cols-6
        xl:gap-6 xl:grid-cols-7
        2xl:gap-6 2xl:grid-cols-8
        auto-rows-fr
        justify-items-center
      ">
        {#each photos as photo, index}
          <div data-index={index} class="
            w-full max-w-[180px]
            xs:max-w-[160px]
            sm:max-w-[170px]
            md:max-w-[180px]
            lg:max-w-[190px]
            xl:max-w-[200px]
            2xl:max-w-[220px]
          ">
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

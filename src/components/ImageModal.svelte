<script lang="ts">
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { photos } from "../stores/galleryStore";

  export let selectedImage: string | null;
  export let onClose: () => void;

  $: currentIndex = selectedImage ? $photos.indexOf(selectedImage) : -1;
  let isFavorite = false;

  async function toggleFavorite() {
    if (!selectedImage) return;

    try {
      if (isFavorite) {
        await invoke("remove_favourite", { path: selectedImage });
        isFavorite = false;
      } else {
        await invoke("add_favourite", { path: selectedImage });
        isFavorite = true;
      }
    } catch (error) {
      console.error("Error toggling favorite:", error);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      onClose();
    } else if (event.key === "ArrowLeft") {
      showPrevious();
    } else if (event.key === "ArrowRight") {
      showNext();
    } else if (event.key === "l") {
      toggleFavorite();
    }
  }

  function showPrevious() {
    if (currentIndex > 0) {
      currentIndex--;
      selectedImage = $photos[currentIndex];
    }
  }

  function showNext() {
    if (currentIndex < $photos.length - 1) {
      currentIndex++;
      selectedImage = $photos[currentIndex];
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if selectedImage}
  <div class="modal-overlay" on:click={onClose}>
    <button class="close-button" on:click={onClose} aria-label="Close modal">
      ×
    </button>
    <button
      class="nav-button prev"
      on:click={showPrevious}
      disabled={currentIndex === 0}
      aria-label="Previous image"
    >
      ‹
    </button>
    <div class="modal-content" on:click|stopPropagation>
      <img src={convertFileSrc(selectedImage)} alt="" class="modal-image" />
      <div class="toolbar">
        <button
          class="favorite-button"
          on:click={toggleFavorite}
          class:active={isFavorite}
          aria-label={isFavorite ? "Remove from favorites" : "Add to favorites"}
        >
          ♥
        </button>
      </div>
    </div>

    <button
      class="nav-button next"
      on:click={showNext}
      disabled={currentIndex === $photos.length - 1}
      aria-label="Next image"
    >
      ›
    </button>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background-color: rgba(0, 0, 0, 0.8);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: var(--z-modal);
  }

  .modal-overlay button {
    z-index: 1;
  }

  .modal-content {
    position: relative;
    max-width: 90vw;
    max-height: 90vh;
    display: flex;
    align-items: center;
    border-radius: 0.5rem;
    box-shadow: 0 0 10px rgba(0, 0, 0, 0.5);
    background-color: aliceblue;
    padding: 20px;
    padding-top: 50px;
  }

  .modal-image {
    max-width: 100%;
    max-height: 80vh;
    object-fit: contain;
  }

  .close-button {
    position: absolute;
    background-color: var(--color-gray-700);
    border: none;
    color: white;
    font-size: 2rem;
    cursor: pointer;
    height: 4rem;
    width: 4rem;
    border-radius: 50%;
    display: flex;
    justify-content: center;
    align-items: center;
    transition: background-color 0.2s;
    top: 1rem;
    right: 1rem;
  }

  .nav-button {
    background: var(--color-gray-700);
    border: none;
    color: white;
    font-size: 2rem;
    cursor: pointer;
    height: 4rem;
    width: 4rem;
    border-radius: 50%;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .nav-button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.5);
  }

  .nav-button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .prev {
    position: absolute;
    left: 1rem;
  }

  .next {
    position: absolute;
    right: 1rem;
  }

  .toolbar {
    display: flex;
    justify-content: center;
    align-items: center;
    background-color: var(--color-gray-400);
    height: 4rem;
    position: absolute;
    margin: auto;
    bottom: 0;
    right: 0;
    left: 0;
    z-index: 1;
  }

  .favorite-button {
    position: absolute;
    background: none;
    border: none;
    color: white;
    font-size: 2rem;
    cursor: pointer;
    padding: 0.5rem;
    transition: color 0.2s;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .favorite-button:hover {
    color: var(--color-warning);
  }

  .favorite-button.active {
    color: var(--color-error);
  }
</style>

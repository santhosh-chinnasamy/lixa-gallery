<script lang="ts">
  import { page } from '$app/state';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { favorites, photos } from '../stores/galleryStore';
  import type { KeyboardActions } from '../types/events';

  export let selectedImage: string | null;
  export let onClose: () => void;
  const sourceForPhotos =
    page.url.pathname === '/' ? $photos : Array.from($favorites);

  $: currentIndex = selectedImage ? sourceForPhotos.indexOf(selectedImage) : -1;
  $: canShowPrevious = currentIndex > 0;
  $: canShowNext = currentIndex < sourceForPhotos.length - 1;
  $: isFavorite = selectedImage ? $favorites.has(selectedImage) : false;

  const keyboardActions: KeyboardActions = {
    Escape: onClose,
    ArrowLeft: showPrevious,
    ArrowRight: showNext,
    h: toggleFavorite,
  };

  async function toggleFavorite() {
    if (!selectedImage) return;
    try {
      favorites.toggle(selectedImage);
    } catch (error) {
      console.error(
        `Error ${isFavorite ? 'removing' : 'adding'} favorite:`,
        error,
      );
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    const action = keyboardActions[event.key];
    if (action) {
      event.preventDefault();
      action();
    }
  }

  function showPrevious() {
    if (canShowPrevious && selectedImage) {
      selectedImage = sourceForPhotos[currentIndex - 1];
    }
  }

  function showNext() {
    if (canShowNext && selectedImage) {
      selectedImage = sourceForPhotos[currentIndex + 1];
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if selectedImage}
  <div
    class="modal-overlay"
    on:click={onClose}
    role="dialog"
    aria-modal="true"
    aria-label="Image preview"
  >
    <div class="modal-controls">
      <button
        class="control-button close-button"
        on:click={onClose}
        aria-label="Close modal"
      >
        <span aria-hidden="true">×</span>
      </button>

      <button
        class="control-button nav-button prev"
        on:click={showPrevious}
        disabled={!canShowPrevious}
        aria-label="Previous image"
      >
        <span aria-hidden="true">‹</span>
      </button>

      <button
        class="control-button nav-button next"
        on:click={showNext}
        disabled={!canShowNext}
        aria-label="Next image"
      >
        <span aria-hidden="true">›</span>
      </button>
    </div>

    <div class="modal-content" on:click|stopPropagation>
      <img
        src={convertFileSrc(selectedImage)}
        alt="Selected image preview"
        class="modal-image"
      />

      <div class="toolbar" role="toolbar" aria-label="Image controls">
        <button
          class="control-button favorite-button"
          on:click={toggleFavorite}
          class:active={isFavorite}
          aria-label={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
          aria-pressed={isFavorite}
        >
          <span aria-hidden="true">♥</span>
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    background-color: rgba(0, 0, 0, 0.9);
    display: grid;
    place-items: center;
    z-index: var(--z-modal);
  }

  .modal-controls {
    position: fixed;
    inset: 0;
    pointer-events: none;
    z-index: 2;
  }

  .modal-content {
    position: relative;
    max-width: min(90vw, 1200px);
    max-height: 90vh;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 0.5rem;
    background-color: var(--color-background);
    padding: 1rem;
  }

  .modal-image {
    max-width: 100%;
    max-height: calc(90vh - 6rem);
    object-fit: contain;
    border-radius: 0.25rem;
  }

  .control-button {
    --button-size: 3rem;
    position: absolute;
    background-color: var(--color-gray-700);
    border: none;
    color: white;
    font-size: 1.5rem;
    cursor: pointer;
    height: var(--button-size);
    width: var(--button-size);
    border-radius: 50%;
    display: grid;
    place-items: center;
    transition: all 0.2s ease;
    pointer-events: auto;
  }

  .control-button:hover:not(:disabled),
  .control-button:focus:not(:disabled) {
    background-color: var(--color-gray-600);
    transform: scale(1.1);
  }

  .control-button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .close-button {
    top: 1rem;
    right: 1rem;
  }

  .nav-button.prev {
    left: 1rem;
    top: 50%;
    transform: translateY(-50%);
  }

  .nav-button.next {
    right: 1rem;
    top: 50%;
    transform: translateY(-50%);
  }

  .nav-button:hover:not(:disabled) {
    transform: translateY(-50%) scale(1.1);
  }

  .toolbar {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 3rem;
    background-color: var(--color-gray-800);
    display: flex;
    justify-content: center;
    align-items: center;
    border-bottom-left-radius: 0.5rem;
    border-bottom-right-radius: 0.5rem;
  }

  .favorite-button {
    position: static;
    --button-size: 2.5rem;
    background: none;
    color: var(--color-gray-300);
  }

  .favorite-button:hover,
  .favorite-button:focus {
    color: var(--color-error);
    background: none;
    transform: scale(1.2);
  }

  .favorite-button.active {
    color: var(--color-error);
  }

  @media (max-width: 800px) {
    .control-button {
      --button-size: 2.5rem;
      font-size: 1.25rem;
    }

    .nav-button {
      top: auto;
      bottom: 4rem;
      transform: none;
    }

    .nav-button:hover:not(:disabled) {
      transform: scale(1.1);
    }
  }
</style>

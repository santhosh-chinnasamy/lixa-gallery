<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-dialog';
  import Gallery from '../../components/Gallery.svelte';
  import { favorites } from '../../stores/galleryStore';
  import type { KeyboardActions } from '../../types/events';

  const APP_NAME = 'Lixa Gallery';
  $: exportButtonText = 'Export Favourites';

  const toggleFullScreen = async () => {
    const fullscreen = await getCurrentWindow().isFullscreen();

    if (!fullscreen) {
      await getCurrentWindow().setFullscreen(true);
    }

    if (fullscreen) {
      await getCurrentWindow().setFullscreen(false);
    }
  };

  const keyboardActions: KeyboardActions = {
    F11: toggleFullScreen,
  };

  document.addEventListener('keydown', async (event) => {
    try {
      const action = keyboardActions[event.key];
      if (action) action();
    } catch (error) {
      console.error(
        `Error executing keyboard action: [key: ${event.key}]`,
        error,
      );
    }
  });

  const exportFiles = async () => {
    try {
      const destination = await open({
        multiple: false,
        directory: true,
      });
      if (!destination) return;

      listen('export-progress', (event) => {
        exportButtonText = `Exporting ${event.payload} /`;
      }).then((unlisten) => {
        setTimeout(() => {
          unlisten();
        }, 10000);
      });

      await invoke('export_favourites', {
        destination,
      });

      alert(`Favourites exported to ${destination}`);
    } catch (error) {
      console.error('Failed to export files:', error);
    } finally {
      exportButtonText = 'Export Favourites';
    }
  };

  const clearFavourites = async () => {
    const isClearConfirmed = await confirm(
      'Are you sure you want to clear all favourites? This action cannot be undone.',
    );

    if (!isClearConfirmed) return;

    favorites.clear();
  };
</script>

<main class="container">
  <header class="header">
    <div>
      <h1>{APP_NAME}</h1>
      <p>Select your favorite photos and export them</p>
    </div>
    <div>
      <button class="button" onclick={exportFiles}
        >{exportButtonText} {$favorites.size}</button
      >
      <a href="/">All Photos</a>
      <button class="btn-danger" onclick={clearFavourites}>🗑️</button>
    </div>
  </header>

  {#if $favorites.size === 0}
    <p class="no-photos">No Favourites found.</p>
  {:else}
    <Gallery photos={$favorites} />
  {/if}

  <footer class="footer">
    <p>
      <a
        href="http://github.com/santhosh-chinnasamy/lixa-gallery"
        target="_blank"
      >
        {APP_NAME}
      </a>
      &copy; {new Date().getFullYear()}
    </p>
  </footer>
</main>

<style>
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: white;
    box-shadow: var(--shadow-sm);
    padding: var(--space-6) var(--space-4);
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .header h1 {
    font-size: 2rem;
    margin: 0;
  }
  .header p {
    font-size: 1rem;
    margin: 0;
  }
  .header button {
    font-size: 1rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.25rem;
    background-color: var(--color-primary-900);
    color: #fff;
    cursor: pointer;
  }
  .header button:hover {
    background-color: #0069d9;
  }

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

  .footer {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 1rem;
    position: sticky;
    bottom: 0;
    background-color: none;
  }
  .footer p {
    font-size: 1rem;
    margin: 0;
  }

  .header a {
    font-size: 1rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.25rem;
    background-color: var(--color-primary-900);
    color: #fff;
    cursor: pointer;
    text-decoration: none;
  }

  button.btn-danger:hover {
    background-color: var(--color-error);
  }
</style>

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
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

<main class="max-w-[100vw]">
  <header
    class="sticky top-0 z-10 flex items-center justify-between bg-white px-4 py-6 shadow-sm"
  >
    <div>
      <h1 class="m-0 text-4xl font-bold">{APP_NAME}</h1>
      <p class="m-0 text-base">Select your favorite photos and export them</p>
    </div>
    <div class="flex items-center gap-2">
      <Button variant="default" on:click={exportFiles}>
        {exportButtonText}
        {$favorites.size}
      </Button>
      <Button variant="outline" href="/" class="no-underline">
        All Photos
      </Button>
      <Button variant="destructive" on:click={clearFavourites} class="px-3">
        🗑️
      </Button>
    </div>
  </header>

  {#if $favorites.size === 0}
    <div
      class="flex h-[calc(100vh-100px)] items-center justify-center text-center text-xl font-bold text-gray-600"
    >
      No Favourites found.
    </div>
  {:else}
    <Gallery photos={$favorites} />
  {/if}

  <footer
    class="sticky bottom-0 flex items-center justify-center bg-transparent p-4"
  >
    <p class="m-0 text-base">
      <a
        href="http://github.com/santhosh-chinnasamy/lixa-gallery"
        target="_blank"
        class="text-primary transition-colors hover:text-primary/90"
      >
        {APP_NAME}
      </a>
      &copy; {new Date().getFullYear()}
    </p>
  </footer>
</main>

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-dialog';
  import Gallery from '../components/Gallery.svelte';
  import { favorites, isLoading, photos } from '../stores/galleryStore';
  import type { KeyboardActions } from '../types/events';

  const APP_NAME = 'Lixa Gallery';

  const loadPhotos = async () => {
    try {
      isLoading.set(true);
      const folder = await open({
        multiple: false,
        directory: true,
      });
      if (!folder) {
        isLoading.set(false);
        photos.set([]);
        return;
      }
      const loadedPhotos: string[] = await invoke('scan_folder', {
        path: folder,
      });
      photos.set(loadedPhotos);
    } catch (error) {
      console.error('Failed to load photos:', error);
    } finally {
      isLoading.set(false);
    }
  };

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
    o: loadPhotos,
    F11: toggleFullScreen,
  };

  const handleKeydown = (event: KeyboardEvent) => {
    try {
      const action = keyboardActions[event.key];
      if (action) action();
    } catch (error) {
      console.error(
        `Error executing keyboard action: [key: ${event.key}]`,
        error,
      );
    }
  };
</script>

<svelte:window onkeydown={handleKeydown} />

<main class="max-w-[100vw]">
  <header
    class="sticky top-0 z-10 flex items-center justify-between bg-gray-50 px-4 py-6 shadow-sm"
  >
    <h3 class="scroll-m-20 text-2xl font-semibold tracking-tight">
      {APP_NAME}
    </h3>
    <div>
      <a
        href="/favourites"
        class="inline-flex h-10 items-center justify-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground ring-offset-background transition-colors hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
      >
        Show Favourites {$favorites.size}
      </a>
    </div>
  </header>

  <div class="top-10 flex items-center justify-center p-4">
    {#if $isLoading}
      <p class="text-center">Loading...</p>
    {:else if $photos.length === 0}
      <div>
        <Button on:click={loadPhotos}>Open Folder</Button>
      </div>
    {:else}
      <Gallery photos={$photos} />
    {/if}
  </div>
</main>

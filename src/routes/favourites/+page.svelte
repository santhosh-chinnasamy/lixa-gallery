<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import { listen } from '@tauri-apps/api/event';
  import Gallery from '../../components/Gallery.svelte';
  import {
    clearFavorites,
    exportFavorites,
  } from '../../components/common/ImageOperations';
  import KeyboardShortcuts from '../../components/common/KeyboardShortcuts.svelte';
  import AppLayout from '../../components/layout/AppLayout.svelte';
  import { favorites } from '../../stores/galleryStore';

  let exportButtonText = $state('Export Favourites');

  const keyboardActions = {
    e: handleExport,
  };

  async function handleExport() {
    try {
      const unsubscribe = await listen('export-progress', (event) => {
        exportButtonText = `Exporting ${event.payload} /`;
      });

      setTimeout(() => unsubscribe(), 10000);

      const destination = await exportFavorites();
      if (destination) {
        alert(`Favourites exported to ${destination}`);
      }
    } catch (error) {
      console.error('Export failed:', error);
    } finally {
      exportButtonText = 'Export Favourites';
    }
  }
</script>

<KeyboardShortcuts actions={keyboardActions} />

<AppLayout>
  {#if $favorites.size === 0}
    <div class="flex flex-col items-center justify-end">
      <p class="mb-4 text-lg">No favorites</p>
      <Button href="/">Home</Button>
    </div>
  {:else}
    <div class="fixed right-4 top-4 mb-[4rem] flex items-center z-10">
      <Button variant="default" onclick={handleExport} class="mr-2">
        {exportButtonText}
        {$favorites.size}
      </Button>
      <Button variant="destructive" onclick={clearFavorites} class="px-3">
        <TrashIcon />
      </Button>
    </div>
    <br />
    <Gallery photos={Array.from($favorites)} />
  {/if}
</AppLayout>

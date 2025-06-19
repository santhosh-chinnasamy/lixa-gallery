<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { listen } from '@tauri-apps/api/event';
  import AppLayout from '../../components/layout/AppLayout.svelte';
  import Gallery from '../../components/Gallery.svelte';
  import KeyboardShortcuts from '../../components/common/KeyboardShortcuts.svelte';
  import { exportFavorites, clearFavorites } from '../../components/common/ImageOperations';
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

<AppLayout
  title="Lixa Gallery"
  subtitle="Select your favorite photos and export them"
>
  <svelte:fragment slot="header-actions">
    <Button variant="default" on:click={handleExport}>
      {exportButtonText}
      {$favorites.size}
    </Button>
    <Button variant="outline" href="/" class="no-underline">
      All Photos
    </Button>
    <Button variant="destructive" on:click={clearFavorites} class="px-3">
      🗑️
    </Button>
  </svelte:fragment>

  {#if $favorites.size === 0}
    <div class="text-center">
      <p class="mb-4 text-lg">No favorites yet</p>
      <Button variant="outline" href="/">Browse Photos</Button>
    </div>
  {:else}
    <Gallery photos={Array.from($favorites)} />
  {/if}
</AppLayout>

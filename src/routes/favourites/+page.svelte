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
  import { favorites, photos } from '../../stores/galleryStore';
  
  // Filter photos to only show favorites
  const favoritePhotos = $derived($photos.filter(photo => $favorites.has(photo.path)));

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
    <!-- Action buttons container -->
    <div class="sticky top-4 z-10 mb-4 flex justify-center px-4">
      <div class="flex items-center gap-2 rounded-lg bg-background/80 p-2 shadow-sm backdrop-blur-sm border">
        <Button 
          variant="outline" 
          onclick={handleExport}
          class="text-sm font-medium"
        >
          {exportButtonText}
          <span class="ml-1 rounded-full bg-primary/10 px-2 py-0.5 text-xs">
            {$favorites.size}
          </span>
        </Button>
        <Button 
          variant="destructive" 
          onclick={clearFavorites}
          size="sm"
          class="px-3"
        >
          <TrashIcon class="h-4 w-4" />
        </Button>
      </div>
    </div>
    
    <Gallery photos={favoritePhotos} />
  {/if}
</AppLayout>

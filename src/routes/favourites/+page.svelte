<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import HeartIcon from '@lucide/svelte/icons/heart';
  import { listen } from '@tauri-apps/api/event';
  import Gallery from '../../components/Gallery.svelte';
  import { exportFavorites } from '../../components/common/ImageOperations';
  import KeyboardShortcuts from '../../components/common/KeyboardShortcuts.svelte';
  import ConfirmationModal from '../../components/modals/ConfirmationModal.svelte';
  import { favorites, photos } from '../../stores/galleryStore';

  // Filter photos to only show favorites
  const favoritePhotos = $derived(
    $photos.filter((photo) => $favorites.has(photo.path)),
  );

  let exportButtonText = $state('Export Favourites');
  let showDeleteConfirmation = $state(false);

  async function handleClearFavorites() {
    await favorites.clear();
  }

  function showDeleteModal() {
    showDeleteConfirmation = true;
  }

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

{#if $favorites.size === 0}
  <div
    class="flex h-[80vh] flex-col items-center justify-center p-4 text-center"
  >
    <div
      class="mb-6 flex h-20 w-20 items-center justify-center rounded-2xl bg-primary/10 text-primary"
    >
      <HeartIcon size={40} strokeWidth={1.5} />
    </div>
    <h2 class="text-2xl font-bold tracking-tight text-foreground">
      No Favourites Yet
    </h2>
    <p class="mt-2 max-w-xs leading-relaxed text-muted-foreground">
      Browse your library and mark your best shots with a heart to see them
      here.
    </p>
    <Button href="/" variant="outline" class="mt-8">Back to Library</Button>
  </div>
{:else}
  <!-- Action buttons container -->
  <div class="sticky top-4 z-10 mb-4 flex justify-center px-4">
    <div
      class="flex items-center gap-2 rounded-lg border bg-background/80 p-2 shadow-sm backdrop-blur-sm"
    >
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
        onclick={showDeleteModal}
        size="sm"
        class="px-3"
      >
        <TrashIcon class="h-4 w-4" />
      </Button>
    </div>
  </div>

  <Gallery photos={favoritePhotos} />
{/if}

<ConfirmationModal
  bind:open={showDeleteConfirmation}
  title="Clear All Favourites"
  description="Are you sure you want to clear all favourites? This action cannot be undone."
  confirmText="Clear All"
  cancelText="Cancel"
  variant="destructive"
  icon={TrashIcon}
  onConfirm={handleClearFavorites}
  onCancel={() => {
    showDeleteConfirmation = false;
  }}
/>

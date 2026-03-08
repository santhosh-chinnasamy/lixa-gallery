<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import HeartIcon from '@lucide/svelte/icons/heart';
  import { listen } from '@tauri-apps/api/event';
  import Gallery from '../../components/Gallery.svelte';
  import { exportFavorites } from '../../components/common/ImageOperations';
  import KeyboardShortcuts from '../../components/common/KeyboardShortcuts.svelte';
  import ConfirmationModal from '../../components/modals/ConfirmationModal.svelte';
  import ExportModal from '../../components/modals/ExportModal.svelte';
  import { favorites, photos } from '../../stores/galleryStore';

  // Filter photos to only show favorites
  const favoritePhotos = $derived(
    $photos.filter((photo) => $favorites.has(photo.path)),
  );

  let exportStatus = $state('');
  let showDeleteConfirmation = $state(false);
  let showExportModal = $state(false);
  let isExporting = $state(false);

  async function handleClearFavorites() {
    await favorites.clear();
  }

  function showDeleteModal() {
    showDeleteConfirmation = true;
  }

  const keyboardActions = {
    e: () => {
      showExportModal = true;
    },
  };

  async function handleExport(mode: 'copy' | 'move') {
    try {
      isExporting = true;
      exportStatus = 'Preparing export...';
      const unsubscribe = await listen('export-progress', (event) => {
        exportStatus = `Exporting ${event.payload} / ${$favorites.size} files...`;
      });

      // Temporary timeout fallback for progress listener if it hangs
      setTimeout(() => unsubscribe(), 10000);

      // Pass mode down to the operations layer
      const destination = await exportFavorites(mode);
      if (destination) {
        // Only show success alert if not moving (since moving will empty the view)
        showExportModal = false;
        setTimeout(
          () =>
            alert(
              `Favourites successfully ${mode === 'copy' ? 'copied' : 'moved'} to ${destination}`,
            ),
          100,
        );
      } else {
        showExportModal = false;
      }
    } catch (error) {
      console.error('Export failed:', error);
      showExportModal = false;
      setTimeout(
        () => alert('Export failed. Please check the destination permissions.'),
        100,
      );
    } finally {
      isExporting = false;
      exportStatus = '';
    }
  }
</script>

<KeyboardShortcuts actions={keyboardActions} />

<div class="flex h-full flex-col overflow-hidden">
  {#if $favorites.size === 0}
    <div
      class="flex h-full flex-col items-center justify-center p-4 text-center"
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
    <div class="z-10 flex shrink-0 items-center justify-between p-6 pb-2">
      <div class="flex items-center gap-2">
        <span class="ml-2 text-sm font-semibold tracking-tight text-foreground">
          <span class="mr-1 rounded-md bg-primary/10 px-2 py-1 text-primary">
            {$favorites.size}
          </span>
          Photos Selected
        </span>
      </div>
      <div
        class="flex items-center gap-3 rounded-lg border bg-background/80 p-2 shadow-[0_4px_12px_rgba(0,0,0,0.05)] backdrop-blur-md"
      >
        <Button
          variant="secondary"
          onclick={showDeleteModal}
          size="sm"
          class="text-sm shadow-none"
        >
          Clear Selection
        </Button>
        <Button
          variant="default"
          onclick={() => {
            showExportModal = true;
          }}
          size="sm"
          class="text-sm font-medium shadow-none"
        >
          Export
        </Button>
      </div>
    </div>

    <!-- The Gallery component will take the remaining vertical flex space and control its own internal scrolling -->
    <div class="flex-1 overflow-hidden">
      <Gallery photos={favoritePhotos} />
    </div>
  {/if}
</div>

<ExportModal
  bind:open={showExportModal}
  onExport={handleExport}
  onCancel={() => {
    showExportModal = false;
  }}
  {isExporting}
  {exportStatus}
/>

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

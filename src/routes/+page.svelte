<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import LandingPage from '../components/LandingPage.svelte';
  import Gallery from '../components/Gallery.svelte';
  import { loadPhotosWithModal } from '../components/common/ImageOperations';
  import KeyboardShortcuts from '../components/common/KeyboardShortcuts.svelte';
  import FolderLoadingModal from '../components/modals/FolderLoadingModal.svelte';
  import { isLoading, photos } from '../stores/galleryStore';

  let showFolderModal = $state(false);

  function handleLoadPhotos() {
    loadPhotosWithModal(
      () => {
        showFolderModal = true;
      },
      () => {
        showFolderModal = false;
      },
    );
  }

  const keyboardActions = {
    o: handleLoadPhotos,
  };
</script>

<KeyboardShortcuts actions={keyboardActions} />

{#if $isLoading && $photos.length === 0}
  <div class="flex h-full items-center justify-center">
    <p class="animate-pulse text-muted-foreground">Initializing workspace...</p>
  </div>
{:else if $photos.length === 0}
  <LandingPage />
{:else}
  <!-- Action buttons container -->
  <div class="sticky top-4 z-10 mb-4 flex justify-center px-4">
    <div
      class="flex items-center gap-2 rounded-lg border bg-background/80 p-2 shadow-sm backdrop-blur-sm"
    >
      <Button
        onclick={handleLoadPhotos}
        variant="outline"
        class="text-sm font-medium"
      >
        Choose another folder
      </Button>
    </div>
  </div>

  <Gallery photos={$photos} />
{/if}

<FolderLoadingModal
  bind:open={showFolderModal}
  isLoading={$isLoading}
  onSelectFolder={handleLoadPhotos}
  onCancel={() => {
    showFolderModal = false;
  }}
/>

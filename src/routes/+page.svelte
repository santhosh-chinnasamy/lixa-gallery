<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import LandingPage from '../components/LandingPage.svelte';
  import Gallery from '../components/Gallery.svelte';
  import { loadPhotosWithModal } from '../components/common/ImageOperations';
  import KeyboardShortcuts from '../components/common/KeyboardShortcuts.svelte';
  import FolderLoadingModal from '../components/modals/FolderLoadingModal.svelte';
  import FolderExplorer from '../components/FolderExplorer.svelte';
  import Breadcrumbs from '../components/Breadcrumbs.svelte';
  import { isLoading, photos, currentFolder } from '../stores/galleryStore';

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

{#if $isLoading && $currentFolder === null}
  <div class="flex h-full items-center justify-center">
    <div class="text-center">
      <div class="mb-4 flex justify-center">
        <div
          class="h-10 w-10 animate-spin rounded-full border-4 border-primary border-t-transparent"
        ></div>
      </div>
      <p class="animate-pulse text-lg font-medium text-muted-foreground">
        Initializing workspace...
      </p>
      <p class="mt-2 text-sm text-muted-foreground/60">
        This may take a moment for large folders
      </p>
    </div>
  </div>
{:else if $currentFolder === null}
  <LandingPage />
{:else}
  <div class="flex h-full overflow-hidden bg-background">
    <!-- Main Content Area -->
    <div class="relative flex h-full flex-grow flex-col overflow-hidden">
      <main class="relative flex-grow overflow-y-auto bg-muted/20 p-4 lg:p-6">
        {#if $isLoading}
          <div
            class="absolute inset-0 z-20 flex items-start justify-center bg-background/40 pt-12 backdrop-blur-[1px] transition-all"
          >
            <div
              class="flex items-center gap-3 rounded-full border bg-background/90 px-4 py-2 shadow-lg"
            >
              <div
                class="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent"
              ></div>
              <span class="text-xs font-medium">Scanning Folder...</span>
            </div>
          </div>
        {/if}
        <Gallery photos={$photos} onLoadPhotos={handleLoadPhotos} />
      </main>
    </div>
  </div>
{/if}

<FolderLoadingModal
  bind:open={showFolderModal}
  isLoading={$isLoading}
  onSelectFolder={handleLoadPhotos}
  onCancel={() => {
    showFolderModal = false;
  }}
/>

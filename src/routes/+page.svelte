<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import LandingPage from '../components/LandingPage.svelte';
  import Gallery from '../components/Gallery.svelte';
  import { loadPhotosWithModal } from '../components/common/ImageOperations';
  import KeyboardShortcuts from '../components/common/KeyboardShortcuts.svelte';
  import FolderLoadingModal from '../components/modals/FolderLoadingModal.svelte';
  import FolderExplorer from '../components/FolderExplorer.svelte';
  import Breadcrumbs from '../components/Breadcrumbs.svelte';
  import { isLoading, photos, explorerWidth } from '../stores/galleryStore';

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

  let isResizing = $state(false);

  function startResize(e: MouseEvent) {
    isResizing = true;
    e.preventDefault();
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isResizing) return;
    // sidebar is roughly 250-280px, but let's just calculate relative to window
    // the explorer starts after the sidebar.
    // sidebar width is not in store yet, but let's assume it's fixed for now or calculate from offset.
    const newWidth = Math.max(150, Math.min(600, e.clientX - 280)); // 280 is approx sidebar width
    explorerWidth.set(newWidth);
  }

  function stopResize() {
    isResizing = false;
  }
</script>

<svelte:window onmousemove={handleMouseMove} onmouseup={stopResize} />

<KeyboardShortcuts actions={keyboardActions} />

{#if $isLoading && $photos.length === 0}
  <div class="flex h-full items-center justify-center">
    <p class="animate-pulse text-muted-foreground">Initializing workspace...</p>
  </div>
{:else if $photos.length === 0}
  <LandingPage />
{:else}
  <div class="flex h-full overflow-hidden bg-background">
    <!-- Folder Explorer Pane -->
    <div
      style="width: {$explorerWidth}px"
      class="hidden h-full shrink-0 md:block"
    >
      <FolderExplorer />
    </div>

    <!-- Resizable Handle -->
    <div
      class={`z-20 w-1 shrink-0 cursor-col-resize self-stretch transition-colors hover:bg-primary/50 ${isResizing ? 'bg-primary' : 'bg-border/50'}`}
      onmousedown={startResize}
      role="separator"
      aria-label="Resize Explorer"
    ></div>

    <!-- Main Content Area -->
    <div class="relative flex h-full flex-grow flex-col overflow-hidden">
      <header
        class="sticky top-0 z-10 flex items-center justify-between border-b bg-background/50 px-2 backdrop-blur-md"
      >
        <Breadcrumbs />

        <div class="flex items-center gap-2 p-1">
          <Button
            onclick={handleLoadPhotos}
            variant="ghost"
            size="sm"
            class="h-8 gap-2 text-xs"
          >
            <span>Change Base Folder</span>
          </Button>
        </div>
      </header>

      <main class="custom-scrollbar flex-grow overflow-y-auto p-4 lg:p-6">
        <Gallery photos={$photos} />
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

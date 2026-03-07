<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import LandingPage from '../components/LandingPage.svelte';
  import Gallery from '../components/Gallery.svelte';
  import { loadPhotosWithModal } from '../components/common/ImageOperations';
  import KeyboardShortcuts from '../components/common/KeyboardShortcuts.svelte';
  import FolderLoadingModal from '../components/modals/FolderLoadingModal.svelte';
  import FolderExplorer from '../components/FolderExplorer.svelte';
  import Breadcrumbs from '../components/Breadcrumbs.svelte';
  import {
    isLoading,
    photos,
    explorerWidth,
    currentFolder,
  } from '../stores/galleryStore';

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

      <main class="relative flex-grow overflow-y-auto p-4 lg:p-6">
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

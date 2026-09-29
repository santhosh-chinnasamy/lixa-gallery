<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import HeartIcon from '@lucide/svelte/icons/heart';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import LayersIcon from '@lucide/svelte/icons/layers';
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import { listen } from '@tauri-apps/api/event';
  import Gallery from '../../components/Gallery.svelte';
  import ImageCard from '../../components/ImageCard.svelte';
  import ImageModal from '../../components/ImageModal.svelte';
  import { exportFavorites } from '../../components/common/ImageOperations';
  import KeyboardShortcuts from '../../components/common/KeyboardShortcuts.svelte';
  import ConfirmationModal from '../../components/modals/ConfirmationModal.svelte';
  import ExportModal from '../../components/modals/ExportModal.svelte';
  import {
    favorites,
    currentFolder,
    favouriteScope,
    favouritePhotos,
    favouriteFolderGroups,
    isLoadingFavourites,
    currentFolderFavoritesCount,
    refreshFavourites,
  } from '../../stores/galleryStore';
  import type { FavouriteFolderGroup, PhotoMetadata } from '../../types/photo';

  let exportStatus = $state('');
  let showClearAllConfirmation = $state(false);
  let showClearFolderConfirmation = $state(false);
  let showExportModal = $state(false);
  let isExporting = $state(false);

  // Modal viewer state for All Folders view
  let selectedModalImage = $state<PhotoMetadata | null>(null);
  let modalPhotos = $state<PhotoMetadata[]>([]);

  // Collapsed state for folder accordions in All Folders view
  let collapsedFolders = $state<Set<string>>(new Set());

  // Specific folder targeted for quick export (null = use modal's general scope)
  let exportTargetFolder = $state<string | null>(null);

  // Auto-refresh when entering page
  $effect(() => {
    refreshFavourites();
  });

  // Automatically adjust scope if no folder is open
  $effect(() => {
    if (!$currentFolder && $favouriteScope === 'current') {
      favouriteScope.set('all');
    }
  });

  // Current folder's photos
  const currentFolderPhotos = $derived.by(() => {
    if (!$currentFolder) return [];
    return $favouritePhotos.filter((p) => p.path.startsWith($currentFolder!));
  });

  const keyboardActions = {
    e: () => {
      exportTargetFolder = null;
      showExportModal = true;
    },
  };

  function toggleFolderCollapse(folderPath: string) {
    const next = new Set(collapsedFolders);
    if (next.has(folderPath)) {
      next.delete(folderPath);
    } else {
      next.add(folderPath);
    }
    collapsedFolders = next;
  }

  function handleImageClickInGroup(
    photo: PhotoMetadata,
    groupPhotos: PhotoMetadata[],
  ) {
    modalPhotos = groupPhotos;
    selectedModalImage = photo;
  }

  function handleQuickExportFolder(folderPath: string) {
    exportTargetFolder = folderPath;
    showExportModal = true;
  }

  async function handleClearCurrentFolder() {
    if ($currentFolder) {
      await favorites.clearPrefix($currentFolder);
    }
    showClearFolderConfirmation = false;
  }

  async function handleClearAllFavorites() {
    await favorites.clear();
    showClearAllConfirmation = false;
  }

  async function handleExport(
    mode: 'copy' | 'move',
    paths?: string[],
    preserveFolderStructure = true,
  ) {
    try {
      isExporting = true;
      exportStatus = 'Preparing export...';
      const totalExporting = paths ? paths.length : $favorites.size;
      const unsubscribe = await listen('export-progress', (event) => {
        exportStatus = `Exporting ${event.payload} / ${totalExporting} files...`;
      });

      setTimeout(() => unsubscribe(), 15000);

      const destination = await exportFavorites(
        mode,
        paths,
        preserveFolderStructure,
      );

      if (destination) {
        showExportModal = false;
        await refreshFavourites();
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
      exportTargetFolder = null;
    }
  }
</script>

<KeyboardShortcuts actions={keyboardActions} />

<div class="flex h-full flex-col overflow-hidden bg-background">
  {#if $favorites.size === 0 && !$isLoadingFavourites}
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
    <!-- Top Action & Scope Control Bar -->
    <div
      class="z-10 flex shrink-0 flex-wrap items-center justify-between gap-4 border-b bg-background/80 p-4 backdrop-blur-md lg:px-6"
    >
      <!-- Scope Switcher Pill -->
      <div class="flex items-center gap-3">
        {#if $currentFolder}
          <div class="flex items-center rounded-lg border bg-muted/40 p-1">
            <button
              class={`flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-semibold transition-all ${
                $favouriteScope === 'current'
                  ? 'shadow-xs bg-background text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
              onclick={() => {
                favouriteScope.set('current');
                refreshFavourites();
              }}
            >
              <FolderIcon size={14} />
              <span>Current Folder</span>
              <span
                class="rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] font-bold text-primary"
              >
                {$currentFolderFavoritesCount}
              </span>
            </button>

            <button
              class={`flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-semibold transition-all ${
                $favouriteScope === 'all'
                  ? 'shadow-xs bg-background text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
              onclick={() => {
                favouriteScope.set('all');
                refreshFavourites();
              }}
            >
              <LayersIcon size={14} />
              <span>All Folders</span>
              <span
                class="rounded-full bg-muted-foreground/15 px-1.5 py-0.5 text-[10px] font-semibold text-muted-foreground"
              >
                {$favorites.size}
              </span>
            </button>
          </div>
        {:else}
          <div class="flex items-center gap-2">
            <span class="text-sm font-semibold tracking-tight text-foreground">
              <span
                class="mr-1 rounded-md bg-primary/10 px-2 py-1 text-primary"
              >
                {$favorites.size}
              </span>
              Photos Selected Across All Folders
            </span>
          </div>
        {/if}
      </div>

      <!-- Action Buttons -->
      <div class="flex items-center gap-2.5">
        {#if $favouriteScope === 'current' && $currentFolder && $currentFolderFavoritesCount > 0}
          <Button
            variant="outline"
            size="sm"
            onclick={() => (showClearFolderConfirmation = true)}
            class="text-xs"
          >
            Clear Current Selection
          </Button>
        {:else if $favouriteScope === 'all' && $favorites.size > 0}
          <Button
            variant="outline"
            size="sm"
            onclick={() => (showClearAllConfirmation = true)}
            class="text-xs"
          >
            Clear All
          </Button>
        {/if}

        <Button
          variant="default"
          size="sm"
          onclick={() => {
            exportTargetFolder = null;
            showExportModal = true;
          }}
          class="gap-1.5 text-xs font-medium"
        >
          <DownloadIcon size={14} />
          <span>Export</span>
        </Button>
      </div>
    </div>

    <!-- Main Content Area -->
    <div class="flex-1 overflow-hidden">
      {#if $favouriteScope === 'current' && $currentFolder}
        <!-- View 1: Current Folder Favorites -->
        {#if currentFolderPhotos.length === 0}
          <div
            class="flex h-full flex-col items-center justify-center p-6 text-center"
          >
            <div
              class="mb-4 flex h-14 w-14 items-center justify-center rounded-xl bg-muted text-muted-foreground"
            >
              <FolderIcon size={28} />
            </div>
            <h3 class="text-lg font-semibold text-foreground">
              No Favourites in This Folder
            </h3>
            <p class="mt-1 max-w-sm text-xs text-muted-foreground">
              You haven't marked any photos as favorite in <code
                class="rounded bg-muted px-1.5 py-0.5 font-mono text-[11px]"
                >{$currentFolder.split('/').pop() || $currentFolder}</code
              >.
            </p>
            <div class="mt-6 flex items-center gap-3">
              <Button href="/" variant="outline" size="sm">
                Browse Folder
              </Button>
              {#if $favorites.size > 0}
                <Button
                  variant="secondary"
                  size="sm"
                  onclick={() => {
                    favouriteScope.set('all');
                    refreshFavourites();
                  }}
                >
                  View All Folders ({$favorites.size})
                </Button>
              {/if}
            </div>
          </div>
        {:else}
          <Gallery photos={currentFolderPhotos} />
        {/if}
      {:else}
        <!-- View 2: All Folders (Grouped Accordions) -->
        <div class="custom-scrollbar h-full overflow-y-auto p-4 lg:p-6">
          <div class="space-y-6">
            {#each $favouriteFolderGroups as group (group.folder_path)}
              {@const isCollapsed = collapsedFolders.has(group.folder_path)}
              <div
                class="shadow-xs backdrop-blur-xs rounded-xl border bg-card/60 transition-all"
              >
                <!-- Folder Section Header -->
                <div
                  class="flex items-center justify-between border-b bg-muted/30 px-4 py-3"
                >
                  <button
                    type="button"
                    class="flex min-w-0 items-center gap-2.5 text-left transition-opacity hover:opacity-80"
                    onclick={() => toggleFolderCollapse(group.folder_path)}
                  >
                    {#if isCollapsed}
                      <ChevronRightIcon
                        size={16}
                        class="shrink-0 text-muted-foreground"
                      />
                    {:else}
                      <ChevronDownIcon
                        size={16}
                        class="shrink-0 text-muted-foreground"
                      />
                    {/if}
                    <FolderIcon size={18} class="shrink-0 text-primary" />
                    <div class="min-w-0">
                      <div class="flex items-center gap-2">
                        <span
                          class="truncate text-sm font-semibold text-foreground"
                        >
                          {group.folder_name}
                        </span>
                        <span
                          class="rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-bold text-primary"
                        >
                          {group.count}
                        </span>
                      </div>
                      <p class="truncate text-[11px] text-muted-foreground">
                        {group.folder_path}
                      </p>
                    </div>
                  </button>

                  <div class="flex items-center gap-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      class="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
                      onclick={() => handleQuickExportFolder(group.folder_path)}
                    >
                      <DownloadIcon size={12} />
                      <span class="hidden sm:inline">Export Folder</span>
                    </Button>
                  </div>
                </div>

                <!-- Photos Grid in this Folder -->
                {#if !isCollapsed}
                  <div class="p-4">
                    <div
                      class="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8"
                    >
                      {#each group.photos as photo (photo.path)}
                        <ImageCard
                          {photo}
                          handleImageClick={(p) =>
                            handleImageClickInGroup(p, group.photos)}
                        />
                      {/each}
                    </div>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- Modal Viewer for All Folders View -->
<ImageModal
  bind:selectedImage={selectedModalImage}
  photos={modalPhotos}
  onClose={() => (selectedModalImage = null)}
/>

<!-- Export Modal -->
<ExportModal
  bind:open={showExportModal}
  folderGroups={$favouriteFolderGroups}
  currentFolderPath={exportTargetFolder ||
    ($favouriteScope === 'current' ? $currentFolder : null)}
  onExport={handleExport}
  onCancel={() => {
    showExportModal = false;
    exportTargetFolder = null;
  }}
  {isExporting}
  {exportStatus}
/>

<!-- Confirmation for Clear Current Folder -->
<ConfirmationModal
  bind:open={showClearFolderConfirmation}
  title="Clear Current Folder Favourites"
  description={`Are you sure you want to remove all favourites from ${$currentFolder ? $currentFolder.split('/').pop() : 'this folder'}?`}
  confirmText="Clear Folder"
  cancelText="Cancel"
  variant="destructive"
  icon={TrashIcon}
  onConfirm={handleClearCurrentFolder}
  onCancel={() => (showClearFolderConfirmation = false)}
/>

<!-- Confirmation for Clear All -->
<ConfirmationModal
  bind:open={showClearAllConfirmation}
  title="Clear All Favourites"
  description="Are you sure you want to clear all favourites across all folders? This action cannot be undone."
  confirmText="Clear All"
  cancelText="Cancel"
  variant="destructive"
  icon={TrashIcon}
  onConfirm={handleClearAllFavorites}
  onCancel={() => (showClearAllConfirmation = false)}
/>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 6px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(0, 0, 0, 0.1);
    border-radius: 10px;
  }
  :global(.dark) .custom-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
  }
</style>

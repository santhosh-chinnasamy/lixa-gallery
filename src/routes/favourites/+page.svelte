<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import HeartIcon from '@lucide/svelte/icons/heart';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import LayersIcon from '@lucide/svelte/icons/layers';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import Play from '@lucide/svelte/icons/play';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { onDestroy, onMount } from 'svelte';
  import Gallery from '../../components/Gallery.svelte';
  import SlideshowModal from '../../components/SlideshowModal.svelte';
  import { error as logError } from '$lib/logger';
  import {
    describeExportError,
    exportFavorites,
  } from '../../components/common/ImageOperations';
  import KeyboardShortcuts from '../../components/common/KeyboardShortcuts.svelte';
  import ConfirmationModal from '../../components/modals/ConfirmationModal.svelte';
  import ExportModal from '../../components/modals/ExportModal.svelte';
  import ExportResultModal from '../../components/modals/ExportResultModal.svelte';
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
  import type { ExportReport } from '../../types/photo';
  import { setShortcutLayer } from '$lib/shortcuts';
  import { probe, startProbeSession } from '$lib/debugProbe';
  import { get } from 'svelte/store';

  let exportStatus = $state('');
  let isExporting = $state(false);
  let unlistenExport: UnlistenFn | null = null;
  let stopRequested = false;
  let showExportResult = $state(false);
  let exportResultReport = $state<ExportReport | null>(null);
  let exportResultDestination = $state('');
  let exportResultMode = $state<'copy' | 'move'>('copy');

  function stopExportListener() {
    const stop = unlistenExport;
    unlistenExport = null;
    stop?.();
  }

  onDestroy(() => {
    const exporting = isExporting;
    stopRequested = true;
    stopExportListener();
    if (exporting) {
      invoke('cancel_export').catch((error) => {
        console.error('Failed to cancel export:', error);
      });
    }
  });

  async function requestStopExport() {
    stopRequested = true;
    exportStatus = 'Stopping after the current file...';
    try {
      await invoke('cancel_export');
    } catch (error) {
      console.error('Failed to cancel export:', error);
    }
  }
  let showClearAllConfirmation = $state(false);
  let showClearFolderConfirmation = $state(false);
  let showExportModal = $state(false);
  let isSlideshowOpen = $state(false);

  // Specific folder targeted for quick export (null = use modal's general scope)
  let exportTargetFolder = $state<string | null>(null);

  // Load once on enter. An $effect here can re-fire if refresh touches tracked state.
  onMount(() => {
    const stopProbe = startProbeSession('favourites');
    probe('favourites.mount', {
      favorites: get(favorites).size,
      scope: get(favouriteScope),
      folder: get(currentFolder),
    });
    void refreshFavourites().then(() => {
      probe('favourites.refreshed', {
        favorites: get(favorites).size,
        photos: get(favouritePhotos).length,
        groups: get(favouriteFolderGroups).length,
        scope: get(favouriteScope),
      });
    });
    return () => {
      probe('favourites.unmount');
      stopProbe();
    };
  });

  $effect(() => {
    setShortcutLayer('favourites', true);
    probe('favourites.shortcutLayer', { active: true });
    return () => {
      setShortcutLayer('favourites', false);
      probe('favourites.shortcutLayer', { active: false });
    };
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

  const activeFavouritesList = $derived(
    $favouriteScope === 'current' ? currentFolderPhotos : $favouritePhotos,
  );

  const keyboardActions = {
    e: () => {
      exportTargetFolder = null;
      showExportModal = true;
    },
  };

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
    stopRequested = false;
    try {
      isExporting = true;
      exportStatus = 'Preparing export...';
      // Cleared before the folder dialog so Cancel during that dialog is not
      // wiped when the export command starts, and a previous run cannot stick.
      await invoke('begin_export');
      const totalExporting = paths ? paths.length : $favorites.size;
      unlistenExport = await listen('export-progress', (event) => {
        if (stopRequested) return;
        exportStatus = `Exporting ${event.payload} / ${totalExporting} files...`;
      });

      const exported = await exportFavorites(
        mode,
        paths,
        preserveFolderStructure,
        () => stopRequested,
      );

      showExportModal = false;
      if (exported) {
        await refreshFavourites();
        exportResultMode = mode;
        exportResultDestination = exported.destination;
        exportResultReport = exported.report;
        showExportResult = true;
      }
    } catch (error) {
      logError(`Export failed: ${error}`);
      showExportModal = false;
      const detail = describeExportError(error);
      exportResultMode = mode;
      exportResultDestination = '';
      exportResultReport = {
        copied: 0,
        renamed: 0,
        skipped: 0,
        failed: Math.max(paths?.length ?? 0, 1),
        failures: [
          {
            path: '',
            reason: detail,
          },
        ],
        cancelled: false,
        total: Math.max(paths?.length ?? 0, 1),
      };
      showExportResult = true;
    } finally {
      stopExportListener();
      isExporting = false;
      exportStatus = '';
      exportTargetFolder = null;
      stopRequested = false;
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

        {#if activeFavouritesList.length > 0}
          <Button
            variant="outline"
            size="sm"
            onclick={() => (isSlideshowOpen = true)}
            class="gap-1.5 text-xs font-medium transition-colors hover:border-primary/50 hover:text-primary"
            title="Start Slideshow of Favourites"
          >
            <Play size={13} class="fill-current text-primary" />
            <span>Slideshow</span>
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
    <div class="min-h-0 flex-1 overflow-hidden">
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
          <div class="h-full min-h-0">
            <Gallery
              photos={currentFolderPhotos}
              embedded
              source="favourites"
            />
          </div>
        {/if}
      {:else}
        <!-- All Folders: one virtualized grid. Nested accordion + scrollport
             painted blank under WebKit while blob thumbs still loaded. -->
        {#if $isLoadingFavourites}
          <div
            class="flex h-full items-center justify-center text-sm text-muted-foreground"
          >
            Loading favourites…
          </div>
        {:else if $favouritePhotos.length === 0}
          <div
            class="flex h-full flex-col items-center justify-center p-6 text-center text-muted-foreground"
          >
            <p class="text-sm">No favourite photos to show yet.</p>
          </div>
        {:else}
          <div class="h-full min-h-0">
            <Gallery
              photos={$favouritePhotos}
              embedded
              source="favourites"
            />
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<!-- Slideshow Modal -->
<SlideshowModal
  bind:open={isSlideshowOpen}
  photos={activeFavouritesList}
  onClose={() => (isSlideshowOpen = false)}
/>

<!-- Export Modal -->
<ExportModal
  bind:open={showExportModal}
  folderGroups={$favouriteFolderGroups}
  currentFolderPath={exportTargetFolder ||
    ($favouriteScope === 'current' ? $currentFolder : null)}
  onExport={handleExport}
  onStop={requestStopExport}
  onCancel={() => {
    showExportModal = false;
    exportTargetFolder = null;
  }}
  {isExporting}
  {exportStatus}
/>

<ExportResultModal
  bind:open={showExportResult}
  report={exportResultReport}
  destination={exportResultDestination}
  mode={exportResultMode}
  onClose={() => {
    showExportResult = false;
  }}
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

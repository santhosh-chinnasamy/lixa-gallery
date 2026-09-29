<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
  } from '$lib/components/ui/dialog';
  import { Switch } from '$lib/components/ui/switch';
  import { Label } from '$lib/components/ui/label';
  import Download from '@lucide/svelte/icons/download';
  import FolderOutput from '@lucide/svelte/icons/folder-output';
  import Copy from '@lucide/svelte/icons/copy';
  import Folder from '@lucide/svelte/icons/folder';
  import Check from '@lucide/svelte/icons/check';
  import Layers from '@lucide/svelte/icons/layers';
  import type { FavouriteFolderGroup } from '../../types/photo';

  let {
    open = $bindable(false),
    folderGroups = [],
    currentFolderPath = null,
    onExport,
    onCancel,
    isExporting = false,
    exportStatus = '',
  }: {
    open: boolean;
    folderGroups?: FavouriteFolderGroup[];
    currentFolderPath?: string | null;
    onExport: (
      mode: 'copy' | 'move',
      paths?: string[],
      preserveFolderStructure?: boolean,
    ) => void;
    onCancel: () => void;
    isExporting?: boolean;
    exportStatus?: string;
  } = $props();

  let exportScope = $state<'current' | 'all' | 'custom'>('all');
  let preserveFolderStructure = $state(true);
  let selectedFolderPaths = $state<Set<string>>(new Set());

  // Initialize or adjust scope when opening
  $effect(() => {
    if (open) {
      if (
        currentFolderPath &&
        folderGroups.some((g) => g.folder_path === currentFolderPath)
      ) {
        exportScope = 'current';
      } else {
        exportScope = 'all';
      }
      selectedFolderPaths = new Set(folderGroups.map((g) => g.folder_path));
    }
  });

  const currentGroup = $derived(
    currentFolderPath
      ? folderGroups.find((g) => g.folder_path === currentFolderPath)
      : null,
  );

  const totalPhotoCount = $derived(
    folderGroups.reduce((acc, g) => acc + g.count, 0),
  );

  const selectedPhotos = $derived.by(() => {
    if (exportScope === 'current' && currentGroup) {
      return currentGroup.photos.map((p) => p.path);
    }
    if (exportScope === 'all') {
      return folderGroups.flatMap((g) => g.photos.map((p) => p.path));
    }
    return folderGroups
      .filter((g) => selectedFolderPaths.has(g.folder_path))
      .flatMap((g) => g.photos.map((p) => p.path));
  });

  function toggleFolder(path: string) {
    const next = new Set(selectedFolderPaths);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    selectedFolderPaths = next;
  }

  function handleModeClick(mode: 'copy' | 'move') {
    onExport(mode, selectedPhotos, preserveFolderStructure);
  }
</script>

<Dialog bind:open onOpenChange={(v) => !v && onCancel()}>
  <DialogContent class="sm:max-w-[500px]">
    <DialogHeader>
      <DialogTitle class="flex items-center gap-2">
        <Download class="h-5 w-5 text-primary" />
        Export Favourites
      </DialogTitle>
      <DialogDescription>
        Choose which folder(s) to export and configure your output options.
      </DialogDescription>
    </DialogHeader>

    {#if isExporting}
      <div class="flex flex-col items-center justify-center space-y-4 py-8">
        <div
          class="h-8 w-8 animate-spin rounded-full border-b-2 border-primary"
        ></div>
        <p class="text-sm font-medium text-muted-foreground">{exportStatus}</p>
      </div>
    {:else}
      <div class="space-y-4 py-2">
        <!-- Scope Selection -->
        <div class="space-y-2">
          <Label
            class="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
          >
            Export Source
          </Label>
          <div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
            {#if currentGroup}
              <button
                type="button"
                class={`flex flex-col items-start rounded-lg border p-2.5 text-left text-xs transition-all ${
                  exportScope === 'current'
                    ? 'border-primary bg-primary/10 font-medium text-primary ring-1 ring-primary'
                    : 'bg-card text-muted-foreground hover:bg-accent'
                }`}
                onclick={() => (exportScope = 'current')}
              >
                <span class="truncate font-semibold text-foreground"
                  >Current Folder</span
                >
                <span class="mt-1 text-[11px] opacity-80">
                  {currentGroup.count} photo{currentGroup.count === 1
                    ? ''
                    : 's'}
                </span>
              </button>
            {/if}

            <button
              type="button"
              class={`flex flex-col items-start rounded-lg border p-2.5 text-left text-xs transition-all ${
                exportScope === 'all'
                  ? 'border-primary bg-primary/10 font-medium text-primary ring-1 ring-primary'
                  : 'bg-card text-muted-foreground hover:bg-accent'
              } ${!currentGroup ? 'sm:col-span-2' : ''}`}
              onclick={() => (exportScope = 'all')}
            >
              <span class="font-semibold text-foreground">All Folders</span>
              <span class="mt-1 text-[11px] opacity-80">
                {totalPhotoCount} photo{totalPhotoCount === 1 ? '' : 's'} across
                {folderGroups.length} folder{folderGroups.length === 1
                  ? ''
                  : 's'}
              </span>
            </button>

            <button
              type="button"
              class={`flex flex-col items-start rounded-lg border p-2.5 text-left text-xs transition-all ${
                exportScope === 'custom'
                  ? 'border-primary bg-primary/10 font-medium text-primary ring-1 ring-primary'
                  : 'bg-card text-muted-foreground hover:bg-accent'
              }`}
              onclick={() => (exportScope = 'custom')}
            >
              <span class="font-semibold text-foreground">Select Folders</span>
              <span class="mt-1 text-[11px] opacity-80">
                {selectedFolderPaths.size} of {folderGroups.length} selected
              </span>
            </button>
          </div>
        </div>

        <!-- Custom Folder Checklist (when custom scope is active) -->
        {#if exportScope === 'custom'}
          <div
            class="max-h-40 space-y-1 overflow-y-auto rounded-lg border bg-muted/20 p-2"
          >
            {#each folderGroups as group (group.folder_path)}
              {@const isSelected = selectedFolderPaths.has(group.folder_path)}
              <button
                type="button"
                class="flex w-full items-center justify-between rounded px-2.5 py-1.5 text-left text-xs transition-colors hover:bg-muted"
                onclick={() => toggleFolder(group.folder_path)}
              >
                <div class="flex min-w-0 items-center gap-2">
                  <div
                    class={`flex h-4 w-4 shrink-0 items-center justify-center rounded border ${
                      isSelected
                        ? 'border-primary bg-primary text-primary-foreground'
                        : 'border-muted-foreground/40 bg-background'
                    }`}
                  >
                    {#if isSelected}
                      <Check size={12} strokeWidth={3} />
                    {/if}
                  </div>
                  <span class="truncate font-medium text-foreground">
                    {group.folder_name}
                  </span>
                </div>
                <span
                  class="ml-2 shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground"
                >
                  {group.count}
                </span>
              </button>
            {/each}
          </div>
        {/if}

        <!-- Output Settings -->
        <div class="rounded-lg border bg-muted/30 p-3">
          <div class="flex items-center justify-between gap-3">
            <div class="space-y-0.5">
              <Label class="text-xs font-medium text-foreground">
                Preserve Folder Structure
              </Label>
              <p class="text-[11px] text-muted-foreground">
                Creates subfolders in destination to prevent filename conflicts
              </p>
            </div>
            <Switch
              checked={preserveFolderStructure}
              onCheckedChange={(v) => (preserveFolderStructure = v)}
            />
          </div>
        </div>

        <!-- Summary & Action Buttons -->
        <div class="pt-2 text-xs font-medium text-foreground">
          Ready to export <span
            class="rounded bg-primary/10 px-1.5 py-0.5 font-bold text-primary"
            >{selectedPhotos.length}</span
          >
          photo{selectedPhotos.length === 1 ? '' : 's'}:
        </div>

        <div class="grid gap-2.5">
          <Button
            variant="outline"
            class="h-auto w-full flex-col items-start gap-1 p-3 text-left hover:bg-muted/50"
            disabled={selectedPhotos.length === 0}
            onclick={() => handleModeClick('copy')}
          >
            <div class="flex items-center gap-2 font-semibold">
              <Copy class="h-4 w-4 text-primary" />
              Copy to Folder
            </div>
            <p class="text-xs font-normal text-muted-foreground">
              Keeps original files safe. Creates copies in your chosen export
              folder.
            </p>
          </Button>

          <Button
            variant="outline"
            class="h-auto w-full flex-col items-start gap-1 p-3 text-left transition-colors hover:border-destructive/30 hover:bg-destructive/5 hover:text-destructive"
            disabled={selectedPhotos.length === 0}
            onclick={() => handleModeClick('move')}
          >
            <div class="flex items-center gap-2 font-semibold">
              <FolderOutput class="h-4 w-4 text-destructive" />
              Move to Folder
            </div>
            <p class="text-xs font-normal text-muted-foreground">
              Physically relocates files to the export folder.
            </p>
          </Button>
        </div>
      </div>
    {/if}

    <DialogFooter>
      <Button variant="ghost" onclick={onCancel} disabled={isExporting}>
        Cancel
      </Button>
    </DialogFooter>
  </DialogContent>
</Dialog>

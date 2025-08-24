<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { Loader2, FolderOpen, X } from '@lucide/svelte/icons';

  let {
    open = $bindable(false),
    isLoading = false,
    onCancel = () => {},
    onSelectFolder = () => {}
  }: {
    open: boolean;
    isLoading?: boolean;
    onCancel?: () => void;
    onSelectFolder?: () => void;
  } = $props();

  function handleCancel() {
    if (!isLoading) {
      open = false;
      onCancel();
    }
  }

  function handleSelectFolder() {
    onSelectFolder();
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="sm:max-w-md">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2">
        <FolderOpen class="h-5 w-5" />
        {isLoading ? 'Processing Images...' : 'Select Folder'}
      </Dialog.Title>
    </Dialog.Header>
    
    <div class="flex flex-col items-center gap-6 py-6">
      {#if isLoading}
        <div class="flex flex-col items-center gap-4">
          <Loader2 class="h-12 w-12 animate-spin text-primary" />
          <div class="text-center">
            <p class="text-sm text-muted-foreground">
              Scanning folder for images...
            </p>
            <p class="text-xs text-muted-foreground mt-1">
              This may take a moment for large folders
            </p>
          </div>
        </div>
      {:else}
        <div class="text-center">
          <FolderOpen class="h-16 w-16 mx-auto text-muted-foreground mb-4" />
          <p class="text-sm text-muted-foreground">
            Choose a folder to load images from
          </p>
        </div>
      {/if}
    </div>

    <div class="flex justify-end gap-2">
      {#if !isLoading}
        <Button variant="outline" onclick={handleCancel}>
          Cancel
        </Button>
        <Button onclick={handleSelectFolder}>
          <FolderOpen class="h-4 w-4 mr-2" />
          Select Folder
        </Button>
      {:else}
        <Button variant="outline" onclick={handleCancel} disabled={isLoading}>
          <X class="h-4 w-4 mr-2" />
          Cancel
        </Button>
      {/if}
    </div>
  </Dialog.Content>
</Dialog.Root>

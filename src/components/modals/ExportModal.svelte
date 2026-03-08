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
  import Download from '@lucide/svelte/icons/download';
  import FolderOutput from '@lucide/svelte/icons/folder-output';
  import Copy from '@lucide/svelte/icons/copy';

  export let open = false;
  export let onExport: (mode: 'copy' | 'move') => void;
  export let onCancel: () => void;
  export let isExporting = false;
  export let exportStatus = '';
</script>

<Dialog bind:open onOpenChange={(v) => !v && onCancel()}>
  <DialogContent class="sm:max-w-[425px]">
    <DialogHeader>
      <DialogTitle class="flex items-center gap-2">
        <Download class="h-5 w-5 text-primary" />
        Export Favourites
      </DialogTitle>
      <DialogDescription>
        How would you like to export your selected photos? Note that moving
        files will remove them from your current gallery folders.
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
      <div class="grid gap-4 py-4">
        <Button
          variant="outline"
          class="h-auto w-full flex-col items-start gap-2 whitespace-normal p-4 text-left hover:bg-muted/50"
          onclick={() => onExport('copy')}
        >
          <div class="flex items-center gap-2 font-semibold">
            <Copy class="h-4 w-4" />
            Copy to Folder
          </div>
          <p class="text-sm font-normal text-muted-foreground">
            Creates a copy of your photos in the new destination. Your originals
            stay exactly where they are.
          </p>
        </Button>

        <Button
          variant="outline"
          class="h-auto w-full flex-col items-start gap-2 whitespace-normal p-4 text-left transition-colors hover:border-destructive/30 hover:bg-destructive/5 hover:text-destructive"
          onclick={() => onExport('move')}
        >
          <div class="flex items-center gap-2 font-semibold">
            <FolderOutput class="h-4 w-4" />
            Move to Folder
          </div>
          <p class="text-sm font-normal text-muted-foreground opacity-90">
            Physically moves the files to the new destination. They will no
            longer appear in their original location.
          </p>
        </Button>
      </div>
    {/if}

    <DialogFooter>
      <Button variant="ghost" onclick={onCancel} disabled={isExporting}>
        Cancel
      </Button>
    </DialogFooter>
  </DialogContent>
</Dialog>

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { recentFolders, isLoading } from '../stores/galleryStore';
  import { loadPhotos, loadPath } from './common/ImageOperations';
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

  const handleOpenFolder = () => {
    loadPhotos();
  };

  const handleRecentClick = (path: string) => {
    loadPath(path);
  };
</script>

<div class="flex h-full flex-col items-center justify-center p-4">
  <div class="w-full max-w-md text-center">
    <!-- Centerpiece -->
    <div class="mb-8 flex flex-col items-center">
      <div
        class="mb-6 flex h-24 w-24 items-center justify-center rounded-3xl bg-primary/10 text-primary"
      >
        <FolderOpenIcon size={48} strokeWidth={1.5} />
      </div>
      <h1 class="text-3xl font-bold tracking-tight text-foreground">
        Ready to Cull?
      </h1>
      <p class="mt-3 leading-relaxed text-muted-foreground">
        Select a folder containing your latest photo shoot to start organizing
        and selecting your best shots.
      </p>
    </div>

    <!-- Primary Action -->
    <Button
      onclick={handleOpenFolder}
      size="lg"
      class="h-12 px-8 text-base font-medium transition-all hover:scale-105 active:scale-95"
      disabled={$isLoading}
    >
      <PlusIcon size={20} class="mr-2" />
      Open Folder
    </Button>

    <!-- Recent History -->
    {#if $recentFolders.length > 0}
      <div class="mt-16 w-full text-left">
        <div
          class="mb-4 flex items-center gap-2 text-sm font-semibold uppercase tracking-wider text-muted-foreground/70"
        >
          <HistoryIcon size={16} />
          <span>Recent Sessions</span>
        </div>

        <div class="space-y-2">
          {#each $recentFolders as path}
            <button
              onclick={() => handleRecentClick(path)}
              class="group flex w-full items-center justify-between rounded-xl border border-transparent bg-secondary/50 p-4 transition-all hover:border-border hover:bg-secondary active:bg-secondary/80 disabled:opacity-50"
              disabled={$isLoading}
            >
              <div class="flex flex-col items-start overflow-hidden pr-4">
                <span
                  class="max-w-full truncate text-sm font-medium text-foreground"
                >
                  {path.split('/').pop()}
                </span>
                <span class="max-w-full truncate text-xs text-muted-foreground">
                  {path}
                </span>
              </div>
              <ChevronRightIcon
                size={18}
                class="text-muted-foreground transition-transform group-hover:translate-x-1"
              />
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

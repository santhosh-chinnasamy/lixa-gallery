<script lang="ts">
  import {
    folderTree,
    currentFolder,
    expandedFolders,
  } from '../stores/galleryStore';
  import { loadSubfolder } from './common/ImageOperations';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Folder from '@lucide/svelte/icons/folder';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import { cn } from '$lib/utils';
  import type { FolderNode } from '../stores/galleryStore';

  const { inSidebar = false }: { inSidebar?: boolean } = $props();

  function toggleExpand(path: string, event: MouseEvent) {
    event.stopPropagation();
    expandedFolders.toggle(path);
  }

  function handleFolderClick(path: string) {
    loadSubfolder(path);
  }
</script>

{#snippet FolderItem(node: FolderNode, depth: number)}
  {@const isExpanded = $expandedFolders.has(node.path)}
  {@const isActive = $currentFolder === node.path}
  {@const hasChildren = node.children && node.children.length > 0}

  <div class="flex flex-col">
    <div
      class={cn(
        'group flex w-full items-center gap-1 rounded-md px-1 py-0.5 transition-colors',
        isActive
          ? 'bg-primary/10 font-medium text-primary'
          : 'text-muted-foreground hover:bg-accent',
      )}
      style={`padding-left: ${depth * 12 + 4}px`}
    >
      {#if hasChildren}
        <button
          onclick={(e) => toggleExpand(node.path, e)}
          class="shrink-0 rounded p-0.5 opacity-70 transition-opacity hover:bg-black/5 group-hover:opacity-100 dark:hover:bg-white/5"
          aria-label={isExpanded ? 'Collapse' : 'Expand'}
        >
          <ChevronRight
            size={14}
            class={cn(
              'transition-transform duration-200',
              isExpanded && 'rotate-90',
            )}
          />
        </button>
      {:else}
        <div class="w-[22px] shrink-0"></div>
      {/if}

      <button
        onclick={() => handleFolderClick(node.path)}
        class="flex min-w-0 flex-grow items-center gap-2 py-1 text-left text-sm"
      >
        {#if isActive || isExpanded}
          <FolderOpen
            size={16}
            class={cn(
              'shrink-0',
              isActive ? 'text-primary' : 'text-muted-foreground/70',
            )}
          />
        {:else}
          <Folder size={16} class="shrink-0 text-muted-foreground/70" />
        {/if}
        <span class="truncate">{node.name}</span>
      </button>
    </div>

    {#if isExpanded && hasChildren}
      <div class="ml-[10px] flex flex-col border-l border-border/40">
        {#each node.children as child (child.path)}
          {@render FolderItem(child, depth + 1)}
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<div class="custom-scrollbar flex w-full select-none flex-col pb-20">
  <div class="px-1 py-2">
    {#if $folderTree}
      {@render FolderItem($folderTree, 0)}
    {:else}
      <div class="px-4 py-4 text-center">
        <p class="text-xs italic text-muted-foreground">No folder loaded</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
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

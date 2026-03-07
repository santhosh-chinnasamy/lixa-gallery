<script lang="ts">
  import { currentFolder, folderTree } from '../stores/galleryStore';
  import { loadSubfolder } from './common/ImageOperations';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import House from '@lucide/svelte/icons/house';

  interface Crumb {
    name: string;
    path: string;
  }

  // Calculate crumbs based on currentFolder and folderTree
  let crumbs = $derived.by(() => {
    if (!$currentFolder || !$folderTree) return [];

    const rootPath = $folderTree.path;
    const currentPath = $currentFolder;

    if (!currentPath.startsWith(rootPath)) return [];

    const relativePath = currentPath.slice(rootPath.length);
    const parts = relativePath.split('/').filter((p) => p !== '');

    let result: Crumb[] = [{ name: $folderTree.name, path: $folderTree.path }];
    let currentBuildingPath = rootPath;

    for (const part of parts) {
      if (!currentBuildingPath.endsWith('/')) currentBuildingPath += '/';
      currentBuildingPath += part;
      result.push({ name: part, path: currentBuildingPath });
    }

    return result;
  });

  function handleCrumbClick(path: string) {
    loadSubfolder(path);
  }
</script>

<nav
  class="scrollbar-hide flex items-center gap-1 overflow-x-auto whitespace-nowrap px-4 py-2 text-sm text-muted-foreground"
>
  <button
    onclick={() => handleCrumbClick($folderTree?.path || '')}
    class="rounded-md p-1 transition-colors hover:text-foreground"
    title="Root"
  >
    <House size={16} />
  </button>

  {#each crumbs as crumb, i}
    <ChevronRight size={14} class="shrink-0 text-muted-foreground/40" />
    <button
      onclick={() => handleCrumbClick(crumb.path)}
      class={`rounded-md px-1.5 py-1 transition-colors hover:bg-accent hover:text-accent-foreground ${i === crumbs.length - 1 ? 'pointer-events-none font-medium text-foreground' : ''}`}
    >
      {crumb.name}
    </button>
  {/each}
</nav>

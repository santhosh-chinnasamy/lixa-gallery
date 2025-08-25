<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { PhotoMetadata } from '../types/photo';

  export let items: PhotoMetadata[];
  export let itemHeight = 200; // Estimated height per row
  export let containerHeight = 600;
  export let itemsPerRow = 6; // Grid columns
  export let overscan = 3; // Extra rows to render for smooth scrolling

  let scrollContainer: HTMLElement;
  let scrollTop = 0;
  let containerWidth = 0;
  
  $: totalRows = Math.ceil(items.length / itemsPerRow);
  $: rowHeight = itemHeight + 20; // Include gap
  $: totalHeight = totalRows * rowHeight;
  
  // Calculate visible range
  $: startRow = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
  $: endRow = Math.min(totalRows, Math.ceil((scrollTop + containerHeight) / rowHeight) + overscan);
  $: visibleRows = endRow - startRow;
  
  // Get visible items
  $: visibleItems = items.slice(
    startRow * itemsPerRow,
    Math.min(endRow * itemsPerRow, items.length)
  );
  
  $: offsetY = startRow * rowHeight;

  function handleScroll() {
    if (scrollContainer) {
      scrollTop = scrollContainer.scrollTop;
    }
  }

  onMount(() => {
    if (scrollContainer) {
      scrollContainer.addEventListener('scroll', handleScroll, { passive: true });
    }
  });

  onDestroy(() => {
    if (scrollContainer) {
      scrollContainer.removeEventListener('scroll', handleScroll);
    }
  });
</script>

<div 
  bind:this={scrollContainer}
  bind:clientWidth={containerWidth}
  class="h-full overflow-auto scroll-smooth"
  style="height: {containerHeight}px;"
>
  <div style="height: {totalHeight}px; position: relative;">
    <div 
      style="transform: translateY({offsetY}px); position: absolute; width: 100%;"
      class="grid gap-4 p-4"
      style:grid-template-columns="repeat({itemsPerRow}, minmax(0, 1fr))"
    >
      {#each visibleItems as item, index}
        <div class="aspect-square">
          <slot {item} index={startRow * itemsPerRow + index} />
        </div>
      {/each}
    </div>
  </div>
</div>

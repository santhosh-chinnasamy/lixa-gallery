<script lang="ts">
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Switch } from '$lib/components/ui/switch';
  import { Slider } from '$lib/components/ui/slider';
  import * as Select from '$lib/components/ui/select';
  import Search from '@lucide/svelte/icons/search';
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import { Button } from '$lib/components/ui/button';
  import Breadcrumbs from './Breadcrumbs.svelte';
  import {
    searchQuery,
    showImagesOnly,
    sortBy,
    sortOrder,
    gridSize,
    type SortOption,
  } from '../stores/galleryStore';

  const { onLoadPhotos }: { onLoadPhotos?: () => void } = $props();

  const sortOptions: { value: SortOption; label: string }[] = [
    { value: 'name', label: 'Name' },
    { value: 'date', label: 'Date Created' },
    { value: 'size', label: 'File Size' },
  ];

  // Derive simple properties for select components
  let selectedSortOption = $derived(
    sortOptions.find((opt) => opt.value === $sortBy) || sortOptions[0],
  );

  function handleSearchInput(e: Event) {
    searchQuery.set((e.target as HTMLInputElement).value);
  }

  function toggleSortOrder() {
    sortOrder.update((current) => (current === 'asc' ? 'desc' : 'asc'));
  }
</script>

<div
  class="sticky top-0 z-10 flex flex-col gap-3 border-b bg-background/80 px-4 py-3 backdrop-blur-md lg:px-6"
>
  <!-- Top Row: Navigation and Primary Actions -->
  <div class="flex items-center justify-between">
    <Breadcrumbs />
    {#if onLoadPhotos}
      <Button
        onclick={onLoadPhotos}
        variant="outline"
        size="sm"
        class="h-8 gap-2 bg-background text-xs font-medium"
      >
        <FolderOpen size={14} />
        <span>Open Folder</span>
      </Button>
    {/if}
  </div>

  <!-- Bottom Row: Search, Filters, and Sorting -->
  <div
    class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between"
  >
    <!-- Search & Filter Area -->
    <div class="flex min-w-[200px] max-w-md flex-1 items-center gap-4">
      <div class="relative flex-1">
        <Search
          class="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground transition-colors focus-within:text-primary"
        />
        <Input
          type="search"
          placeholder="Search files..."
          class="h-9 w-full bg-background/50 pl-9 transition-colors focus-visible:bg-background"
          value={$searchQuery}
          oninput={handleSearchInput}
        />
      </div>

      <!-- <div
        class="flex shrink-0 items-center space-x-2 rounded-md border bg-background/50 px-3 py-1.5 shadow-sm"
      >
        <Switch
          id="images-only"
          checked={$showImagesOnly}
          onCheckedChange={(checked: boolean) => showImagesOnly.set(checked)}
        />
        <Label for="images-only" class="cursor-pointer text-xs font-medium"
          >Images Only</Label
        >
      </div> -->
    </div>

    <!-- Sorting & Sizing Controls -->
    <div class="flex items-center gap-4 overflow-x-auto pb-1 sm:pb-0">
      <div class="flex items-center gap-2 border-none">
        <Label
          class="whitespace-nowrap text-xs font-medium text-muted-foreground"
          >Sort</Label
        >
        <Select.Root
          type="single"
          value={$sortBy}
          onValueChange={(v) => {
            if (v) sortBy.set(v as SortOption);
          }}
        >
          <Select.Trigger
            class="h-9 w-full min-w-[110px] bg-background/50 focus:bg-background"
            aria-label="Sort by"
          >
            {selectedSortOption.label}
          </Select.Trigger>
          <Select.Content>
            {#each sortOptions as option}
              <Select.Item value={option.value}>{option.label}</Select.Item>
            {/each}
          </Select.Content>
        </Select.Root>

        <Button
          variant="outline"
          size="icon"
          class="h-9 w-9 shrink-0 bg-background/50 focus-visible:bg-background"
          onclick={toggleSortOrder}
          aria-label={`Sort ${$sortOrder === 'asc' ? 'Ascending' : 'Descending'}`}
        >
          {#if $sortOrder === 'asc'}
            <ArrowUp class="h-4 w-4 text-muted-foreground" />
          {:else}
            <ArrowDown class="h-4 w-4 text-muted-foreground" />
          {/if}
        </Button>
      </div>

      <div class="hidden h-6 w-px shrink-0 bg-border sm:block"></div>

      <div class="flex min-w-[120px] max-w-[150px] items-center gap-3">
        <Label class="shrink-0 text-xs font-medium text-muted-foreground"
          >Size</Label
        >
        <Slider
          type="single"
          value={$gridSize}
          max={400}
          min={100}
          step={10}
          class="w-full cursor-ew-resize"
          onValueChange={(v) => gridSize.set(v)}
        />
      </div>
    </div>
  </div>
</div>

<script lang="ts">
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Switch } from '$lib/components/ui/switch';
  import { Slider } from '$lib/components/ui/slider';
  import * as Select from '$lib/components/ui/select';
  import * as Sidebar from '$lib/components/ui/sidebar';
  import Search from '@lucide/svelte/icons/search';
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import { Button } from '$lib/components/ui/button';
  import Breadcrumbs from './Breadcrumbs.svelte';
  import {
    searchQuery,
    sortBy,
    sortOrder,
    gridSize,
    loadingMode,
    cullFilter,
    autoAdvance,
    type SortOption,
    type LoadingMode,
    type CullFilter,
  } from '../stores/galleryStore';
  import { loadPhotos } from './common/ImageOperations';
  import Zap from '@lucide/svelte/icons/zap';
  import Clock from '@lucide/svelte/icons/clock';
  import Play from '@lucide/svelte/icons/play';

  let {
    onStartSlideshow,
    canStartSlideshow = false,
    photoCount = 0,
    pickedCount = 0,
  }: {
    onStartSlideshow?: () => void;
    canStartSlideshow?: boolean;
    photoCount?: number;
    pickedCount?: number;
  } = $props();

  const filterOptions: { value: CullFilter; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'picked', label: 'Picked' },
    { value: 'rejected', label: 'Rejected' },
    { value: 'unflagged', label: 'Unflagged' },
  ];

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

  function toggleLoadingMode() {
    loadingMode.set($loadingMode === 'lazy' ? 'sync' : 'lazy');
  }
</script>

<div
  class="sticky top-0 z-10 flex flex-col gap-3 border-b bg-background/80 px-4 py-3 backdrop-blur-md lg:px-6"
>
  <!-- Top Row: Navigation and Primary Actions -->
  <div class="flex items-center justify-between gap-3">
    <div class="flex items-center gap-2 overflow-hidden">
      <Sidebar.Trigger class="-ml-2" />
      <Breadcrumbs />
      {#if photoCount > 0}
        <span
          class="shrink-0 text-xs tabular-nums text-muted-foreground"
          title="Picked in this folder"
        >
          {pickedCount} picked / {photoCount}
        </span>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      {#if canStartSlideshow}
        <Button
          onclick={onStartSlideshow}
          variant="outline"
          size="sm"
          class="h-8 shrink-0 gap-1.5 bg-background text-xs font-medium transition-colors hover:border-primary/50 hover:text-primary"
          title="Start slideshow (S)"
        >
          <Play size={13} class="fill-current" />
          <span class="hidden sm:inline">Slideshow</span>
        </Button>
      {/if}
      <Button
        onclick={() => loadPhotos()}
        variant="outline"
        size="sm"
        class="h-8 shrink-0 gap-2 bg-background text-xs font-medium"
      >
        <FolderOpen size={14} />
        <span class="hidden sm:inline">Open Folder</span>
      </Button>
    </div>
  </div>

  <!-- Bottom Row: Search, Filters, and Sorting -->
  <div class="flex flex-wrap items-center justify-between gap-4">
    <!-- Performance & Search Area -->
    <div class="flex min-w-[120px] flex-1 flex-wrap items-center gap-3">
      <!-- Performance Toggle -->
      <Button
        variant="outline"
        size="icon"
        class={`h-9 w-9 shrink-0 transition-all duration-200 ${
          $loadingMode === 'lazy'
            ? 'border-blue-200 bg-blue-50 text-blue-600 hover:bg-blue-100 dark:border-blue-800 dark:bg-blue-900/20 dark:text-blue-400'
            : 'bg-background/50 text-muted-foreground'
        }`}
        onclick={toggleLoadingMode}
        title={`Performance Mode: ${$loadingMode === 'lazy' ? 'Lazy (Instant Load)' : 'Sync (Wait for all)'}`}
      >
        {#if $loadingMode === 'lazy'}
          <Zap class="h-4 w-4 fill-current" />
        {:else}
          <Clock class="h-4 w-4" />
        {/if}
      </Button>

      <div
        class="flex items-center rounded-md border bg-background/50 p-0.5"
        role="group"
        aria-label="Cull filter"
      >
        {#each filterOptions as option (option.value)}
          <button
            type="button"
            class={`rounded px-2 py-1 text-xs font-medium transition-colors ${
              $cullFilter === option.value
                ? 'bg-primary/10 text-primary'
                : 'text-muted-foreground hover:text-foreground'
            }`}
            aria-pressed={$cullFilter === option.value}
            onclick={() => cullFilter.set(option.value)}
          >
            {option.label}
          </button>
        {/each}
      </div>

      <button
        type="button"
        class={`h-9 shrink-0 rounded-md border px-2.5 text-xs font-medium transition-colors ${
          $autoAdvance
            ? 'border-primary/30 bg-primary/10 text-primary'
            : 'bg-background/50 text-muted-foreground hover:text-foreground'
        }`}
        aria-pressed={$autoAdvance}
        title="After P, X, or U, move to the next photo"
        onclick={() => autoAdvance.set(!$autoAdvance)}
      >
        Advance
      </button>

      <div class="relative w-full max-w-md">
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
    </div>

    <!-- Sorting & Sizing Controls -->
    <div class="flex flex-wrap items-center gap-4">
      <!-- Sorting Group -->
      <div class="flex shrink-0 items-center gap-2">
        <Label
          class="hidden whitespace-nowrap text-xs font-medium text-muted-foreground sm:block"
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
            class="h-9 w-[110px] bg-background/50 focus:bg-background"
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

      <div class="hidden h-6 w-px shrink-0 bg-border md:block"></div>

      <!-- Sizing Group -->
      <div class="flex min-w-[80px] max-w-[150px] flex-1 items-center gap-3">
        <Label
          class="hidden shrink-0 text-xs font-medium text-muted-foreground sm:block"
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

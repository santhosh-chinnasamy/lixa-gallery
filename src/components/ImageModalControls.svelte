<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { cn } from '$lib/utils';
  import { fade } from 'svelte/transition';
  import ArrowLeft from './icons/ArrowLeft.svelte';
  import ArrowRight from './icons/ArrowRight.svelte';
  import Heart from './icons/Heart.svelte';
  import ZoomIn from '@lucide/svelte/icons/zoom-in';
  import ZoomOut from '@lucide/svelte/icons/zoom-out';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Info from '@lucide/svelte/icons/info';

  let {
    currentIndex,
    totalPhotos,
    canShowPrevious,
    canShowNext,
    onPrevious,
    onNext,
    zoomScale,
    onZoomIn,
    onZoomOut,
    onResetZoom,
    activeFilter,
    onFilterChange,
    onResetAll,
    isFavourite,
    onToggleFavorite,
    showInfo,
    onToggleInfo,
    showUI = true,
  }: {
    currentIndex: number;
    totalPhotos: number;
    canShowPrevious: boolean;
    canShowNext: boolean;
    onPrevious: () => void;
    onNext: () => void;
    zoomScale: number;
    onZoomIn: () => void;
    onZoomOut: () => void;
    onResetZoom: () => void;
    activeFilter: 'none' | 'grayscale' | 'sepia' | 'invert';
    onFilterChange: (filter: 'none' | 'grayscale' | 'sepia' | 'invert') => void;
    onResetAll: () => void;
    isFavourite: boolean;
    onToggleFavorite: () => void;
    showInfo: boolean;
    onToggleInfo: () => void;
    showUI?: boolean;
  } = $props();

  const filters = [
    { id: 'none', label: 'Normal' },
    { id: 'grayscale', label: 'B&W' },
    { id: 'sepia', label: 'Sepia' },
    { id: 'invert', label: 'Invert' },
  ] as const;
</script>

<div
  class={`mt-2 flex shrink-0 flex-col items-center justify-center transition-opacity duration-300 ${
    showUI ? 'opacity-100' : 'pointer-events-none opacity-0'
  }`}
>
  <div
    class="flex flex-wrap items-center justify-center gap-1.5 rounded-full border border-white/10 bg-black/65 px-3 py-1.5 text-white shadow-2xl backdrop-blur-xl sm:gap-2"
  >
    <!-- Previous image button -->
    <Button
      variant="ghost"
      size="icon"
      disabled={!canShowPrevious}
      onclick={onPrevious}
      class={cn(
        'h-9 w-9 rounded-full text-white transition-all hover:bg-white/15 disabled:opacity-30',
        !canShowPrevious && 'cursor-not-allowed',
      )}
      aria-label="Previous image (Left Arrow)"
    >
      <ArrowLeft />
    </Button>

    <!-- Image counter -->
    <div
      class="select-none px-2 text-xs font-medium tabular-nums text-white/90 sm:text-sm"
    >
      {currentIndex + 1} / {totalPhotos}
    </div>

    <!-- Next image button -->
    <Button
      variant="ghost"
      size="icon"
      disabled={!canShowNext}
      onclick={onNext}
      class={cn(
        'h-9 w-9 rounded-full text-white transition-all hover:bg-white/15 disabled:opacity-30',
        !canShowNext && 'cursor-not-allowed',
      )}
      aria-label="Next image (Right Arrow)"
    >
      <ArrowRight />
    </Button>

    <div class="h-5 w-px bg-white/20"></div>

    <!-- Zoom Out button -->
    <Button
      variant="ghost"
      size="icon"
      disabled={zoomScale <= 1.0}
      onclick={onZoomOut}
      class="h-9 w-9 rounded-full text-white transition-all hover:bg-white/15 disabled:opacity-30"
      aria-label="Zoom out (Ctrl -)"
    >
      <ZoomOut size={16} />
    </Button>

    <!-- Zoom percentage (click to reset zoom) -->
    <button
      type="button"
      onclick={onResetZoom}
      title="Click to reset zoom (Ctrl 0)"
      class="min-w-[42px] cursor-pointer select-none px-1 text-center text-xs font-semibold tabular-nums text-white/90 transition-colors hover:text-white"
    >
      {Math.round(zoomScale * 100)}%
    </button>

    <!-- Zoom In button -->
    <Button
      variant="ghost"
      size="icon"
      disabled={zoomScale >= 5.0}
      onclick={onZoomIn}
      class="h-9 w-9 rounded-full text-white transition-all hover:bg-white/15 disabled:opacity-30"
      aria-label="Zoom in (Ctrl +)"
    >
      <ZoomIn size={16} />
    </Button>

    <div class="h-5 w-px bg-white/20"></div>

    <!-- Segmented Filter Pills -->
    <div class="flex items-center rounded-full bg-white/10 p-0.5">
      {#each filters as f}
        <button
          type="button"
          onclick={() => onFilterChange(f.id)}
          class={`rounded-full px-2.5 py-1 text-xs font-medium transition-all ${
            activeFilter === f.id
              ? 'bg-white font-semibold text-black shadow-sm'
              : 'text-white/70 hover:text-white'
          }`}
          aria-pressed={activeFilter === f.id}
        >
          {f.label}
        </button>
      {/each}
    </div>

    <!-- Conditional Reset Button -->
    {#if zoomScale > 1.0 || activeFilter !== 'none'}
      <div
        class="flex items-center gap-1.5 sm:gap-2"
        in:fade={{ duration: 150 }}
      >
        <div class="h-5 w-px bg-white/20"></div>
        <Button
          variant="ghost"
          size="sm"
          onclick={onResetAll}
          class="h-8 gap-1.5 rounded-full bg-amber-500/20 px-2.5 text-xs font-semibold text-amber-300 transition-all hover:bg-amber-500/30 hover:text-amber-200"
          aria-label="Reset zoom and filters"
        >
          <RotateCcw size={13} />
          <span>Reset</span>
        </Button>
      </div>
    {/if}

    <div class="h-5 w-px bg-white/20"></div>

    <!-- Favorite toggle button -->
    <Button
      variant="ghost"
      size="icon"
      onclick={onToggleFavorite}
      class={cn(
        'h-9 w-9 rounded-full transition-all duration-200',
        isFavourite
          ? 'bg-red-500/20 text-red-400 hover:bg-red-500/30'
          : 'text-white/80 hover:bg-white/15 hover:text-red-400',
      )}
      aria-label={isFavourite
        ? 'Remove from favorites (L)'
        : 'Add to favorites (L)'}
      aria-pressed={isFavourite}
    >
      <Heart {isFavourite} />
    </Button>

    <!-- Info toggle button -->
    <Button
      variant="ghost"
      size="icon"
      onclick={onToggleInfo}
      class={cn(
        'h-9 w-9 rounded-full transition-all duration-200',
        showInfo
          ? 'bg-white/25 text-white'
          : 'text-white/80 hover:bg-white/15 hover:text-white',
      )}
      aria-label="Toggle info panel (I)"
      aria-pressed={showInfo}
    >
      <Info size={16} />
    </Button>
  </div>
</div>

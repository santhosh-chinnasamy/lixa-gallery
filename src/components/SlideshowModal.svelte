<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { cn } from '$lib/utils';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { untrack } from 'svelte';
  import { fade } from 'svelte/transition';
  import { favorites } from '../stores/galleryStore';
  import Heart from '@lucide/svelte/icons/heart';
  import Play from '@lucide/svelte/icons/play';
  import Pause from '@lucide/svelte/icons/pause';
  import SkipForward from '@lucide/svelte/icons/skip-forward';
  import SkipBack from '@lucide/svelte/icons/skip-back';
  import Shuffle from '@lucide/svelte/icons/shuffle';
  import Repeat from '@lucide/svelte/icons/repeat';
  import Maximize from '@lucide/svelte/icons/maximize';
  import Minimize from '@lucide/svelte/icons/minimize';
  import Info from '@lucide/svelte/icons/info';
  import X from '@lucide/svelte/icons/x';
  import type { PhotoMetadata } from '../types/photo';

  let {
    photos = [],
    startIndex = 0,
    open = $bindable(false),
    onClose,
  }: {
    photos: PhotoMetadata[];
    startIndex?: number;
    open: boolean;
    onClose?: () => void;
  } = $props();

  // Playback configuration
  const intervalOptions = [2, 3, 5, 8, 10];
  let intervalSeconds = $state(4);
  let isPlaying = $state(true);
  let isShuffle = $state(false);
  let isLoop = $state(true);
  let isFullscreen = $state(false);
  let showHUD = $state(true);
  let showInfo = $state(false);
  let isHoveringHUD = $state(false);

  // Playback state
  let currentIndex = $state(0);
  let shuffleOrder = $state<number[]>([]);
  let shuffleIndex = $state(0);
  let progress = $state(0); // 0 to 100%

  // Double-buffering for butter-smooth crossfading
  // Slot 0 and Slot 1 alternate as active and incoming
  let activeSlot = $state<0 | 1>(0);
  let slot0Photo = $state<PhotoMetadata | null>(null);
  let slot1Photo = $state<PhotoMetadata | null>(null);

  // Preloading cache
  const preloadedPaths = new Set<string>();

  // Timers
  let progressTimer: ReturnType<typeof setInterval> | null = null;
  let hudTimeoutRef: ReturnType<typeof setTimeout> | null = null;

  // Derive current photo
  const currentPhoto = $derived.by(() => {
    if (photos.length === 0) return null;
    if (isShuffle && shuffleOrder.length === photos.length) {
      const idx = shuffleOrder[shuffleIndex] ?? 0;
      return photos[idx] ?? null;
    }
    return photos[currentIndex] ?? null;
  });

  const isFavourite = $derived(
    currentPhoto ? $favorites.has(currentPhoto.path) : false,
  );

  const displayIndex = $derived(
    isShuffle ? shuffleIndex + 1 : currentIndex + 1,
  );

  // Shuffle array helper (Fisher-Yates)
  function createShuffledIndices(length: number, currentIdx: number): number[] {
    const indices = Array.from({ length }, (_, i) => i);
    for (let i = indices.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [indices[i], indices[j]] = [indices[j], indices[i]];
    }
    // Ensure current item stays first in shuffle
    const foundPos = indices.indexOf(currentIdx);
    if (foundPos !== -1 && foundPos !== 0) {
      [indices[0], indices[foundPos]] = [indices[foundPos], indices[0]];
    }
    return indices;
  }

  // Preload next upcoming photos
  function preloadUpcoming(startPos: number) {
    if (photos.length <= 1) return;
    const toPreload: string[] = [];

    if (isShuffle) {
      for (let offset = 1; offset <= 3; offset++) {
        const nextPos = (startPos + offset) % shuffleOrder.length;
        const photoIdx = shuffleOrder[nextPos];
        if (photoIdx !== undefined && photos[photoIdx]) {
          toPreload.push(photos[photoIdx].path);
        }
      }
    } else {
      for (let offset = 1; offset <= 3; offset++) {
        const nextPos = (startPos + offset) % photos.length;
        if (photos[nextPos]) {
          toPreload.push(photos[nextPos].path);
        }
      }
    }

    toPreload.forEach((path) => {
      if (!preloadedPaths.has(path)) {
        preloadedPaths.add(path);
        const img = new Image();
        img.src = convertFileSrc(path);
      }
    });
  }

  // Transition to a photo with crossfade
  function transitionToPhoto(photo: PhotoMetadata | null) {
    if (!photo) return;
    progress = 0;

    const nextSlot: 0 | 1 = activeSlot === 0 ? 1 : 0;
    if (nextSlot === 1) {
      slot1Photo = photo;
    } else {
      slot0Photo = photo;
    }

    // Flip active slot to trigger CSS transition
    activeSlot = nextSlot;

    // Trigger preload for upcoming slides
    if (isShuffle) {
      preloadUpcoming(shuffleIndex);
    } else {
      preloadUpcoming(currentIndex);
    }
  }

  function nextSlide() {
    if (photos.length <= 1) return;
    progress = 0;

    if (isShuffle) {
      if (shuffleIndex < shuffleOrder.length - 1) {
        shuffleIndex++;
      } else if (isLoop) {
        // Reshuffle and loop back
        const curr = shuffleOrder[shuffleIndex] ?? 0;
        shuffleOrder = createShuffledIndices(photos.length, curr);
        shuffleIndex = 0;
      } else {
        isPlaying = false;
        return;
      }
      const nextIdx = shuffleOrder[shuffleIndex] ?? 0;
      transitionToPhoto(photos[nextIdx]);
    } else {
      if (currentIndex < photos.length - 1) {
        currentIndex++;
      } else if (isLoop) {
        currentIndex = 0;
      } else {
        isPlaying = false;
        return;
      }
      transitionToPhoto(photos[currentIndex]);
    }
  }

  function prevSlide() {
    if (photos.length <= 1) return;
    progress = 0;

    if (isShuffle) {
      if (shuffleIndex > 0) {
        shuffleIndex--;
        const prevIdx = shuffleOrder[shuffleIndex] ?? 0;
        transitionToPhoto(photos[prevIdx]);
      } else if (isLoop) {
        shuffleIndex = shuffleOrder.length - 1;
        const prevIdx = shuffleOrder[shuffleIndex] ?? 0;
        transitionToPhoto(photos[prevIdx]);
      }
    } else {
      if (currentIndex > 0) {
        currentIndex--;
      } else if (isLoop) {
        currentIndex = photos.length - 1;
      }
      transitionToPhoto(photos[currentIndex]);
    }
  }

  function togglePlayPause() {
    isPlaying = !isPlaying;
    if (isPlaying) {
      resetHudTimer();
    }
  }

  function toggleShuffle() {
    if (!isShuffle) {
      // Turn on shuffle
      shuffleOrder = createShuffledIndices(photos.length, currentIndex);
      shuffleIndex = 0;
      isShuffle = true;
    } else {
      // Turn off shuffle, sync currentIndex to the photo currently displayed
      if (currentPhoto) {
        const found = photos.findIndex((p) => p.path === currentPhoto.path);
        if (found !== -1) currentIndex = found;
      }
      isShuffle = false;
    }
  }

  function toggleLoop() {
    isLoop = !isLoop;
  }

  function toggleFavorite() {
    if (currentPhoto) {
      favorites.toggle(currentPhoto.path);
    }
  }

  async function toggleFullscreen() {
    try {
      const appWindow = getCurrentWindow();
      const current = await appWindow.isFullscreen();
      await appWindow.setFullscreen(!current);
      isFullscreen = !current;
    } catch (err) {
      console.warn('Unable to toggle fullscreen via Tauri window API:', err);
      // Fallback to web fullscreen API
      if (!document.fullscreenElement) {
        await document.documentElement.requestFullscreen().catch(() => {});
        isFullscreen = true;
      } else {
        await document.exitFullscreen().catch(() => {});
        isFullscreen = false;
      }
    }
  }

  async function exitFullscreenIfActive() {
    try {
      const appWindow = getCurrentWindow();
      const current = await appWindow.isFullscreen();
      if (current) {
        await appWindow.setFullscreen(false);
      }
    } catch {
      if (document.fullscreenElement) {
        await document.exitFullscreen().catch(() => {});
      }
    }
    isFullscreen = false;
  }

  function handleClose() {
    exitFullscreenIfActive();
    open = false;
    onClose?.();
  }

  function resetHudTimer() {
    showHUD = true;
    if (hudTimeoutRef) clearTimeout(hudTimeoutRef);

    // Only auto-hide if playing and not hovering controls
    if (isPlaying && !isHoveringHUD) {
      hudTimeoutRef = setTimeout(() => {
        if (!isHoveringHUD) {
          showHUD = false;
        }
      }, 2500);
    }
  }

  function handleMouseMove() {
    resetHudTimer();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!open) return;

    if (event.key === ' ' || event.code === 'Space') {
      event.preventDefault();
      togglePlayPause();
      return;
    }
    if (event.key === 'ArrowRight' || event.key === 'PageDown') {
      event.preventDefault();
      nextSlide();
      return;
    }
    if (event.key === 'ArrowLeft' || event.key === 'PageUp') {
      event.preventDefault();
      prevSlide();
      return;
    }
    if (
      event.key === 'f' ||
      event.key === 'F' ||
      event.key === 'l' ||
      event.key === 'L'
    ) {
      event.preventDefault();
      toggleFavorite();
      return;
    }
    if (event.key === 's' || event.key === 'S') {
      event.preventDefault();
      toggleShuffle();
      return;
    }
    if (event.key === 'r' || event.key === 'R') {
      event.preventDefault();
      toggleLoop();
      return;
    }
    if (event.key === 'i' || event.key === 'I') {
      event.preventDefault();
      showInfo = !showInfo;
      return;
    }
    if (event.key === 'F11' || event.key === 'Enter') {
      event.preventDefault();
      toggleFullscreen();
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      handleClose();
      return;
    }
  }

  // Format bytes for info overlay
  function formatBytes(bytes: number, decimals = 1) {
    if (!+bytes) return '0 B';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
  }

  // Guard to ensure initialization only runs once when modal opens
  let wasOpen = false;

  // Initialize when opened
  $effect(() => {
    if (open && !wasOpen) {
      wasOpen = true;
      untrack(() => {
        if (photos.length > 0) {
          currentIndex = Math.max(0, Math.min(startIndex, photos.length - 1));
          slot0Photo = photos[currentIndex] ?? null;
          slot1Photo = null;
          activeSlot = 0;
          progress = 0;
          isPlaying = true;
          isShuffle = false;
          showInfo = false;
          preloadUpcoming(currentIndex);
          resetHudTimer();

          getCurrentWindow()
            .isFullscreen()
            .then((f) => (isFullscreen = f))
            .catch(() => {});
        }
      });
    } else if (!open && wasOpen) {
      wasOpen = false;
      untrack(() => {
        if (progressTimer) {
          clearInterval(progressTimer);
          progressTimer = null;
        }
        if (hudTimeoutRef) {
          clearTimeout(hudTimeoutRef);
          hudTimeoutRef = null;
        }
      });
    }
  });

  // Playback timer ticker (every 50ms)
  $effect(() => {
    if (progressTimer) {
      clearInterval(progressTimer);
      progressTimer = null;
    }

    if (open && isPlaying && !isHoveringHUD && photos.length > 1) {
      const stepMs = 50;
      const totalMs = intervalSeconds * 1000;
      const stepIncrement = (stepMs / totalMs) * 100;

      progressTimer = setInterval(() => {
        progress += stepIncrement;
        if (progress >= 100) {
          progress = 0;
          nextSlide();
        }
      }, stepMs);
    } else {
      progress = 0;
    }

    return () => {
      if (progressTimer) {
        clearInterval(progressTimer);
        progressTimer = null;
      }
    };
  });
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open && photos.length > 0}
  <div
    class={cn(
      'fixed inset-0 z-50 flex select-none flex-col justify-between overflow-hidden bg-black transition-colors duration-500',
      !showHUD && 'cursor-none',
    )}
    role="dialog"
    aria-modal="true"
    aria-label="Photo Slideshow"
    tabindex="-1"
    onmousemove={handleMouseMove}
    in:fade={{ duration: 250 }}
    out:fade={{ duration: 200 }}
  >
    <!-- Top Countdown Progress Line -->
    <div
      class="absolute left-0 right-0 top-0 z-30 h-1 overflow-hidden bg-white/10"
    >
      <div
        class="h-full bg-primary/80 transition-all duration-75 ease-linear"
        style="width: {progress}%;"
      ></div>
    </div>

    <!-- Top Bar: Navigation Info, Speed & Controls -->
    <header
      class={cn(
        'absolute left-0 right-0 top-0 z-20 flex items-center justify-between bg-gradient-to-b from-black/80 via-black/40 to-transparent p-4 transition-opacity duration-300 sm:p-6',
        showHUD ? 'opacity-100' : 'pointer-events-none opacity-0',
      )}
    >
      <div class="flex items-center gap-3">
        <!-- Slide Counter -->
        <span
          class="rounded-full bg-white/15 px-3 py-1 font-mono text-xs font-semibold text-white/90 shadow-sm backdrop-blur-md"
        >
          {displayIndex} / {photos.length}
        </span>

        <!-- Filename & Details -->
        {#if currentPhoto}
          <div class="hidden max-w-md truncate sm:block">
            <p class="truncate text-sm font-medium text-white/90">
              {currentPhoto.metadata.name}
            </p>
          </div>
        {/if}
      </div>

      <div class="flex items-center gap-2">
        <!-- Info Overlay Toggle -->
        <Button
          variant="ghost"
          size="icon"
          onclick={() => (showInfo = !showInfo)}
          class={cn(
            'h-9 w-9 rounded-full text-white/80 hover:bg-white/20 hover:text-white',
            showInfo && 'bg-white/20 text-white',
          )}
          aria-label="Toggle Image Information (I)"
          title="Toggle Image Info (I)"
        >
          <Info size={18} />
        </Button>

        <!-- Fullscreen Toggle -->
        <Button
          variant="ghost"
          size="icon"
          onclick={toggleFullscreen}
          class="h-9 w-9 rounded-full text-white/80 hover:bg-white/20 hover:text-white"
          aria-label="Toggle Fullscreen (F11)"
          title="Toggle Fullscreen (F11)"
        >
          {#if isFullscreen}
            <Minimize size={18} />
          {:else}
            <Maximize size={18} />
          {/if}
        </Button>

        <!-- Exit Slideshow Button -->
        <Button
          variant="ghost"
          size="icon"
          onclick={handleClose}
          class="h-9 w-9 rounded-full bg-white/10 text-white hover:bg-red-500/80 hover:text-white"
          aria-label="Close Slideshow (Esc)"
          title="Close Slideshow (Esc)"
        >
          <X size={18} />
        </Button>
      </div>
    </header>

    <!-- Main Image Display (Double-buffered for smooth cross-fades) -->
    <main
      class="relative flex h-full w-full flex-1 items-center justify-center overflow-hidden"
    >
      <!-- Click left/right halves to navigate -->
      <button
        type="button"
        class="absolute bottom-0 left-0 top-0 z-10 w-1/4 cursor-w-resize opacity-0 focus:outline-none"
        onclick={prevSlide}
        aria-label="Previous image"
      ></button>
      <button
        type="button"
        class="absolute bottom-0 right-0 top-0 z-10 w-1/4 cursor-e-resize opacity-0 focus:outline-none"
        onclick={nextSlide}
        aria-label="Next image"
      ></button>

      <!-- Slot 0 Image -->
      {#if slot0Photo}
        <img
          src={convertFileSrc(slot0Photo.path)}
          alt={slot0Photo.metadata.name}
          class={cn(
            'absolute inset-0 m-auto max-h-full max-w-full object-contain transition-opacity duration-500 ease-in-out',
            activeSlot === 0
              ? 'z-10 opacity-100'
              : 'pointer-events-none z-0 opacity-0',
          )}
          draggable="false"
        />
      {/if}

      <!-- Slot 1 Image -->
      {#if slot1Photo}
        <img
          src={convertFileSrc(slot1Photo.path)}
          alt={slot1Photo.metadata.name}
          class={cn(
            'absolute inset-0 m-auto max-h-full max-w-full object-contain transition-opacity duration-500 ease-in-out',
            activeSlot === 1
              ? 'z-10 opacity-100'
              : 'pointer-events-none z-0 opacity-0',
          )}
          draggable="false"
        />
      {/if}

      <!-- Detailed Info Overlay Card -->
      {#if showInfo && currentPhoto}
        <div
          class="absolute bottom-24 left-6 z-20 max-w-sm rounded-xl border border-white/15 bg-black/75 p-4 text-white shadow-2xl backdrop-blur-xl"
          in:fade={{ duration: 150 }}
          out:fade={{ duration: 100 }}
        >
          <div
            class="mb-2 flex items-center justify-between border-b border-white/10 pb-2"
          >
            <span
              class="text-xs font-semibold uppercase tracking-wider text-white/60"
            >
              Photo Info
            </span>
            <span class="text-[11px] capitalize text-white/50">
              {currentPhoto.metadata.name.split('.').pop() || 'Image'}
            </span>
          </div>
          <div class="space-y-1.5 text-xs">
            <p class="truncate font-medium text-white/90">
              {currentPhoto.metadata.name}
            </p>
            <div class="flex justify-between text-white/70">
              <span>Size:</span>
              <span class="font-mono"
                >{formatBytes(currentPhoto.metadata.size)}</span
              >
            </div>
            <div class="flex justify-between text-white/70">
              <span>Modified:</span>
              <span>
                {new Intl.DateTimeFormat(undefined, {
                  dateStyle: 'medium',
                  timeStyle: 'short',
                }).format(new Date(currentPhoto.metadata.modified * 1000))}
              </span>
            </div>
            <div class="truncate pt-1 text-[11px] text-white/40">
              {currentPhoto.path}
            </div>
          </div>
        </div>
      {/if}
    </main>

    <!-- Floating Bottom HUD Bar -->
    <footer
      class={cn(
        'absolute bottom-6 left-0 right-0 z-20 flex justify-center px-4 transition-opacity duration-300',
        showHUD ? 'opacity-100' : 'pointer-events-none opacity-0',
      )}
      onmouseenter={() => (isHoveringHUD = true)}
      onmouseleave={() => {
        isHoveringHUD = false;
        resetHudTimer();
      }}
      role="region"
      aria-label="Slideshow Controls"
    >
      <div
        class="flex flex-wrap items-center justify-center gap-1.5 rounded-full border border-white/15 bg-black/75 px-4 py-2 text-white shadow-2xl backdrop-blur-2xl sm:gap-2"
      >
        <!-- Previous Button -->
        <Button
          variant="ghost"
          size="icon"
          onclick={prevSlide}
          class="h-9 w-9 rounded-full text-white/80 hover:bg-white/20 hover:text-white"
          aria-label="Previous Slide (Left Arrow)"
          title="Previous Slide (←)"
        >
          <SkipBack size={17} />
        </Button>

        <!-- Play / Pause Button -->
        <Button
          variant="ghost"
          size="icon"
          onclick={togglePlayPause}
          class="h-10 w-10 rounded-full bg-white/20 text-white transition-transform hover:scale-105 hover:bg-white/30"
          aria-label={isPlaying
            ? 'Pause Slideshow (Space)'
            : 'Play Slideshow (Space)'}
          title={isPlaying ? 'Pause (Space)' : 'Play (Space)'}
        >
          {#if isPlaying}
            <Pause size={20} class="fill-current" />
          {:else}
            <Play size={20} class="ml-0.5 fill-current" />
          {/if}
        </Button>

        <!-- Next Button -->
        <Button
          variant="ghost"
          size="icon"
          onclick={nextSlide}
          class="h-9 w-9 rounded-full text-white/80 hover:bg-white/20 hover:text-white"
          aria-label="Next Slide (Right Arrow)"
          title="Next Slide (→)"
        >
          <SkipForward size={17} />
        </Button>

        <div class="mx-1 h-5 w-px bg-white/20"></div>

        <!-- Speed / Interval Selector -->
        <div class="flex items-center rounded-full bg-white/10 p-0.5">
          {#each intervalOptions as sec}
            <button
              type="button"
              onclick={() => {
                intervalSeconds = sec;
                progress = 0;
              }}
              class={cn(
                'rounded-full px-2 py-0.5 text-xs font-semibold transition-all',
                intervalSeconds === sec
                  ? 'shadow-xs bg-white font-bold text-black'
                  : 'text-white/70 hover:text-white',
              )}
              title={`Slide duration: ${sec} seconds`}
            >
              {sec}s
            </button>
          {/each}
        </div>

        <div class="mx-1 h-5 w-px bg-white/20"></div>

        <!-- Shuffle Button -->
        <Button
          variant="ghost"
          size="icon"
          onclick={toggleShuffle}
          class={cn(
            'h-9 w-9 rounded-full transition-all',
            isShuffle
              ? 'bg-primary/30 text-primary hover:bg-primary/40'
              : 'text-white/70 hover:bg-white/20 hover:text-white',
          )}
          aria-label="Toggle Shuffle (S)"
          title={isShuffle ? 'Shuffle Active (S)' : 'Shuffle Disabled (S)'}
          aria-pressed={isShuffle}
        >
          <Shuffle size={16} />
        </Button>

        <!-- Loop / Repeat Button -->
        <Button
          variant="ghost"
          size="icon"
          onclick={toggleLoop}
          class={cn(
            'h-9 w-9 rounded-full transition-all',
            isLoop
              ? 'bg-primary/30 text-primary hover:bg-primary/40'
              : 'text-white/70 hover:bg-white/20 hover:text-white',
          )}
          aria-label="Toggle Loop (R)"
          title={isLoop ? 'Loop Active (R)' : 'Loop Disabled (R)'}
          aria-pressed={isLoop}
        >
          <Repeat size={16} />
        </Button>

        <div class="mx-1 h-5 w-px bg-white/20"></div>

        <!-- Favorite Button -->
        <Button
          variant="ghost"
          size="icon"
          onclick={toggleFavorite}
          class={cn(
            'h-9 w-9 rounded-full transition-all',
            isFavourite
              ? 'bg-red-500/25 text-red-400 hover:bg-red-500/35'
              : 'text-white/70 hover:bg-white/20 hover:text-red-400',
          )}
          aria-label={isFavourite
            ? 'Remove from favorites (F/L)'
            : 'Add to favorites (F/L)'}
          title={isFavourite ? 'Favorite (F/L)' : 'Mark as Favorite (F/L)'}
          aria-pressed={isFavourite}
        >
          <Heart size={16} class={isFavourite ? 'fill-current' : ''} />
        </Button>
      </div>
    </footer>
  </div>
{/if}

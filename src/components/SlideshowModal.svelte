<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { cn } from '$lib/utils';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { untrack, onDestroy } from 'svelte';
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
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
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
  let isImageLoaded = $state(false);

  // Timing references (monotonic wall-clock)
  let rafId: number | null = null;
  let startTime = 0;
  let elapsedBeforePause = 0;
  let hudTimeoutRef: ReturnType<typeof setTimeout> | null = null;
  let loadSafetyTimeout: ReturnType<typeof setTimeout> | null = null;

  // Preloading cache
  const preloadedPaths = new Set<string>();

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
  function createShuffledIndices(length: number, forceFirstIdx = -1): number[] {
    const indices = Array.from({ length }, (_, i) => i);
    for (let i = indices.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [indices[i], indices[j]] = [indices[j], indices[i]];
    }
    if (forceFirstIdx >= 0) {
      const foundPos = indices.indexOf(forceFirstIdx);
      if (foundPos !== -1 && foundPos !== 0) {
        [indices[0], indices[foundPos]] = [indices[foundPos], indices[0]];
      }
    }
    return indices;
  }

  // Preload upcoming photos
  function preloadUpcoming(startPos: number) {
    if (photos.length <= 1) return;
    const toPreload: string[] = [];

    if (isShuffle && shuffleOrder.length === photos.length) {
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

  // Animation frame ticker (monotonic wall-clock)
  function tickTimer() {
    if (
      !open ||
      !isPlaying ||
      isHoveringHUD ||
      !isImageLoaded ||
      photos.length <= 1
    ) {
      rafId = null;
      return;
    }

    const durationMs = intervalSeconds * 1000;
    const elapsed = performance.now() - startTime + elapsedBeforePause;
    progress = Math.min(100, (elapsed / durationMs) * 100);

    if (elapsed >= durationMs) {
      progress = 0;
      elapsedBeforePause = 0;
      nextSlide();
      return;
    }

    rafId = requestAnimationFrame(tickTimer);
  }

  function startAnimationLoop() {
    if (rafId) cancelAnimationFrame(rafId);
    if (
      open &&
      isPlaying &&
      !isHoveringHUD &&
      isImageLoaded &&
      photos.length > 1
    ) {
      rafId = requestAnimationFrame(tickTimer);
    }
  }

  function pauseTimer() {
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
    if (isImageLoaded) {
      elapsedBeforePause += performance.now() - startTime;
    }
  }

  function prepareImageLoad() {
    isImageLoaded = false;
    progress = 0;
    elapsedBeforePause = 0;
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
    if (loadSafetyTimeout) {
      clearTimeout(loadSafetyTimeout);
    }
    // Safety fallback: if onload doesn't fire within 800ms, start countdown anyway
    loadSafetyTimeout = setTimeout(() => {
      if (!isImageLoaded && open) {
        handleImageLoad();
      }
    }, 800);
  }

  function handleImageLoad() {
    if (loadSafetyTimeout) {
      clearTimeout(loadSafetyTimeout);
      loadSafetyTimeout = null;
    }
    if (!isImageLoaded) {
      isImageLoaded = true;
      startTime = performance.now();
      elapsedBeforePause = 0;
      startAnimationLoop();
    }
  }

  function handleImageError() {
    if (loadSafetyTimeout) {
      clearTimeout(loadSafetyTimeout);
      loadSafetyTimeout = null;
    }
    isImageLoaded = true;
    startTime = performance.now();
    elapsedBeforePause = 0;
    startAnimationLoop();
  }

  function nextSlide() {
    if (photos.length <= 1) return;
    prepareImageLoad();

    if (isShuffle) {
      if (shuffleIndex < shuffleOrder.length - 1) {
        shuffleIndex++;
      } else if (isLoop) {
        // Reshuffle loop: ensure first photo of new cycle is never the last one displayed
        const curr = shuffleOrder[shuffleIndex] ?? 0;
        const newOrder = createShuffledIndices(photos.length, -1);
        if (newOrder[0] === curr && newOrder.length > 1) {
          [newOrder[0], newOrder[1]] = [newOrder[1], newOrder[0]];
        }
        shuffleOrder = newOrder;
        shuffleIndex = 0;
      } else {
        isPlaying = false;
        return;
      }
      preloadUpcoming(shuffleIndex);
    } else {
      if (currentIndex < photos.length - 1) {
        currentIndex++;
      } else if (isLoop) {
        currentIndex = 0;
      } else {
        isPlaying = false;
        return;
      }
      preloadUpcoming(currentIndex);
    }
  }

  function prevSlide() {
    if (photos.length <= 1) return;
    prepareImageLoad();

    if (isShuffle) {
      if (shuffleIndex > 0) {
        shuffleIndex--;
      } else if (isLoop) {
        shuffleIndex = shuffleOrder.length - 1;
      }
      preloadUpcoming(shuffleIndex);
    } else {
      if (currentIndex > 0) {
        currentIndex--;
      } else if (isLoop) {
        currentIndex = photos.length - 1;
      }
      preloadUpcoming(currentIndex);
    }
  }

  function togglePlayPause() {
    isPlaying = !isPlaying;
    if (isPlaying) {
      startTime = performance.now();
      startAnimationLoop();
      resetHudTimer();
    } else {
      pauseTimer();
    }
  }

  function toggleShuffle() {
    if (!isShuffle) {
      shuffleOrder = createShuffledIndices(photos.length, currentIndex);
      shuffleIndex = 0;
      isShuffle = true;
      preloadUpcoming(0);
    } else {
      if (currentPhoto) {
        const found = photos.findIndex((p) => p.path === currentPhoto.path);
        if (found !== -1) currentIndex = found;
      }
      isShuffle = false;
      preloadUpcoming(currentIndex);
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
          isShuffle = false;
          shuffleOrder = [];
          shuffleIndex = 0;
          isPlaying = true;
          showInfo = false;
          prepareImageLoad();
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
        if (rafId) {
          cancelAnimationFrame(rafId);
          rafId = null;
        }
        if (hudTimeoutRef) {
          clearTimeout(hudTimeoutRef);
          hudTimeoutRef = null;
        }
        if (loadSafetyTimeout) {
          clearTimeout(loadSafetyTimeout);
          loadSafetyTimeout = null;
        }
        progress = 0;
        elapsedBeforePause = 0;
        isImageLoaded = false;
      });
    }
  });

  onDestroy(() => {
    if (rafId) cancelAnimationFrame(rafId);
    if (hudTimeoutRef) clearTimeout(hudTimeoutRef);
    if (loadSafetyTimeout) clearTimeout(loadSafetyTimeout);
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
        class="h-full bg-primary/80 transition-none"
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

    <!-- Main Presentation Viewport -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <main
      class={cn(
        'relative flex h-full w-full flex-1 items-center justify-center overflow-hidden',
        showHUD ? 'cursor-pointer' : 'cursor-none',
      )}
      onclick={(e) => {
        if (
          e.target === e.currentTarget ||
          (e.target as HTMLElement).tagName === 'IMG'
        ) {
          togglePlayPause();
        }
      }}
    >
      <!-- Edge Navigation Chevrons -->
      <button
        type="button"
        class={cn(
          'absolute left-4 top-1/2 z-20 -translate-y-1/2 rounded-full border border-white/10 bg-black/50 p-3 text-white/80 backdrop-blur-md transition-all duration-300 hover:scale-110 hover:bg-black/80 hover:text-white',
          showHUD ? 'opacity-100' : 'pointer-events-none opacity-0',
        )}
        onmouseenter={() => pauseTimer()}
        onmouseleave={() => {
          if (isPlaying) {
            startTime = performance.now();
            startAnimationLoop();
          }
        }}
        onclick={(e) => {
          e.stopPropagation();
          prevSlide();
        }}
        aria-label="Previous Slide"
        title="Previous Slide (←)"
      >
        <ChevronLeft size={28} />
      </button>

      <button
        type="button"
        class={cn(
          'absolute right-4 top-1/2 z-20 -translate-y-1/2 rounded-full border border-white/10 bg-black/50 p-3 text-white/80 backdrop-blur-md transition-all duration-300 hover:scale-110 hover:bg-black/80 hover:text-white',
          showHUD ? 'opacity-100' : 'pointer-events-none opacity-0',
        )}
        onmouseenter={() => pauseTimer()}
        onmouseleave={() => {
          if (isPlaying) {
            startTime = performance.now();
            startAnimationLoop();
          }
        }}
        onclick={(e) => {
          e.stopPropagation();
          nextSlide();
        }}
        aria-label="Next Slide"
        title="Next Slide (→)"
      >
        <ChevronRight size={28} />
      </button>

      <!-- Keyed Image Crossfade: completely eliminates WebKit texture reuse ghosting -->
      {#if currentPhoto}
        {#key currentPhoto.path}
          <img
            src={convertFileSrc(currentPhoto.path)}
            alt={currentPhoto.metadata.name}
            onload={handleImageLoad}
            onerror={handleImageError}
            in:fade={{ duration: 400 }}
            out:fade={{ duration: 400 }}
            class="absolute inset-0 m-auto max-h-full max-w-full select-none object-contain"
            draggable="false"
          />
        {/key}
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
      onmouseenter={() => {
        isHoveringHUD = true;
        pauseTimer();
      }}
      onmouseleave={() => {
        isHoveringHUD = false;
        if (isPlaying) {
          startTime = performance.now();
          startAnimationLoop();
        }
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
                elapsedBeforePause = 0;
                startTime = performance.now();
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

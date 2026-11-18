<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import { gridImageUrl, UNSUPPORTED_FORMAT_MESSAGE } from '$lib/imageSource';
  import { probe, probeWarn } from '$lib/debugProbe';
  import { webviewSafeImageSrc } from '$lib/protocolImage';
  import { cullFlags, gridFit } from '../stores/galleryStore';
  import Filename from './Filename.svelte';
  import Heart from '@lucide/svelte/icons/heart';
  import X from '@lucide/svelte/icons/x';
  import type { PhotoMetadata } from '../types/photo';
  import type { CullFlag } from '../lib/cull';

  let {
    photo,
    tabindex = 0,
    isActive = false,
    galleryIndex = 0,
    handleImageClick,
    isVisible = false,
  }: {
    photo: PhotoMetadata;
    tabindex?: number;
    isActive?: boolean;
    galleryIndex?: number;
    handleImageClick: (photo: PhotoMetadata) => void;
    isVisible?: boolean;
  } = $props();

  let flag = $derived<CullFlag>($cullFlags.get(photo.path) ?? 'unflagged');
  let isPicked = $derived(flag === 'picked');
  let isRejected = $derived(flag === 'rejected');
  let fileName = $derived(photo.metadata.name);
  let showWholeFrame = $derived($gridFit === 'fit');

  // Decoded WebP thumbnail, or the original HEIC on macOS. Null means this
  // platform cannot show the file, so the card does not request it.
  let imageSrc = $derived(gridImageUrl(photo));

  let displaySrc = $state<string | null>(null);
  let imageLoaded = $state(false);
  let imageError = $state(false);

  // Bridge custom-protocol URLs through blob: under http:// Vite origins
  // (WebKitGTK 2.54.1 blocks lixa-thumbnail:// on <img>).
  $effect(() => {
    const src = imageSrc;
    imageLoaded = false;
    imageError = false;
    displaySrc = null;

    if (!src) return;

    let cancelled = false;
    let revoke = () => {};

    void (async () => {
      try {
        const safe = await webviewSafeImageSrc(src);
        if (cancelled) {
          safe.revoke();
          return;
        }
        revoke = safe.revoke;
        displaySrc = safe.src;
        probe('thumb.ready', {
          name: fileName,
          bridged: safe.src.startsWith('blob:'),
        });
      } catch (error) {
        if (cancelled) return;
        imageError = true;
        probeWarn('thumb.fail', {
          name: fileName,
          err: error instanceof Error ? error.message : String(error),
        });
      }
    })();

    return () => {
      cancelled = true;
      revoke();
    };
  });

  const handleClick = () => handleImageClick(photo);

  const onBadgeClick = (event?: Event) => {
    if (event) event.stopPropagation();
    cullFlags.apply(photo.path, flag === 'rejected' ? 'unflag' : 'pick');
  };

  function handleImageLoad() {
    imageLoaded = true;
    imageError = false;
  }

  function handleImageError() {
    imageError = true;
    imageLoaded = false;
    probeWarn('thumb.img.onerror', {
      name: fileName,
      src: displaySrc?.slice(0, 80) ?? null,
    });
  }
</script>

<Card.Root
  class={`group relative cursor-pointer transition-colors duration-150 hover:border-foreground/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background ${
    isActive ? 'ring-2 ring-primary ring-offset-2 ring-offset-background' : ''
  }`}
  {tabindex}
  data-gallery-index={galleryIndex}
  role="gridcell"
  aria-selected={isActive}
  aria-label={fileName}
  onclick={handleClick}
>
  <Card.Content class="relative overflow-hidden rounded-lg p-0">
    <!-- Image container with consistent aspect ratio -->
    <div
      class={`relative aspect-[1/1] h-auto w-full overflow-hidden ${
        showWholeFrame ? 'bg-neutral-300 dark:bg-neutral-800' : 'bg-muted'
      }`}
    >
      {#if imageSrc === null}
        <div
          class="flex h-full w-full items-center justify-center bg-muted px-3 text-center"
        >
          <div class="text-xs text-muted-foreground">
            <div>{UNSUPPORTED_FORMAT_MESSAGE}</div>
            <div class="mt-1 max-w-full truncate">{fileName}</div>
          </div>
        </div>
      {:else}
        <!-- Loading placeholder -->
        {#if !imageLoaded && !imageError}
          <div
            class="flex h-full w-full animate-pulse items-center justify-center bg-muted"
          >
            <div class="text-sm text-muted-foreground">Loading...</div>
          </div>
        {/if}

        <!-- Error placeholder -->
        {#if imageError}
          <div
            class="flex h-full w-full items-center justify-center bg-muted"
          >
            <div class="p-2 text-center text-xs text-muted-foreground">
              <div>Failed to load</div>
              <div class="max-w-full truncate px-2">
                {fileName}
              </div>
            </div>
          </div>
        {/if}

        <!-- Actual image -->
        {#if displaySrc}
          <img
            src={displaySrc}
            alt={fileName}
            loading={isVisible ? 'eager' : 'lazy'}
            class={`h-full w-full transition-opacity duration-300 ${
              showWholeFrame ? 'object-contain' : 'object-cover'
            } ${imageLoaded ? 'opacity-100' : 'opacity-0'}`}
            onload={handleImageLoad}
            onerror={handleImageError}
          />
        {/if}
        {#if isRejected}
          <div
            class="pointer-events-none absolute inset-0 bg-background/55"
          ></div>
        {/if}
      {/if}
    </div>

    <button
      class={`absolute right-2 top-2 rounded-full p-2 shadow-md backdrop-blur-sm transition-opacity duration-200 ${
        isPicked
          ? 'bg-background/90 text-red-600'
          : isRejected
            ? 'bg-background/90 text-foreground'
            : 'bg-background/80 text-muted-foreground opacity-0 hover:text-red-600 group-hover:opacity-100'
      }`}
      onclick={onBadgeClick}
      aria-label={isPicked
        ? 'Unflag pick'
        : isRejected
          ? 'Unflag reject'
          : 'Pick'}
      aria-pressed={isPicked}
    >
      {#if isRejected && !isPicked}
        <X size={16} />
      {:else}
        <Heart size={16} class={isPicked ? 'fill-current' : ''} />
      {/if}
    </button>

    <div
      class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/60 to-transparent p-3 opacity-0 transition-opacity duration-200 group-hover:opacity-100 group-focus:opacity-100"
    >
      <Filename name={fileName} />
    </div>
  </Card.Content>
</Card.Root>

<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import { UNSUPPORTED_FORMAT_MESSAGE } from '$lib/imageSource';
  import { ensureThumbnailPath } from '$lib/ensureThumbQueue';
  import { gridThumbUrl } from '$lib/gridThumbUrl';
  import { pageIsHttpOrigin } from '$lib/protocolImage';
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
    isVisible = true,
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

  // Sync URL from disk path. Override only after a one-shot ensure.
  // Under http, never fall through to protocol URLs (gridThumbUrl returns null).
  let ensuredSrc = $state<string | null>(null);
  let ensureAttempted = $state(false);
  let imageLoaded = $state(false);
  let imageError = $state(false);
  let imageSrc = $derived(ensuredSrc ?? gridThumbUrl(photo));
  // Http + no disk webp yet: stay on Loading until ensure finishes (or errors).
  let awaitingHttpThumb = $derived(
    pageIsHttpOrigin() && imageSrc === null && !imageError,
  );

  function applyEnsuredThumb(thumbPath: string) {
    if (!thumbPath || !/\.webp$/i.test(thumbPath)) {
      imageError = true;
      imageLoaded = false;
      return;
    }
    ensuredSrc = `/__lixa_thumb__?p=${encodeURIComponent(thumbPath)}`;
    imageLoaded = false;
    imageError = false;
  }

  $effect(() => {
    const path = photo.path;
    void photo.thumbnail_path;
    ensuredSrc = null;
    ensureAttempted = false;
    imageLoaded = false;
    imageError = false;

    // Under Vite http: empty thumb → keep Loading and materialize once.
    // Do not emit protocol URLs or invoke→blob on the grid.
    if (!pageIsHttpOrigin()) return;
    if (gridThumbUrl(photo) !== null) return;

    let cancelled = false;
    ensureAttempted = true;
    void ensureThumbnailPath(path)
      .then((thumbPath) => {
        if (cancelled) return;
        applyEnsuredThumb(thumbPath);
      })
      .catch(() => {
        if (!cancelled) {
          imageError = true;
          imageLoaded = false;
        }
      });

    return () => {
      cancelled = true;
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
    // Stale DB path (legacy .thumbnails/NAME.webp). Rematerialize once and
    // point at the new file via same-origin middleware — never invoke→blob.
    if (!ensureAttempted && pageIsHttpOrigin()) {
      ensureAttempted = true;
      void ensureThumbnailPath(photo.path)
        .then(applyEnsuredThumb)
        .catch(() => {
          imageError = true;
          imageLoaded = false;
        });
      return;
    }
    imageError = true;
    imageLoaded = false;
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
    <div
      class={`relative aspect-[1/1] h-auto w-full overflow-hidden ${
        showWholeFrame ? 'bg-neutral-300 dark:bg-neutral-800' : 'bg-muted'
      }`}
    >
      {#if awaitingHttpThumb}
        <div
          class="flex h-full w-full animate-pulse items-center justify-center bg-muted"
        >
          <div class="text-sm text-muted-foreground">Loading...</div>
        </div>
      {:else if imageSrc === null && imageError}
        <div class="flex h-full w-full items-center justify-center bg-muted">
          <div class="p-2 text-center text-xs text-muted-foreground">
            <div>Failed to load</div>
            <div class="max-w-full truncate px-2">{fileName}</div>
          </div>
        </div>
      {:else if imageSrc === null}
        <div
          class="flex h-full w-full items-center justify-center bg-muted px-3 text-center"
        >
          <div class="text-xs text-muted-foreground">
            <div>{UNSUPPORTED_FORMAT_MESSAGE}</div>
            <div class="mt-1 max-w-full truncate">{fileName}</div>
          </div>
        </div>
      {:else}
        {#if !imageLoaded && !imageError}
          <div
            class="flex h-full w-full animate-pulse items-center justify-center bg-muted"
          >
            <div class="text-sm text-muted-foreground">Loading...</div>
          </div>
        {/if}

        {#if imageError}
          <div class="flex h-full w-full items-center justify-center bg-muted">
            <div class="p-2 text-center text-xs text-muted-foreground">
              <div>Failed to load</div>
              <div class="max-w-full truncate px-2">{fileName}</div>
            </div>
          </div>
        {/if}

        <img
          src={imageSrc}
          alt={fileName}
          loading={isVisible ? 'eager' : 'lazy'}
          class={`h-full w-full transition-opacity duration-300 ${
            showWholeFrame ? 'object-contain' : 'object-cover'
          } ${imageLoaded ? 'opacity-100' : 'opacity-0'}`}
          onload={handleImageLoad}
          onerror={handleImageError}
        />
        {#if isRejected}
          <div class="pointer-events-none absolute inset-0 bg-background/55"
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

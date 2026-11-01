<script lang="ts">
  import * as Card from '$lib/components/ui/card';
  import { cullFlags } from '../stores/galleryStore';
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

  let thumbnailSrc = $derived.by(() => {
    const thumb = photo.thumbnail_path;
    if (
      thumb.startsWith('data:') ||
      thumb.startsWith('blob:') ||
      thumb.startsWith('http://') ||
      thumb.startsWith('https://')
    ) {
      return thumb;
    }
    return `lixa-thumbnail://localhost/${encodeURIComponent(photo.path)}`;
  });

  let imageLoaded = $state(false);
  let imageError = $state(false);

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
  }
</script>

<Card.Root
  class={`group relative cursor-pointer transition-shadow duration-200 ease-in-out hover:shadow-lg focus:outline-none ${
    isActive
      ? 'ring-2 ring-primary ring-offset-2 ring-offset-background'
      : 'hover:scale-[1.02]'
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
      class="relative aspect-[1/1] h-auto w-full overflow-hidden bg-gray-100 dark:bg-gray-800"
    >
      {#if !imageLoaded && !imageError}
        <div
          class="flex h-full w-full animate-pulse items-center justify-center bg-gray-200 dark:bg-gray-700"
        >
          <div class="text-sm text-gray-400">Loading...</div>
        </div>
      {/if}

      {#if imageError}
        <div
          class="flex h-full w-full items-center justify-center bg-gray-100 dark:bg-gray-800"
        >
          <div class="p-2 text-center text-xs text-gray-500">
            <div>Failed to load</div>
            <div class="max-w-full truncate px-2 text-gray-400">{fileName}</div>
          </div>
        </div>
      {/if}

      <img
        src={thumbnailSrc}
        alt={fileName}
        loading={isVisible ? 'eager' : 'lazy'}
        class={`h-full w-full object-cover transition-opacity duration-300 ${
          isActive ? '' : 'group-hover:scale-105'
        } ${imageLoaded ? 'opacity-100' : 'opacity-0'}`}
        onload={handleImageLoad}
        onerror={handleImageError}
      />
      {#if isRejected}
        <div
          class="pointer-events-none absolute inset-0 bg-background/55"
        ></div>
      {/if}
    </div>

    <button
      class={`absolute right-2 top-2 rounded-full p-2 shadow-md transition-all duration-200 ${
        isPicked
          ? 'bg-red-50 text-red-500 hover:bg-red-100'
          : isRejected
            ? 'bg-background/90 text-foreground'
            : 'bg-white/80 text-gray-400 opacity-0 hover:bg-white hover:text-red-500 group-hover:opacity-100'
      } backdrop-blur-sm`}
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

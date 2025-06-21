<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import AppLayout from '../components/layout/AppLayout.svelte';
  import Gallery from '../components/Gallery.svelte';
  import KeyboardShortcuts from '../components/common/KeyboardShortcuts.svelte';
  import { loadPhotos } from '../components/common/ImageOperations';
  import { favorites, isLoading, photos } from '../stores/galleryStore';

  const keyboardActions = {
    o: loadPhotos,
  };
</script>

<KeyboardShortcuts actions={keyboardActions} />

<AppLayout title="Lixa Gallery">
  <svelte:fragment slot="header-actions">
    <a
      href="/favourites"
      class="inline-flex h-10 items-center justify-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground ring-offset-background transition-colors hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
    >
      Show Favourites {$favorites.size}
    </a>
  </svelte:fragment>

  {#if $isLoading}
    <p class="text-center">Loading...</p>
  {:else if $photos.length === 0}
    <div class="flex items-center justify-center">
      <Button on:click={loadPhotos}>Open Folder</Button>
    </div>
  {:else}
    <Gallery photos={$photos} />
  {/if}
</AppLayout>

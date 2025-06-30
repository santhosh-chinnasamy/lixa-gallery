<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import Gallery from '../components/Gallery.svelte';
  import { loadPhotos } from '../components/common/ImageOperations';
  import KeyboardShortcuts from '../components/common/KeyboardShortcuts.svelte';
  import AppLayout from '../components/layout/AppLayout.svelte';
  import { isLoading, photos } from '../stores/galleryStore';

  const keyboardActions = {
    o: loadPhotos,
  };
</script>

<KeyboardShortcuts actions={keyboardActions} />

<AppLayout>
  {#if $isLoading}
    <p class="text-center">Loading...</p>
  {:else if $photos.length === 0}
    <div class="flex items-center justify-center">
      <Button onclick={loadPhotos}>Open Folder</Button>
    </div>
  {:else}
    <div class="flex flex-col items-center justify-end">
      <Button onclick={loadPhotos}>Choose another folder</Button>
      <Gallery photos={$photos} />
    </div>
  {/if}
</AppLayout>

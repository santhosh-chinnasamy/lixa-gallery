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
    <!-- Action buttons container -->
    <div class="sticky top-4 z-10 mb-4 flex justify-center px-4">
      <div class="flex items-center gap-2 rounded-lg bg-background/80 p-2 shadow-sm backdrop-blur-sm border">
        <Button 
          onclick={loadPhotos}
          variant="outline"
          class="text-sm font-medium"
        >
          Choose another folder
        </Button>
      </div>
    </div>
    
    <Gallery photos={$photos} />
  {/if}
</AppLayout>

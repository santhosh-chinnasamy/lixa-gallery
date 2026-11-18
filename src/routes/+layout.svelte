<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import '../app.css';
  import AppSidebar from '../components/AppSidebar.svelte';
  import ShortcutOverlay from '../components/ShortcutOverlay.svelte';
  import type { KeyboardActions } from '../types/events';
  import { initLogger } from '$lib/logger';
  import ErrorToast from '../components/common/ErrorToast.svelte';
  import { initTheme } from '../stores/themeStore';
  import { mergePhotoUpdates } from '../stores/galleryStore';
  import type { PhotosUpdatedEvent } from '../types/photo';
  import type { Snippet } from 'svelte';

  let { children }: { children?: Snippet } = $props();

  $effect(() => {
    initLogger();
    return initTheme();
  });

  $effect(() => {
    let unlisten: UnlistenFn | null = null;
    let cancelled = false;
    listen<PhotosUpdatedEvent>('photos-updated', (event) => {
      const payload = event.payload;
      if (!payload?.photos?.length) return;
      mergePhotoUpdates(payload.generation, payload.photos);
    }).then((stop) => {
      if (cancelled) stop();
      else unlisten = stop;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  });

  const toggleFullScreen = async () => {
    const fullscreen = await getCurrentWindow().isFullscreen();
    await getCurrentWindow().setFullscreen(!fullscreen);
  };

  const keyboardActions: KeyboardActions = {
    F11: toggleFullScreen,
  };

  const handleKeydown = (event: KeyboardEvent) => {
    try {
      const action = keyboardActions[event.key];
      if (action) action();
    } catch (error) {
      console.error(
        `Error executing keyboard action: [key: ${event.key}]`,
        error,
      );
    }
  };
</script>

<svelte:window onkeydown={handleKeydown} />
<ShortcutOverlay />
<Sidebar.Provider open={true}>
  <div
    class="flex h-screen w-full overflow-hidden bg-background font-sans text-foreground antialiased"
  >
    <AppSidebar />
    <main class="flex h-full min-w-0 flex-1 flex-col overflow-hidden">
      <!-- min-h-0 so route pages can own overflow-y-auto (e.g. Settings). -->
      <div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden">
        {@render children?.()}
      </div>
    </main>
    <ErrorToast />
  </div>
</Sidebar.Provider>

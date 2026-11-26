<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import { beforeNavigate, afterNavigate } from '$app/navigation';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import '../app.css';
  import AppSidebar from '../components/AppSidebar.svelte';
  import ShortcutOverlay from '../components/ShortcutOverlay.svelte';
  import type { KeyboardActions } from '../types/events';
  import { initLogger } from '$lib/logger';
  import { probe } from '$lib/debugProbe';
  import ErrorToast from '../components/common/ErrorToast.svelte';
  import { initTheme } from '../stores/themeStore';
  import { folderGeneration, mergePhotoUpdates } from '../stores/galleryStore';
  import type { PhotoMetadata, PhotosUpdatedEvent } from '../types/photo';
  import type { Snippet } from 'svelte';
  import { get } from 'svelte/store';

  let { children }: { children?: Snippet } = $props();

  // WebKit: `nav.from?.url.pathname` throws when `from` exists but `url` is
  // null (`?.` only guards one hop). That unhandled rejection aborts SvelteKit
  // client start, so every <a> becomes a full reload and Favourites feels stuck.
  beforeNavigate((nav) => {
    try {
      probe('nav.before', {
        from: nav.from?.url?.pathname ?? null,
        to: nav.to?.url?.pathname ?? null,
        type: nav.type,
      });
    } catch (error) {
      console.warn('[probe] nav.before failed', error);
    }
  });

  afterNavigate((nav) => {
    try {
      probe('nav.after', {
        from: nav.from?.url?.pathname ?? null,
        to: nav.to?.url?.pathname ?? null,
        type: nav.type,
      });
    } catch (error) {
      console.warn('[probe] nav.after failed', error);
    }
  });

  $effect(() => {
    initLogger();
    probe('layout.mount', {
      origin: typeof location !== 'undefined' ? location.origin : null,
      href: typeof location !== 'undefined' ? location.href : null,
    });
    return initTheme();
  });

  $effect(() => {
    let unlisten: UnlistenFn | null = null;
    let cancelled = false;
    listen<PhotosUpdatedEvent | PhotoMetadata[]>('photos-updated', (event) => {
      const payload = event.payload;
      // Tolerate a bare array from an older backend build.
      if (Array.isArray(payload)) {
        mergePhotoUpdates(get(folderGeneration), payload);
        return;
      }
      if (!payload?.photos?.length) return;
      mergePhotoUpdates(
        Number(payload.generation) || get(folderGeneration),
        payload.photos,
      );
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

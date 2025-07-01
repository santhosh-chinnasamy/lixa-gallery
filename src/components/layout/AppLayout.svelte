<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { KeyboardActions } from '../../types/events';
  import AppSidebar from '../AppSidebar.svelte';
  import { page } from '$app/state';

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
  $: isHomePage = page.url.pathname === '/';
</script>

<svelte:window onkeydown={handleKeydown} />
<Sidebar.Provider>
  <AppSidebar />
  <Sidebar.Trigger />
  <main
    class={`m-auto flex min-h-screen max-w-[100vw] justify-center ${isHomePage && 'items-center'}`}
  >
    <div class="flex flex-1 flex-col p-4">
      <slot />
    </div>
  </main>
</Sidebar.Provider>

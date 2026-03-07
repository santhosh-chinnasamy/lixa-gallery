<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import '../app.css';
  import AppSidebar from '../components/AppSidebar.svelte';
  import type { KeyboardActions } from '../types/events';

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
<Sidebar.Provider open={true}>
  <div class="flex min-h-screen w-full bg-background font-sans antialiased text-foreground">
    <AppSidebar />
    <main class="flex-1 flex flex-col min-w-0 overflow-hidden">
      <div class="flex-1 overflow-y-auto p-6 lg:p-10">
        <slot />
      </div>
    </main>
  </div>
</Sidebar.Provider>

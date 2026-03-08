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
  <div
    class="flex h-screen w-full overflow-hidden bg-background font-sans text-foreground antialiased"
  >
    <AppSidebar />
    <main class="flex h-full min-w-0 flex-1 flex-col overflow-hidden">
      <!-- Removed p-6 lg:p-10 from here to allow ControlBar to stretch full width. Added it inside Gallery.svelte -->
      <div class="h-full flex-1 overflow-hidden">
        <slot />
      </div>
    </main>
  </div>
</Sidebar.Provider>

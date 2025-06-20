<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { KeyboardActions } from '../../types/events';
  import GithubLogo from '../icons/Github.svelte';

  let {
    title = 'Lixa Gallery',
    subtitle = 'Select your favorite photos and export',
  } = $props();

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

<main class="flex min-h-screen max-w-[100vw] flex-col">
  <header
    class="sticky top-0 z-10 flex items-center justify-between bg-gray-50 px-4 py-6 shadow-sm"
  >
    <div>
      <span class="flex items-center">
        <h1 class="scroll-m-20 text-2xl font-semibold tracking-tight">
          {title}
        </h1>
        <a
          href="https://github.com/santhosh-chinnasamy/lixa-gallery"
          target="_blank"
          rel="noopener noreferrer"
        >
          <GithubLogo />
        </a>
      </span>
      {#if subtitle}
        <p class="text-base text-muted-foreground">{subtitle}</p>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <slot name="header-actions" />
    </div>
  </header>

  <div class="flex-1 p-4">
    <slot />
  </div>
</main>

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import * as Tabs from '$lib/components/ui/tabs';
  import { openLogsFolder, getLogDirectory, info } from '$lib/logger';
  import {
    autoAdvance,
    loadingMode,
    type LoadingMode,
  } from '../../stores/galleryStore';
  import ShortcutSection from '../../components/ShortcutSection.svelte';
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import CheckIcon from '@lucide/svelte/icons/check';
  import FileTextIcon from '@lucide/svelte/icons/file-text';
  import InfoIcon from '@lucide/svelte/icons/info';
  import ZapIcon from '@lucide/svelte/icons/zap';
  import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
  import SlidersHorizontalIcon from '@lucide/svelte/icons/sliders-horizontal';
  import KeyboardIcon from '@lucide/svelte/icons/keyboard';
  import EyeIcon from '@lucide/svelte/icons/eye';
  import CompassIcon from '@lucide/svelte/icons/compass';
  import PlayIcon from '@lucide/svelte/icons/play';

  let logDirPath = $state<string>('');
  let isCopied = $state(false);
  let statusMessage = $state<string>('');

  $effect(() => {
    getLogDirectory()
      .then((path) => {
        logDirPath = path;
      })
      .catch((err) => {
        logDirPath = 'Unable to determine log path';
      });
  });

  async function handleOpenLogs() {
    try {
      statusMessage = 'Opening logs folder...';
      const opened = await openLogsFolder();
      statusMessage = `Opened: ${opened}`;
      setTimeout(() => {
        statusMessage = '';
      }, 4000);
    } catch (err) {
      statusMessage = `Failed to open logs: ${err}`;
      setTimeout(() => {
        statusMessage = '';
      }, 5000);
    }
  }

  async function handleCopyLogPath() {
    if (!logDirPath) return;
    try {
      await navigator.clipboard.writeText(logDirPath);
      isCopied = true;
      setTimeout(() => {
        isCopied = false;
      }, 2500);
    } catch (err) {
      statusMessage = 'Failed to copy to clipboard';
    }
  }

  function handleModeChange(mode: LoadingMode) {
    loadingMode.set(mode);
    info(`[Settings] Scanning mode changed to: ${mode}`);
  }
</script>

<div class="custom-scrollbar flex-1 overflow-y-auto p-6 lg:p-10">
  <div class="mx-auto max-w-4xl space-y-8">
    <!-- Page Header -->
    <div class="border-b pb-5">
      <h1 class="text-3xl font-bold tracking-tight text-foreground">
        Settings
      </h1>
      <p class="mt-1 text-sm text-muted-foreground">
        Configure application preferences, performance, diagnostics, and
        keyboard shortcuts.
      </p>
    </div>

    <!-- Tabs Navigation -->
    <Tabs.Root value="general" class="w-full">
      <Tabs.List class="grid w-full max-w-md grid-cols-2">
        <Tabs.Trigger value="general" class="flex items-center gap-2">
          <SlidersHorizontalIcon class="h-4 w-4" />
          <span>General & Diagnostics</span>
        </Tabs.Trigger>
        <Tabs.Trigger value="shortcuts" class="flex items-center gap-2">
          <KeyboardIcon class="h-4 w-4" />
          <span>Keyboard Shortcuts</span>
        </Tabs.Trigger>
      </Tabs.List>

      <!-- Tab 1: General & Diagnostics -->
      <Tabs.Content value="general" class="mt-6 space-y-8">
        <!-- Section 1: Diagnostics & Logs -->
        <Card.Root class="shadow-sm">
          <Card.Header>
            <div class="flex items-center gap-2">
              <FileTextIcon class="h-5 w-5 text-primary" />
              <Card.Title>Diagnostics & Troubleshooting</Card.Title>
            </div>
            <Card.Description>
              Logs are stored locally on your device to help troubleshoot bugs,
              crashes, and performance issues.
            </Card.Description>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div>
              <label
                class="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                for="log-path"
              >
                Log Directory Location
              </label>
              <div
                class="mt-1.5 flex flex-col gap-2 sm:flex-row sm:items-center"
              >
                <input
                  id="log-path"
                  type="text"
                  readonly
                  value={logDirPath || 'Loading log directory path...'}
                  class="w-full rounded-md border bg-muted/40 px-3 py-2 font-mono text-xs text-foreground focus:outline-none"
                />
                <div class="flex shrink-0 gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onclick={handleCopyLogPath}
                    class="flex items-center gap-1.5"
                    title="Copy log directory path"
                  >
                    {#if isCopied}
                      <CheckIcon class="h-4 w-4 text-green-500" />
                      <span>Copied</span>
                    {:else}
                      <CopyIcon class="h-4 w-4" />
                      <span>Copy</span>
                    {/if}
                  </Button>
                  <Button
                    variant="default"
                    size="sm"
                    onclick={handleOpenLogs}
                    class="flex items-center gap-1.5"
                  >
                    <FolderOpenIcon class="h-4 w-4" />
                    <span>Open Folder</span>
                  </Button>
                </div>
              </div>
              {#if statusMessage}
                <p class="mt-2 text-xs font-medium text-primary">
                  {statusMessage}
                </p>
              {/if}
            </div>

            <div class="rounded-lg border bg-muted/30 p-4">
              <div class="flex items-start gap-3">
                <InfoIcon
                  class="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground"
                />
                <div class="text-xs leading-relaxed text-muted-foreground">
                  <p class="font-medium text-foreground">
                    How to report an issue:
                  </p>
                  <p class="mt-0.5">
                    When opening a bug report or feature request on GitHub,
                    attach the active <code
                      class="rounded bg-muted px-1 py-0.5 font-mono"
                      >Lixa Gallery.log</code
                    > file from this directory. It includes application event logs,
                    error stack traces, and crash information.
                  </p>
                </div>
              </div>
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Section 2: Scanning & Performance -->
        <Card.Root class="shadow-sm">
          <Card.Header>
            <div class="flex items-center gap-2">
              <ZapIcon class="h-5 w-5 text-amber-500" />
              <Card.Title>Scanning & Indexing Performance</Card.Title>
            </div>
            <Card.Description>
              Controls how photos and thumbnail previews are indexed when
              browsing folders.
            </Card.Description>
          </Card.Header>
          <Card.Content class="space-y-4">
            <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <button
                type="button"
                onclick={() => handleModeChange('lazy')}
                class={`flex flex-col items-start rounded-xl border p-4 text-left transition-all ${
                  $loadingMode === 'lazy'
                    ? 'border-primary bg-primary/5 ring-2 ring-primary/20'
                    : 'bg-card hover:bg-muted/50'
                }`}
              >
                <div class="flex w-full items-center justify-between">
                  <span class="font-semibold text-foreground"
                    >Lazy Loading (Recommended)</span
                  >
                  {#if $loadingMode === 'lazy'}
                    <span
                      class="rounded-full bg-primary/20 px-2 py-0.5 text-xs font-medium text-primary"
                      >Active</span
                    >
                  {/if}
                </div>
                <p class="mt-2 text-xs text-muted-foreground">
                  Renders photo metadata instantly and creates high-performance
                  WebP thumbnails on-the-fly via the custom Rust protocol.
                </p>
              </button>

              <button
                type="button"
                onclick={() => handleModeChange('sync')}
                class={`flex flex-col items-start rounded-xl border p-4 text-left transition-all ${
                  $loadingMode === 'sync'
                    ? 'border-primary bg-primary/5 ring-2 ring-primary/20'
                    : 'bg-card hover:bg-muted/50'
                }`}
              >
                <div class="flex w-full items-center justify-between">
                  <span class="font-semibold text-foreground"
                    >Sequential Sync</span
                  >
                  {#if $loadingMode === 'sync'}
                    <span
                      class="rounded-full bg-primary/20 px-2 py-0.5 text-xs font-medium text-primary"
                      >Active</span
                    >
                  {/if}
                </div>
                <p class="mt-2 text-xs text-muted-foreground">
                  Indexes all folder thumbnails upfront with micro-sleeps to
                  maintain smooth UI responsiveness before display.
                </p>
              </button>
            </div>
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-sm">
          <Card.Header>
            <div class="flex items-center gap-2">
              <KeyboardIcon class="h-5 w-5 text-primary" />
              <Card.Title>Culling</Card.Title>
            </div>
            <Card.Description>
              After you pick, reject, or unflag from the keyboard, move to the
              next photo.
            </Card.Description>
          </Card.Header>
          <Card.Content>
            <div class="flex items-center justify-between gap-4">
              <div>
                <div class="text-sm font-medium text-foreground">
                  Auto-advance
                </div>
                <p class="mt-0.5 text-xs text-muted-foreground">
                  A full pass with P, X, and U. Turn it off to stay on the frame
                  you just marked.
                </p>
              </div>
              <button
                type="button"
                class={`h-8 shrink-0 rounded-md border px-3 text-xs font-medium transition-colors ${
                  $autoAdvance
                    ? 'border-primary/30 bg-primary/10 text-primary'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
                aria-pressed={$autoAdvance}
                onclick={() => autoAdvance.set(!$autoAdvance)}
              >
                {$autoAdvance ? 'On' : 'Off'}
              </button>
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Section 3: Privacy & Telemetry Notice -->
        <Card.Root class="shadow-sm">
          <Card.Header>
            <div class="flex items-center gap-2">
              <ShieldCheckIcon class="h-5 w-5 text-emerald-500" />
              <Card.Title>Privacy & Data Storage</Card.Title>
            </div>
          </Card.Header>
          <Card.Content>
            <p class="text-xs leading-relaxed text-muted-foreground">
              Lixa Gallery operates 100% locally. Photos are never uploaded or
              synced to external servers. All favorites and thumbnails are saved
              inside your operating system's local application data directory.
            </p>
          </Card.Content>
        </Card.Root>
      </Tabs.Content>

      <!-- Tab 2: Keyboard Shortcuts -->
      <Tabs.Content value="shortcuts" class="mt-6 space-y-8">
        <Card.Root class="shadow-sm">
          <Card.Header>
            <div class="flex items-center gap-2">
              <EyeIcon class="h-5 w-5 text-primary" />
              <Card.Title>Photo Viewer</Card.Title>
            </div>
            <Card.Description>
              The same keys as the ? overlay while a frame is open. Drag pans a
              zoomed frame. Double-click swaps 100% and 250%. The scroll wheel
              or a pinch zooms.
            </Card.Description>
          </Card.Header>
          <Card.Content>
            <ShortcutSection context="viewer" />
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-sm">
          <Card.Header>
            <div class="flex items-center gap-2">
              <PlayIcon class="h-5 w-5 text-primary" />
              <Card.Title>Slideshow</Card.Title>
            </div>
            <Card.Description>
              Keys while a slideshow is playing. Fullscreen stays on F11.
            </Card.Description>
          </Card.Header>
          <Card.Content>
            <ShortcutSection context="slideshow" />
          </Card.Content>
        </Card.Root>

        <Card.Root class="shadow-sm">
          <Card.Header>
            <div class="flex items-center gap-2">
              <CompassIcon class="h-5 w-5 text-primary" />
              <Card.Title>Grid</Card.Title>
            </div>
            <Card.Description>
              Move through the folder, then mark frames without leaving the
              grid.
            </Card.Description>
          </Card.Header>
          <Card.Content>
            <ShortcutSection context="grid" />
          </Card.Content>
        </Card.Root>
      </Tabs.Content>
    </Tabs.Root>
  </div>
</div>

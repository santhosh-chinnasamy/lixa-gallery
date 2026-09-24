<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import { openLogsFolder, getLogDirectory, info } from '$lib/logger';
  import { loadingMode, type LoadingMode } from '../../stores/galleryStore';
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import CheckIcon from '@lucide/svelte/icons/check';
  import FileTextIcon from '@lucide/svelte/icons/file-text';
  import InfoIcon from '@lucide/svelte/icons/info';
  import ZapIcon from '@lucide/svelte/icons/zap';
  import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';

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
        Configure application preferences, performance, and diagnostics.
      </p>
    </div>

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
          <div class="mt-1.5 flex flex-col gap-2 sm:flex-row sm:items-center">
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
            <p class="mt-2 text-xs font-medium text-primary">{statusMessage}</p>
          {/if}
        </div>

        <div class="rounded-lg border bg-muted/30 p-4">
          <div class="flex items-start gap-3">
            <InfoIcon class="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
            <div class="text-xs leading-relaxed text-muted-foreground">
              <p class="font-medium text-foreground">How to report an issue:</p>
              <p class="mt-0.5">
                When opening a bug report or feature request on GitHub, attach
                the active <code class="rounded bg-muted px-1 py-0.5 font-mono"
                  >Lixa Gallery.log</code
                > file from this directory. It includes application event logs, error
                stack traces, and crash information.
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
          Controls how photos and thumbnail previews are indexed when browsing
          folders.
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
              Renders photo metadata instantly and creates high-performance WebP
              thumbnails on-the-fly via the custom Rust protocol.
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
              <span class="font-semibold text-foreground">Sequential Sync</span>
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
  </div>
</div>

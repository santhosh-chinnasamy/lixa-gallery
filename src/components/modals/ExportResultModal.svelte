<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
  } from '$lib/components/ui/dialog';
  import { openPath } from '@tauri-apps/plugin-opener';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import type { ExportResult } from '../../types/photo';

  let {
    open = $bindable(false),
    result = null,
    onClose,
  }: {
    open: boolean;
    result?: ExportResult | null;
    onClose: () => void;
  } = $props();

  let openError = $state('');

  const transferred = $derived(
    result ? (result.mode === 'move' ? result.moved : result.copied) : 0,
  );
  const transferLabel = $derived(result?.mode === 'move' ? 'Moved' : 'Copied');
  const hasFailures = $derived((result?.failed ?? 0) > 0);
  const failures = $derived(
    (result?.files ?? []).filter((file) => file.status === 'failed'),
  );

  const title = $derived.by(() => {
    if (!result || !hasFailures) return 'Export complete';
    if (transferred === 0 && result.skipped === 0) return 'Export failed';
    return 'Export finished with errors';
  });

  const summary = $derived.by(() => {
    if (!result) return '';
    if (!hasFailures) return 'The files are in the folder below.';
    if (transferred === 0 && result.skipped === 0) {
      return 'Nothing was exported. Each file is listed with the reason it failed.';
    }
    return 'Some files were not exported. Each failure is listed with the reason.';
  });

  $effect(() => {
    if (open) openError = '';
  });

  function fileLabel(source: string): string {
    const trimmed = source.trim();
    if (!trimmed) return 'Export';
    const parts = trimmed.split(/[/\\]/);
    return parts[parts.length - 1] || trimmed;
  }

  async function openFolder() {
    if (!result?.destination) return;
    openError = '';
    try {
      await openPath(result.destination);
    } catch (error) {
      openError = error instanceof Error ? error.message : String(error);
    }
  }

  function handleDone() {
    open = false;
    onClose();
  }
</script>

<Dialog bind:open onOpenChange={(next) => !next && onClose()}>
  <DialogContent class="sm:max-w-lg">
    <DialogHeader>
      <DialogTitle class="flex items-center gap-2">
        {#if hasFailures}
          <CircleAlert class="h-5 w-5 text-destructive" />
        {:else}
          <CircleCheck class="h-5 w-5 text-foreground" />
        {/if}
        {title}
      </DialogTitle>
      <DialogDescription class="text-left">{summary}</DialogDescription>
    </DialogHeader>

    {#if result}
      <div class="space-y-3">
        <dl class="divide-y divide-border rounded-lg border bg-muted/30">
          <div class="flex items-baseline justify-between gap-3 px-3 py-2">
            <dt class="text-xs text-muted-foreground">{transferLabel}</dt>
            <dd class="text-sm font-semibold tabular-nums text-foreground">
              {transferred}
            </dd>
          </div>
          <div class="flex items-baseline justify-between gap-3 px-3 py-2">
            <dt class="text-xs text-muted-foreground">Skipped</dt>
            <dd class="text-sm font-semibold tabular-nums text-foreground">
              {result.skipped}
            </dd>
          </div>
          <div class="flex items-baseline justify-between gap-3 px-3 py-2">
            <dt class="text-xs text-muted-foreground">Failed</dt>
            <dd
              class={`text-sm font-semibold tabular-nums ${
                result.failed > 0 ? 'text-destructive' : 'text-foreground'
              }`}
            >
              {result.failed}
            </dd>
          </div>
        </dl>

        {#if result.destination}
          <div class="space-y-1">
            <div
              class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
            >
              Destination
            </div>
            <p
              class="break-all rounded-md border bg-muted/40 px-2.5 py-2 font-mono text-[11px] leading-relaxed text-foreground"
              title={result.destination}
            >
              {result.destination}
            </p>
          </div>
        {/if}

        {#if failures.length > 0}
          <div class="space-y-1">
            <div
              class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
            >
              Could not export
            </div>
            <ul
              class="max-h-40 divide-y divide-border overflow-y-auto rounded-md border"
            >
              {#each failures as file, index (`${file.source}-${index}`)}
                <li class="px-2.5 py-2">
                  <div class="truncate text-xs font-medium text-foreground">
                    {fileLabel(file.source)}
                  </div>
                  {#if file.reason}
                    <p
                      class="mt-0.5 text-[11px] leading-relaxed text-muted-foreground"
                    >
                      {file.reason}
                    </p>
                  {/if}
                </li>
              {/each}
            </ul>
          </div>
        {/if}

        {#if openError}
          <p class="text-xs text-destructive">{openError}</p>
        {/if}
      </div>
    {/if}

    <DialogFooter>
      {#if result?.destination}
        <Button variant="outline" onclick={openFolder}>
          <FolderOpen class="h-4 w-4" />
          Open folder
        </Button>
      {/if}
      <Button onclick={handleDone}>Done</Button>
    </DialogFooter>
  </DialogContent>
</Dialog>

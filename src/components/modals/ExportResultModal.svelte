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
  import type { ExportReport } from '../../types/photo';

  let {
    open = $bindable(false),
    report = null,
    destination = '',
    mode = 'copy',
    onClose,
  }: {
    open: boolean;
    report?: ExportReport | null;
    destination?: string;
    mode?: 'copy' | 'move';
    onClose: () => void;
  } = $props();

  let openError = $state('');

  const transferred = $derived(report ? report.copied + report.renamed : 0);
  const transferLabel = $derived(mode === 'move' ? 'Moved' : 'Copied');
  const hasFailures = $derived((report?.failed ?? 0) > 0);
  const cancelled = $derived(report?.cancelled ?? false);

  const title = $derived.by(() => {
    if (!report) return 'Export complete';
    if (cancelled) return 'Export cancelled';
    if (!hasFailures) return 'Export complete';
    if (transferred === 0 && report.skipped === 0) return 'Export failed';
    return 'Export finished with errors';
  });

  const summary = $derived.by(() => {
    if (!report) return '';
    if (cancelled) {
      return `Stopped after ${transferred} of ${report.total} files. Counts below are only what finished before the stop.`;
    }
    if (!hasFailures) return 'The files are in the folder below.';
    if (transferred === 0 && report.skipped === 0) {
      return 'Nothing was exported. Each file is listed with the reason it failed.';
    }
    return 'Some files were not exported. Each failure is listed with the reason.';
  });

  $effect(() => {
    if (open) openError = '';
  });

  function fileLabel(path: string): string {
    const trimmed = path.trim();
    if (!trimmed) return 'Export';
    const parts = trimmed.split(/[/\\]/);
    return parts[parts.length - 1] || trimmed;
  }

  async function openFolder() {
    if (!destination) return;
    openError = '';
    try {
      await openPath(destination);
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
        {#if hasFailures || cancelled}
          <CircleAlert class="h-5 w-5 text-destructive" />
        {:else}
          <CircleCheck class="h-5 w-5 text-foreground" />
        {/if}
        {title}
      </DialogTitle>
      <DialogDescription class="text-left">{summary}</DialogDescription>
    </DialogHeader>

    {#if report}
      <div class="space-y-3">
        <dl class="divide-y divide-border rounded-lg border bg-muted/30">
          <div class="flex items-baseline justify-between gap-3 px-3 py-2">
            <dt class="text-xs text-muted-foreground">{transferLabel}</dt>
            <dd class="text-sm font-semibold tabular-nums text-foreground">
              {report.copied}
            </dd>
          </div>
          <div class="flex items-baseline justify-between gap-3 px-3 py-2">
            <dt class="text-xs text-muted-foreground">Renamed</dt>
            <dd class="text-sm font-semibold tabular-nums text-foreground">
              {report.renamed}
            </dd>
          </div>
          <div class="flex items-baseline justify-between gap-3 px-3 py-2">
            <dt class="text-xs text-muted-foreground">Skipped</dt>
            <dd class="text-sm font-semibold tabular-nums text-foreground">
              {report.skipped}
            </dd>
          </div>
          <div class="flex items-baseline justify-between gap-3 px-3 py-2">
            <dt class="text-xs text-muted-foreground">Failed</dt>
            <dd
              class={`text-sm font-semibold tabular-nums ${
                report.failed > 0 ? 'text-destructive' : 'text-foreground'
              }`}
            >
              {report.failed}
            </dd>
          </div>
        </dl>

        {#if destination}
          <div class="space-y-1">
            <div
              class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
            >
              Destination
            </div>
            <p
              class="break-all rounded-md border bg-muted/40 px-2.5 py-2 font-mono text-[11px] leading-relaxed text-foreground"
              title={destination}
            >
              {destination}
            </p>
          </div>
        {/if}

        {#if report.failures.length > 0}
          <div class="space-y-1">
            <div
              class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
            >
              Could not export
            </div>
            <ul
              class="max-h-40 divide-y divide-border overflow-y-auto rounded-md border"
            >
              {#each report.failures as failure, index (`${failure.path}-${index}`)}
                <li class="px-2.5 py-2">
                  <div class="truncate text-xs font-medium text-foreground">
                    {fileLabel(failure.path)}
                  </div>
                  {#if failure.reason}
                    <p
                      class="mt-0.5 text-[11px] leading-relaxed text-muted-foreground"
                    >
                      {failure.reason}
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
      {#if destination}
        <Button variant="outline" onclick={openFolder}>
          <FolderOpen class="h-4 w-4" />
          Open folder
        </Button>
      {/if}
      <Button onclick={handleDone}>Done</Button>
    </DialogFooter>
  </DialogContent>
</Dialog>

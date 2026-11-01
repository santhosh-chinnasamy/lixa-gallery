<script lang="ts">
  import { shortcutsFor, type ShortcutContext } from '$lib/shortcuts';
  import ShortcutKeys from './ShortcutKeys.svelte';

  let { context }: { context: ShortcutContext } = $props();

  const items = $derived(shortcutsFor(context));
</script>

<div class="divide-y divide-border text-sm">
  {#each items as shortcut (shortcut.id)}
    <div class="flex items-center justify-between gap-4 py-3">
      <div class="min-w-0">
        <div class="font-medium text-foreground">{shortcut.label}</div>
        {#if shortcut.detail}
          <div class="text-xs text-muted-foreground">{shortcut.detail}</div>
        {/if}
      </div>
      <ShortcutKeys chords={shortcut.chords} />
    </div>
  {/each}
</div>

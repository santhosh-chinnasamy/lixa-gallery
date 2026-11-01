<script lang="ts">
  import X from '@lucide/svelte/icons/x';
  import {
    closeShortcutHelp,
    contextLabel,
    isTextEntry,
    shortcutContext,
    shortcutHelpOpen,
    shortcutMatches,
    shortcutsFor,
    toggleShortcutHelp,
  } from '$lib/shortcuts';
  import ShortcutKeys from './ShortcutKeys.svelte';

  const items = $derived(shortcutsFor($shortcutContext));

  function onKeydown(event: KeyboardEvent) {
    if (isTextEntry(event.target)) return;

    if ($shortcutHelpOpen && shortcutMatches(event, 'close')) {
      event.preventDefault();
      event.stopImmediatePropagation();
      closeShortcutHelp();
      return;
    }

    if (shortcutMatches(event, 'help')) {
      event.preventDefault();
      event.stopImmediatePropagation();
      toggleShortcutHelp();
    }
  }

  function onBackdrop(event: MouseEvent) {
    if (event.target === event.currentTarget) closeShortcutHelp();
  }

  $effect(() => {
    const handler = (event: KeyboardEvent) => onKeydown(event);
    window.addEventListener('keydown', handler, true);
    return () => window.removeEventListener('keydown', handler, true);
  });
</script>

{#if $shortcutHelpOpen}
  <div
    class="fixed inset-0 z-[80] flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    onclick={onBackdrop}
  >
    <div
      class="flex max-h-[80vh] w-full max-w-lg flex-col overflow-hidden rounded-xl border bg-background shadow-lg"
      role="dialog"
      aria-modal="true"
      aria-labelledby="shortcut-overlay-title"
    >
      <div class="flex items-start justify-between gap-4 border-b px-5 py-4">
        <div>
          <h2
            id="shortcut-overlay-title"
            class="text-lg font-semibold tracking-tight text-foreground"
          >
            Shortcuts
          </h2>
          <p class="mt-0.5 text-sm text-muted-foreground">
            {contextLabel($shortcutContext)}
          </p>
        </div>
        <button
          type="button"
          class="rounded-md p-1 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          onclick={closeShortcutHelp}
          aria-label="Close shortcuts"
        >
          <X class="size-4" />
        </button>
      </div>
      <div
        class="custom-scrollbar max-h-[calc(80vh-8.5rem)] divide-y divide-border overflow-y-auto px-5"
      >
        {#each items as shortcut (shortcut.id)}
          <div class="flex items-center justify-between gap-4 py-3 text-sm">
            <div class="min-w-0">
              <div class="font-medium text-foreground">{shortcut.label}</div>
              {#if shortcut.detail}
                <div class="text-xs text-muted-foreground">
                  {shortcut.detail}
                </div>
              {/if}
            </div>
            <ShortcutKeys chords={shortcut.chords} />
          </div>
        {/each}
      </div>
      <p class="border-t px-5 py-3 text-xs text-muted-foreground">
        Press ? again or Esc to close. The same list is in Settings.
      </p>
    </div>
  </div>
{/if}

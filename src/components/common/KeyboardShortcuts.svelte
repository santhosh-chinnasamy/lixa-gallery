<script lang="ts">
  import { get } from 'svelte/store';
  import type { KeyboardActions } from '../../types/events';
  import {
    isTextEntry,
    shortcutContext,
    shortcutHelpOpen,
  } from '$lib/shortcuts';

  let { actions } = $props<{ actions: KeyboardActions }>();

  const handleKeydown = (event: KeyboardEvent) => {
    if (isTextEntry(event.target) || isTextEntry(document.activeElement))
      return;
    if (get(shortcutHelpOpen) || event.defaultPrevented) return;
    const context = get(shortcutContext);
    if (context === 'viewer' || context === 'slideshow') return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;

    const pressed =
      event.key.length === 1 ? event.key.toLowerCase() : event.key;

    try {
      const action = actions[pressed] ?? actions[event.key];
      if (action) {
        event.preventDefault();
        action();
      }
    } catch (error) {
      console.error(
        `Error executing keyboard action: [key: ${event.key}]`,
        error,
      );
    }
  };
</script>

<svelte:window onkeydown={handleKeydown} />

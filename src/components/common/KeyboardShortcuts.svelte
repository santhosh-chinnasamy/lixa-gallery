<script lang="ts">
  import type { KeyboardActions } from '../../types/events';

  let { actions } = $props<{ actions: KeyboardActions }>();

  const handleKeydown = (event: KeyboardEvent) => {
    // Ignore shortcuts if the user is typing in an input field
    if (
      document.activeElement instanceof HTMLInputElement ||
      document.activeElement instanceof HTMLTextAreaElement ||
      (document.activeElement as HTMLElement)?.isContentEditable
    ) {
      return;
    }

    try {
      const action = actions[event.key];
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

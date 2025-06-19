<script lang="ts">
  import type { KeyboardActions } from '../../types/events';

  let { actions } = $props<{ actions: KeyboardActions }>();

  const handleKeydown = (event: KeyboardEvent) => {
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
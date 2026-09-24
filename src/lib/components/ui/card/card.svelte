<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';
  import type { Snippet } from 'svelte';
  import { cn } from '$lib/utils.js';

  let {
    class: className,
    children,
    ...restProps
  }: HTMLAttributes<HTMLDivElement> & { children?: Snippet } = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class={cn('rounded-xl border bg-card text-card-foreground shadow', className)}
  {...restProps}
>
  {@render children?.()}
</div>

<script lang="ts">
  let { chords }: { chords: string[][] } = $props();

  const kbdClass =
    'inline-flex min-w-[24px] items-center justify-center rounded-md border border-border bg-muted px-2 py-1 font-mono text-xs font-semibold text-foreground';
</script>

<div class="flex shrink-0 items-center gap-1.5">
  {#each chords as chord, chordIndex (chordIndex)}
    {#if chordIndex > 0}
      <span class="text-xs text-muted-foreground">or</span>
    {/if}
    <span class="flex items-center gap-1">
      {#each chord as cap, capIndex (capIndex)}
        {#if capIndex > 0}
          <span class="text-xs text-muted-foreground">+</span>
        {/if}
        <kbd class={kbdClass}>{cap}</kbd>
      {/each}
    </span>
  {/each}
</div>

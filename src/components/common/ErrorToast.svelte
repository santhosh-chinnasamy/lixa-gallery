<script lang="ts">
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import X from '@lucide/svelte/icons/x';
  import { toasts } from '../../stores/toastStore';

  function runAction(id: number, action: { label: string; run: () => void }) {
    action.run();
    toasts.dismiss(id);
  }
</script>

{#if $toasts.length > 0}
  <div
    class="pointer-events-none fixed bottom-4 right-4 z-50 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-2"
    role="region"
    aria-label="Notifications"
  >
    {#each $toasts as toast (toast.id)}
      <div
        class="pointer-events-auto flex items-start gap-3 rounded-xl border border-destructive/30 bg-background px-4 py-3 text-sm shadow-lg"
        role="alert"
      >
        <CircleAlert size={18} class="mt-0.5 shrink-0 text-destructive" />
        <p class="min-w-0 flex-1 break-words text-foreground">
          {toast.message}
        </p>
        {#if toast.action}
          <button
            type="button"
            class="shrink-0 font-medium text-primary underline-offset-2 hover:underline"
            onclick={() => toast.action && runAction(toast.id, toast.action)}
          >
            {toast.action.label}
          </button>
        {/if}
        <button
          type="button"
          class="shrink-0 rounded-md p-1 text-muted-foreground hover:bg-accent"
          aria-label="Dismiss"
          onclick={() => toasts.dismiss(toast.id)}
        >
          <X size={14} />
        </button>
      </div>
    {/each}
  </div>
{/if}

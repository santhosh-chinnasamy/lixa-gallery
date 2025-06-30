<script lang="ts">
  export let name: string = '';
</script>

<div class="mt-2 text-center">
  <div class="text-sm text-white/70">
    {name ?? 'Unknown'}
  </div>
</div>

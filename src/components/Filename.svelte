<script lang="ts">
  export let selectedImage: string = '';
</script>

<div class="mt-2 text-center">
  <div class="text-sm text-white/70">
    {selectedImage?.split('/').pop() || 'Unknown'}
  </div>
</div>

<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { currentPage, totalPages, paginationControls, memoryStats } from '../stores/paginationStore';
  import ChevronLeft from './icons/ChevronLeft.svelte';
  import ChevronRight from './icons/ChevronRight.svelte';
  import { onMount } from 'svelte';

  let pageInput = '';
  
  $: {
    pageInput = String($currentPage + 1);
  }

  const handlePageInput = () => {
    const page = parseInt(pageInput) - 1;
    if (!isNaN(page) && page >= 0 && page < $totalPages) {
      paginationControls.goToPage(page);
    } else {
      pageInput = String($currentPage + 1);
    }
  };

  const handleKeyPress = (event: KeyboardEvent) => {
    if (event.key === 'Enter') {
      handlePageInput();
    }
  };

  // Global keyboard navigation for pagination
  const handleGlobalKeyPress = (event: KeyboardEvent) => {
    if (event.target instanceof HTMLInputElement) return; // Don't interfere with input fields
    
    switch (event.key) {
      case 'ArrowLeft':
      case 'h':
        if ($currentPage > 0) {
          event.preventDefault();
          paginationControls.prevPage();
        }
        break;
      case 'ArrowRight':
      case 'l':
        if ($currentPage < $totalPages - 1) {
          event.preventDefault();
          paginationControls.nextPage();
        }
        break;
      case 'Home':
        event.preventDefault();
        paginationControls.goToPage(0);
        break;
      case 'End':
        event.preventDefault();
        paginationControls.goToPage($totalPages - 1);
        break;
    }
  };

  onMount(() => {
    document.addEventListener('keydown', handleGlobalKeyPress);
    return () => {
      document.removeEventListener('keydown', handleGlobalKeyPress);
    };
  });
</script>

{#if $totalPages > 1}
  <div class="flex items-center justify-between gap-4 p-4 bg-white/80 backdrop-blur-sm border-t border-gray-200">
    <!-- Memory stats -->
    <div class="hidden sm:flex items-center gap-4 text-sm text-gray-600">
      <span>Total: {$memoryStats.totalImages.toLocaleString()} images</span>
      <span class="text-green-600">Page {$memoryStats.currentPage} of {$memoryStats.totalPages}</span>
    </div>

    <!-- Pagination controls -->
    <div class="flex items-center gap-2">
      <Button
        variant="outline"
        size="sm"
        disabled={$currentPage === 0}
        on:click={paginationControls.prevPage}
        class="h-8 w-8 p-0"
      >
        <ChevronLeft class="h-4 w-4" />
      </Button>

      <div class="flex items-center gap-2">
        <Input
          type="number"
          min="1"
          max={$totalPages}
          bind:value={pageInput}
          on:blur={handlePageInput}
          on:keypress={handleKeyPress}
          class="h-8 w-16 text-center text-sm"
        />
        <span class="text-sm text-gray-500">of {$totalPages}</span>
      </div>

      <Button
        variant="outline"
        size="sm"
        disabled={$currentPage >= $totalPages - 1}
        on:click={paginationControls.nextPage}
        class="h-8 w-8 p-0"
      >
        <ChevronRight class="h-4 w-4" />
      </Button>
    </div>

    <!-- Quick jump buttons -->
    <div class="hidden md:flex items-center gap-1">
      <Button
        variant="ghost"
        size="sm"
        disabled={$currentPage === 0}
        on:click={() => paginationControls.goToPage(0)}
        class="h-8 px-2 text-xs"
      >
        First
      </Button>
      <Button
        variant="ghost"
        size="sm"
        disabled={$currentPage >= $totalPages - 1}
        on:click={() => paginationControls.goToPage($totalPages - 1)}
        class="h-8 px-2 text-xs"
      >
        Last
      </Button>
    </div>
  </div>
{/if}

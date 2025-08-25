<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { memoryStats } from '../stores/paginationStore';

  let performanceData = {
    memoryUsage: 0,
    loadTime: 0,
    fps: 0,
    imagesLoaded: 0
  };

  let performanceObserver: PerformanceObserver | null = null;
  let frameCount = 0;
  let lastTime = 0;
  let animationFrame: number;

  function updateFPS() {
    frameCount++;
    const now = performance.now();
    
    if (now - lastTime >= 1000) {
      performanceData.fps = Math.round((frameCount * 1000) / (now - lastTime));
      frameCount = 0;
      lastTime = now;
    }
    
    animationFrame = requestAnimationFrame(updateFPS);
  }

  function getMemoryUsage() {
    if ('memory' in performance) {
      const memory = (performance as any).memory;
      performanceData.memoryUsage = Math.round(memory.usedJSHeapSize / 1024 / 1024);
    }
  }

  onMount(() => {
    // Start FPS monitoring
    lastTime = performance.now();
    updateFPS();

    // Memory usage monitoring
    const memoryInterval = setInterval(getMemoryUsage, 1000);

    // Performance observer for load times
    if ('PerformanceObserver' in window) {
      performanceObserver = new PerformanceObserver((list) => {
        const entries = list.getEntries();
        entries.forEach((entry) => {
          if (entry.entryType === 'measure' && entry.name.includes('image-load')) {
            performanceData.loadTime = Math.round(entry.duration);
          }
        });
      });
      
      performanceObserver.observe({ entryTypes: ['measure'] });
    }

    return () => {
      clearInterval(memoryInterval);
      if (performanceObserver) {
        performanceObserver.disconnect();
      }
    };
  });

  onDestroy(() => {
    if (animationFrame) {
      cancelAnimationFrame(animationFrame);
    }
  });

  // Subscribe to memory stats from pagination store
  $: performanceData.imagesLoaded = $memoryStats.loadedImages;
</script>

{#if import.meta.env.DEV}
  <div class="fixed bottom-4 right-4 bg-black/80 text-white text-xs p-3 rounded-lg font-mono z-50">
    <div class="space-y-1">
      <div>Memory: {performanceData.memoryUsage}MB</div>
      <div>FPS: {performanceData.fps}</div>
      <div>Images: {performanceData.imagesLoaded}</div>
      <div>Page: {$memoryStats.currentPage}/{$memoryStats.totalPages}</div>
      <div>Total: {$memoryStats.totalImages.toLocaleString()}</div>
    </div>
  </div>
{/if}

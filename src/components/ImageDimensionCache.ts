// Image dimension caching to prevent layout shifts
export class ImageDimensionCache {
  private cache = new Map<string, { width: number; height: number; aspectRatio: number }>();
  private static instance: ImageDimensionCache;

  static getInstance(): ImageDimensionCache {
    if (!ImageDimensionCache.instance) {
      ImageDimensionCache.instance = new ImageDimensionCache();
    }
    return ImageDimensionCache.instance;
  }

  async getDimensions(imagePath: string): Promise<{ width: number; height: number; aspectRatio: number }> {
    // Check cache first
    if (this.cache.has(imagePath)) {
      return this.cache.get(imagePath)!;
    }

    // Load image to get dimensions
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => {
        const dimensions = {
          width: img.width,
          height: img.height,
          aspectRatio: img.width / img.height
        };
        
        // Cache the result
        this.cache.set(imagePath, dimensions);
        resolve(dimensions);
      };
      
      img.onerror = () => {
        reject(new Error(`Failed to load image: ${imagePath}`));
      };
      
      img.src = imagePath;
    });
  }

  preloadDimensions(imagePaths: string[]): void {
    imagePaths.forEach(path => {
      if (!this.cache.has(path)) {
        this.getDimensions(path).catch(() => {
          // Silently fail for preloading
        });
      }
    });
  }

  clear(): void {
    this.cache.clear();
  }

  getCacheSize(): number {
    return this.cache.size;
  }
}

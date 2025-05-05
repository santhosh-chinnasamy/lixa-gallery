import { convertFileSrc } from "@tauri-apps/api/core";

export type Photo = string;

export class Gallery {
  private photos: Photo[];
  private galleryElement: HTMLElement;
  private overlayElement: HTMLElement;
  private previewImageElement: HTMLImageElement;
  private topLoadingSpinner: HTMLElement;
  private loadingSpinner: HTMLElement;
  private activeIndex: number | null = null;

  constructor(photos: Photo[]) {
    this.photos = photos;
    this.galleryElement = document.getElementById('gallery') as HTMLElement;
    this.overlayElement = document.getElementById('preview-overlay') as HTMLElement;
    this.previewImageElement = document.getElementById('preview-image') as HTMLImageElement;
    this.topLoadingSpinner = document.getElementById('top-loading-spinner') as HTMLElement;
    this.loadingSpinner = document.getElementById('loading-spinner') as HTMLElement;

    this.initGallery();
    this.initEventListeners();
  }

  private initGallery(): void {
    // Create thumbnails
    this.galleryElement.innerHTML = '';
    this.topLoadingSpinner.classList.add('active');
    this.photos.forEach((photo, index) => {
      const thumbnailElement = document.createElement('div');
      thumbnailElement.className = 'gallery-item';
      thumbnailElement.innerHTML = `
        <div class="thumbnail-container">
          <img src="${convertFileSrc(photo)}" alt="${photo}" class="thumbnail-image" loading="lazy">
        </div>
      `;

      thumbnailElement.addEventListener('click', () => this.openPreview(index));
      this.galleryElement.appendChild(thumbnailElement);
    });
  }

  private initEventListeners(): void {
    // Close button
    const closeButton = document.getElementById('close-button') as HTMLElement;
    closeButton.addEventListener('click', () => this.closePreview());

    // Navigation buttons
    const prevButton = document.getElementById('prev-button') as HTMLElement;
    const nextButton = document.getElementById('next-button') as HTMLElement;

    prevButton.addEventListener('click', () => this.goToPrevious());
    nextButton.addEventListener('click', () => this.goToNext());

    // Keyboard navigation
    document.addEventListener('keydown', (e) => {
      if (this.activeIndex === null) return;

      switch (e.key) {
        case 'Escape':
          this.closePreview();
          break;
        case 'ArrowLeft':
          this.goToPrevious();
          break;
        case 'ArrowRight':
          this.goToNext();
          break;
      }
    });

    const handleSwipe = (): void => {
      const swipeThreshold = 50;

      if (touchEndX - touchStartX > swipeThreshold) {
        this.goToPrevious(); // Swipe right
      } else if (touchStartX - touchEndX > swipeThreshold) {
        this.goToNext(); // Swipe left
      }
    };

    // Touch swipe navigation on the preview image
    let touchStartX: number = 0;
    let touchEndX: number = 0;

    this.previewImageElement.addEventListener('touchstart', (e) => {
      touchStartX = e.changedTouches[0].screenX;
    });

    this.previewImageElement.addEventListener('touchend', (e) => {
      touchEndX = e.changedTouches[0].screenX;
      handleSwipe();
    });


  }

  private openPreview(index: number): void {

    this.activeIndex = index;
    this.loadingSpinner.classList.add('active');
    this.overlayElement.classList.add('active');
    document.body.classList.add('no-scroll');

    this.updatePreviewContent();
    this.preloadImages();
  }

  private closePreview(): void {
    this.overlayElement.classList.remove('active');
    document.body.classList.remove('no-scroll');
    this.activeIndex = null;
  }

  private goToPrevious(): void {
    if (this.activeIndex === null) return;

    this.loadingSpinner.classList.add('active');
    this.activeIndex = this.activeIndex === 0 ? this.photos.length - 1 : this.activeIndex - 1;
    this.updatePreviewContent();
    this.preloadImages();
  }

  private goToNext(): void {
    if (this.activeIndex === null) return;

    this.loadingSpinner.classList.add('active');
    this.activeIndex = this.activeIndex === this.photos.length - 1 ? 0 : this.activeIndex + 1;
    this.updatePreviewContent();
    this.preloadImages();
  }

  private updatePreviewContent(): void {
    if (this.activeIndex === null) return;

    const photo = this.photos[this.activeIndex];

    // Set image source to trigger loading
    this.previewImageElement.src = convertFileSrc(photo);
    this.previewImageElement.alt = photo;
    this.previewImageElement.classList.add('loading');

    // Update info
    // this.previewTitleElement.textContent = photo.title;
    // this.previewDescriptionElement.textContent = photo.description;

    /*  if (photo.location) {
       this.previewLocationElement.textContent = `📍 ${photo.location}`;
       this.previewLocationElement.style.display = 'block';
     } else {
       this.previewLocationElement.style.display = 'none';
     } */

    // Handle image load complete
    this.previewImageElement.onload = () => {

      this.loadingSpinner.classList.remove('active');
      this.previewImageElement.classList.remove('loading');
      this.previewImageElement.classList.add('loaded');
    };
  }

  private preloadImages(): void {
    if (this.activeIndex === null) return;

    // Preload next and previous images
    const nextIndex = this.activeIndex === this.photos.length - 1 ? 0 : this.activeIndex + 1;
    const prevIndex = this.activeIndex === 0 ? this.photos.length - 1 : this.activeIndex - 1;

    [nextIndex, prevIndex].forEach(index => {
      const preloadImage = new Image();
      preloadImage.src = convertFileSrc(this.photos[index]);
    });
  }
}
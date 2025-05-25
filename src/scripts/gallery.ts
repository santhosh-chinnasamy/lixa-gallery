import { convertFileSrc } from "@tauri-apps/api/core";
import { FavouriteManager } from "./favourite";

export type Photo = string;

interface DOMElements {
  gallery: HTMLElement;
  overlay: HTMLElement;
  previewImage: HTMLImageElement;
  topLoadingSpinner: HTMLElement;
  loadingSpinner: HTMLElement;
  heartCheckbox: HTMLInputElement;
  closeButton: HTMLElement;
  prevButton: HTMLElement;
  nextButton: HTMLElement;
}

export class Gallery {
  private readonly photos: Photo[];
  private readonly elements: DOMElements;
  private readonly favourites: FavouriteManager;
  private activeIndex: number | null = null;
  private readonly SWIPE_THRESHOLD = 50;

  constructor(photos: Photo[]) {
    this.photos = photos;
    this.elements = this.initializeDOMElements();
    this.favourites = new FavouriteManager();

    this.initGallery();
    this.initEventListeners();
  }

  private initializeDOMElements(): DOMElements {
    const getElement = <T extends HTMLElement>(id: string): T => {
      const element = document.getElementById(id);
      if (!element) throw new Error(`Element with id '${id}' not found`);
      return element as T;
    };

    const getHeartCheckbox = (): HTMLInputElement => {
      const checkbox = document.getElementsByName("heart-checkbox")[0];
      if (!checkbox || !(checkbox instanceof HTMLInputElement)) {
        throw new Error("Heart checkbox not found");
      }
      return checkbox;
    };

    return {
      gallery: getElement("gallery"),
      overlay: getElement("preview-overlay"),
      previewImage: getElement("preview-image"),
      topLoadingSpinner: getElement("top-loading-spinner"),
      loadingSpinner: getElement("loading-spinner"),
      heartCheckbox: getHeartCheckbox(),
      closeButton: getElement("close-button"),
      prevButton: getElement("prev-button"),
      nextButton: getElement("next-button"),
    };
  }

  private async initGallery(): Promise<void> {
    try {
      await this.createThumbnails();
      await this.initializeFavorites();
    } catch (error) {
      console.error("Failed to initialize gallery:", error);
      throw new Error("Gallery initialization failed");
    }
  }

  private createThumbnails(): void {
    this.elements.gallery.innerHTML = "";
    if (this.photos.length === 0) {
      this.elements.gallery.innerHTML =
        '<div class="no-photos">No photos found</div>';
      return;
    }

    this.elements.topLoadingSpinner.classList.add("active");
    const fragment = document.createDocumentFragment();

    this.photos.forEach((photo, index) => {
      const thumbnailElement = document.createElement("div");
      thumbnailElement.className = "gallery-item";
      thumbnailElement.tabIndex = index + 1;
      thumbnailElement.innerHTML = `
        <div class="thumbnail-container">
          <img src="${convertFileSrc(
            photo
          )}" alt="${photo}" class="thumbnail-image" loading="lazy" id="thumbnail-image-${photo}" />
        </div>
      `;

      thumbnailElement.addEventListener("click", () => this.openPreview(index));
      thumbnailElement.addEventListener("keydown", (e) => {
        if (e.key === "Enter") {
          this.openPreview(index);
        }
      });

      fragment.appendChild(thumbnailElement);
      this.activeIndex = index;
      this.toggleFavourite(true);
    });

    this.elements.gallery.appendChild(fragment);
  }

  private async initializeFavorites(): Promise<void> {
    try {
      const favourites = await this.favourites.getFavourites();
      favourites.forEach((favourite) => {
        const thumbnailImage = document.getElementById(
          `thumbnail-image-${favourite}`
        ) as HTMLImageElement;

        if (thumbnailImage) {
          thumbnailImage.classList.add("favourite");
        }
      });
    } catch (error) {
      console.error("Failed to initialize favorites:", error);
      throw new Error("Favorites initialization failed");
    }
  }
  private initEventListeners(): void {
    this.initKeyboardEvents();
    this.initButtonEvents();
    this.initTouchEvents();
    this.initFavoriteEvents();
  }

  private initKeyboardEvents(): void {
    document.addEventListener("keydown", (event) => {
      switch (event.key) {
        case "Escape":
          this.closePreview();
          break;
        case "ArrowLeft":
          this.goToPrevious();
          break;
        case "ArrowRight":
          this.goToNext();
          break;
        case "l":
          this.toggleFavourite();
          break;
        case " ":
          this.closePreview();
          break;
      }
    });
  }

  private initButtonEvents(): void {
    this.elements.closeButton.addEventListener("click", () =>
      this.closePreview()
    );
    this.elements.prevButton.addEventListener("click", () =>
      this.goToPrevious()
    );
    this.elements.nextButton.addEventListener("click", () => this.goToNext());
  }

  private initTouchEvents(): void {
    let touchStartX: number = 0;
    let touchEndX: number = 0;

    this.elements.previewImage.addEventListener("touchstart", (e) => {
      touchStartX = e.changedTouches[0].screenX;
    });

    this.elements.previewImage.addEventListener("touchend", (e) => {
      touchEndX = e.changedTouches[0].screenX;
      this.handleSwipe(touchStartX, touchEndX);
    });
  }

  private handleSwipe(startX: number, endX: number): void {
    const swipeDistance = endX - startX;
    if (Math.abs(swipeDistance) < this.SWIPE_THRESHOLD) return;

    if (swipeDistance > 0) {
      this.goToPrevious();
    } else {
      this.goToNext();
    }
  }

  private initFavoriteEvents(): void {
    this.elements.heartCheckbox.addEventListener("change", () =>
      this.toggleFavourite()
    );
  }

  private async openPreview(index: number): Promise<void> {
    try {
      if (index < 0 || index >= this.photos.length) {
        throw new Error(`Invalid photo index: ${index}`);
      }

      this.activeIndex = index;
      this.elements.loadingSpinner.classList.add("active");
      this.elements.overlay.classList.add("active");
      document.body.classList.add("no-scroll");

      await this.updatePreviewContent();
      this.preloadImages();
    } catch (error) {
      console.error("Failed to open preview:", error);
      this.closePreview();
    }
  }

  private closePreview(): void {
    this.elements.overlay.classList.remove("active");
    document.body.classList.remove("no-scroll");
    this.elements.previewImage.src = "";
    this.elements.previewImage.classList.remove("loading", "loaded");
    this.activeIndex = null;
  }

  private async goToPrevious(): Promise<void> {
    if (!this.canNavigate()) return;

    try {
      this.elements.loadingSpinner.classList.add("active");
      this.activeIndex = this.calculatePreviousIndex();
      await this.updatePreviewContent();
      this.preloadImages();
    } catch (error) {
      console.error("Failed to navigate to previous image:", error);
      this.elements.loadingSpinner.classList.remove("active");
    }
  }

  private async goToNext(): Promise<void> {
    if (!this.canNavigate()) return;

    try {
      this.elements.loadingSpinner.classList.add("active");
      this.activeIndex = this.calculateNextIndex();
      await this.updatePreviewContent();
      this.preloadImages();
    } catch (error) {
      console.error("Failed to navigate to next image:", error);
      this.elements.loadingSpinner.classList.remove("active");
    }
  }

  private calculatePreviousIndex(): number {
    return this.activeIndex === 0
      ? this.photos.length - 1
      : this.activeIndex! - 1;
  }

  private calculateNextIndex(): number {
    return this.activeIndex === this.photos.length - 1
      ? 0
      : this.activeIndex! + 1;
  }

  private canNavigate(): boolean {
    return this.activeIndex !== null && this.photos.length > 0;
  }

  private async updatePreviewContent(): Promise<void> {
    if (this.activeIndex === null) return;

    const photo = this.photos[this.activeIndex];
    try {
      const isFavourite = await this.favourites.isFavourite(photo);
      this.elements.heartCheckbox.checked = isFavourite;

      this.elements.previewImage.src = convertFileSrc(photo);
      this.elements.previewImage.alt = `Preview of ${photo}`;
      this.elements.previewImage.classList.add("loading");

      await this.waitForImageLoad();
    } catch (error) {
      console.error("Failed to update preview content:", error);
      throw error;
    }
  }

  private waitForImageLoad(): Promise<void> {
    return new Promise((resolve) => {
      this.elements.previewImage.onload = () => {
        this.elements.loadingSpinner.classList.remove("active");
        this.elements.previewImage.classList.remove("loading");
        this.elements.previewImage.classList.add("loaded");
        resolve();
      };
    });
  }

  private preloadImages(): void {
    if (!this.canNavigate()) return;

    const indicesToPreload = [
      this.calculatePreviousIndex(),
      this.calculateNextIndex(),
    ];

    indicesToPreload.forEach((index) => {
      const preloadImage = new Image();
      preloadImage.src = convertFileSrc(this.photos[index]);
    });
  }

  private async toggleFavourite(isFirstLoad: boolean = false): Promise<void> {
    try {
      if (!this.canNavigate()) return;

      const photo = this.photos[this.activeIndex!];
      const thumbnailImage = document.getElementById(
        `thumbnail-image-${photo}`
      ) as HTMLImageElement;

      // if (!thumbnailImage) {
      //   throw new Error(`Thumbnail image not found for photo: ${photo}`);
      // }

      const isFavourite = await this.favourites.isFavourite(photo);

      if (isFavourite) {
        await this.removeFavourite(photo, thumbnailImage);
      } else if (!isFirstLoad) {
        await this.addFavourite(photo, thumbnailImage);
      }
    } catch (error) {
      console.error("Failed to toggle favourite:", error);
    }
  }

  private async removeFavourite(
    photo: Photo,
    thumbnailImage: HTMLImageElement
  ): Promise<void> {
    await this.favourites.removeFromFavourites(photo);
    this.elements.heartCheckbox.checked = false;
    thumbnailImage.classList.remove("favourite");
  }

  private async addFavourite(
    photo: Photo,
    thumbnailImage: HTMLImageElement
  ): Promise<void> {
    await this.favourites.addToFavourites(photo);
    this.elements.heartCheckbox.checked = true;
    thumbnailImage.classList.add("favourite");
  }
}

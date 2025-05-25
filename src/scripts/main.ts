import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { attachConsole } from "@tauri-apps/plugin-log";
import { Gallery, Photo } from "./gallery";

class AppInitializer {
  constructor() {
    this.initializeApp();
  }

  private async initializeApp(): Promise<void> {
    try {
      await attachConsole();
      this.initializeEventListeners();
      this.updateCopyrightYear();
    } catch (error) {
      console.error("Failed to initialize app:", error);
    }
  }

  private initializeEventListeners(): void {
    const folderSelector = document.querySelector<HTMLButtonElement>(
      "#folder-selector-button"
    );
    if (!folderSelector) {
      throw new Error("Folder selector button not found");
    }

    folderSelector.addEventListener("click", () => this.loadPhotos());
    document.addEventListener("keydown", (e) => {
      if (e.key === "o") {
        this.loadPhotos();
      }
    });
  }

  private async loadPhotos(): Promise<void> {
    try {
      const folder = await open({
        multiple: false,
        directory: true,
      });

      if (!folder) {
        console.log("No folder selected");
        return;
      }

      const imagesContainer =
        document.querySelector<HTMLDivElement>("#images-container");

      if (imagesContainer) imagesContainer.innerHTML = "";

      const photos: Photo[] = await invoke("scan_folder", { path: folder });
      if (!photos.length && imagesContainer) {
        imagesContainer.innerHTML =
          '<div class="no-photos">No photos found in selected folder</div>';
        return;
      }

      new Gallery(photos).load();
    } catch (error) {
      console.error("Failed to load photos:", error);
      this.handleError(error);
    }
  }

  private updateCopyrightYear(): void {
    const currentYear =
      document.querySelector<HTMLSpanElement>("#current-year");
    if (currentYear) {
      currentYear.textContent = new Date().getFullYear().toString();
    }
  }

  private handleError(error: unknown): void {
    const message =
      error instanceof Error ? error.message : "An unexpected error occurred";
    const imagesContainer =
      document.querySelector<HTMLDivElement>("#images-container");
    if (imagesContainer) {
      imagesContainer.innerHTML = `<div class="error-message">${message}</div>`;
    }
  }
}

window.addEventListener("DOMContentLoaded", () => new AppInitializer());

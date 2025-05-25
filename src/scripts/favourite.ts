import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface Favourite {
  path: string;
}

export class FavouriteManager {
  private static readonly EXPORT_BUTTON_ID = "export-favourites-button";
  private favourites: Favourite[] = [];
  private exportFavouritesButton: HTMLButtonElement;

  constructor() {
    const button = document.getElementById(FavouriteManager.EXPORT_BUTTON_ID);
    if (!button) {
      throw new Error(`Export button with ID '${FavouriteManager.EXPORT_BUTTON_ID}' not found`);
    }
    this.exportFavouritesButton = button as HTMLButtonElement;
    this.initializeExportButton();
    this.toggleExportFavouritesButton().catch(this.handleError);
  }

  private initializeExportButton(): void {
    this.exportFavouritesButton.addEventListener("click", async () => {
      try {
        await this.exportFavourites();
        this.showSuccessMessage("Favourites exported successfully");
      } catch (error) {
        this.handleError(error);
      }
    });
  }

  public async addToFavourites(filePath: string): Promise<void> {
    try {
      if (!filePath?.trim()) {
        throw new Error("File path cannot be empty");
      }
      await invoke("add_favourite", { path: filePath });
      await this.toggleExportFavouritesButton();
    } catch (error) {
      this.handleError(error);
    }
  }

  public async removeFromFavourites(filePath: string): Promise<void> {
    try {
      if (!filePath?.trim()) {
        throw new Error("File path cannot be empty");
      }
      await invoke("remove_favourite", { path: filePath });
      await this.toggleExportFavouritesButton();
    } catch (error) {
      this.handleError(error);
    }
  }

  public async getFavourites(): Promise<Favourite[]> {
    try {
      this.favourites = await invoke("get_favourites");
      return this.favourites;
    } catch (error) {
      this.handleError(error);
      return [];
    }
  }

  public isFavourite(filePath: string): boolean {
    if (!filePath?.trim()) return false;
    return this.favourites.some(favourite => favourite.path === filePath);
  }

  private async toggleExportFavouritesButton(): Promise<void> {
    const favourites = await this.getFavourites();
    this.exportFavouritesButton.disabled = favourites.length === 0;
  }

  private async exportFavourites(): Promise<void> {
    const destination = await open({
      multiple: false,
      directory: true,
    });

    if (!destination) {
      throw new Error("No destination selected for export");
    }

    await invoke("export_favourites", { destination });
  }

  private showSuccessMessage(message: string): void {
    alert(message);
  }

  private handleError(error: unknown): void {
    const errorMessage = error instanceof Error ? error.message : 'An unexpected error occurred';
    console.error('Favourite operation failed:', errorMessage);
    alert(`Error: ${errorMessage}`);
  }
}

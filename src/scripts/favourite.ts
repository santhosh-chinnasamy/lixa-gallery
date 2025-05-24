import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface Favourite {
  path: string;
}

export class Favourite {
  private favourites: Favourite[];
  private exportFavouritesButton: HTMLButtonElement;

  constructor() {
    this.favourites = [];
    this.exportFavouritesButton = document.getElementById(
      "export-favourites-button"
    ) as HTMLButtonElement;

    this.exportFavouritesButton.addEventListener("click", async () => {
      await this.exportFavourites();
      alert("Favourites exported to selected folder");
    });

    this.toggleExportFavouritesButton();
  }

  public async addToFavourites(filePath: string): Promise<void> {
    await invoke("add_favourite", {
      path: filePath,
    });
    this.toggleExportFavouritesButton();
  }

  public async removeFromFavourites(filePath: string): Promise<void> {
    await invoke("remove_favourite", {
      path: filePath,
    });
    this.toggleExportFavouritesButton();
  }

  public async getFavourites(): Promise<Favourite[]> {
    const favourites: Favourite[] = await invoke("get_favourites");
    this.favourites = favourites;
    return favourites;
  }

  public isFavourite(filePath: string): boolean {
    return this.favourites.some((favourite) => favourite.path === filePath);
  }

  public async toggleExportFavouritesButton(): Promise<void> {
    const favourites = await this.getFavourites();
    if (favourites.length > 0) {
      this.exportFavouritesButton.removeAttribute("disabled");
    } else {
      this.exportFavouritesButton.setAttribute("disabled", "disabled");
    }
  }

  private async exportFavourites(): Promise<void> {
    const destination = await open({
      multiple: false,
      directory: true,
    });

    await invoke("export_favourites", {
      destination,
    });
  }
}

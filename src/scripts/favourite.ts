import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export class Favourite {
    private favourites: string[];
    private exportFavouritesButton: HTMLButtonElement;

    constructor() {
        this.favourites = []
        this.exportFavouritesButton = document.getElementById('export-favourites-button') as HTMLButtonElement;

        this.exportFavouritesButton.addEventListener('click', async () => {
            await this.exportFavourites();
            alert('Favourites exported to selected folder');
        })
    }

    public addToFavourites(filePath: string): void {
        const favourites = this.getFavourites();
        favourites.push(filePath);
        this.toggleExportFavouritesButton();
    }

    public removeFromFavourites(filePath: string): void {
        const favourites = this.getFavourites();
        const index = favourites.indexOf(filePath);
        if (index !== -1) {
            favourites.splice(index, 1);

        }
        this.toggleExportFavouritesButton();
    }

    public getFavourites(): string[] {
        return this.favourites;
    }

    public setFavourites(favourites: string[]): void {
        this.favourites = favourites;
    }

    public isFavourite(filePath: string): boolean {
        return this.favourites.includes(filePath);
    }

    public toggleExportFavouritesButton(): void {
        console.log('toggleExportFavouritesButton', this.getFavourites().length);

        if (this.getFavourites().length > 0) {
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

        const names = await invoke("export_favourites", {
            destination,
            files: this.getFavourites(),
        });

        console.log('names', names);
    }
}


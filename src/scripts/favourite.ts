export class Favourite {
    private favourites: string[];

    constructor() {
        this.favourites = []
    }

    public addToFavourites(filePath: string): void {
        const favourites = this.getFavourites();
        favourites.push(filePath);
    }

    public removeFromFavourites(filePath: string): void {
        const favourites = this.getFavourites();
        const index = favourites.indexOf(filePath);
        if (index !== -1) {
            favourites.splice(index, 1);

        }
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
}


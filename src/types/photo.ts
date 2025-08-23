export interface PhotoMetadata {
    metadata: FileMetadata,
    thumbnail_path: string,
    path: string,
}

export interface FileMetadata {
    name: string,
    modified: number,
    created: number,
    size: number,
}
import { writable, type Writable } from "svelte/store";

export const photos: Writable<string[]> = writable([]);
export const isLoading: Writable<boolean> = writable(false);

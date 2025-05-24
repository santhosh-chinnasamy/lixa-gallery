import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Gallery, Photo } from "./gallery";
import { attachConsole } from "@tauri-apps/plugin-log";

const loadPhotos = async () => {
  const folder = await open({
    multiple: false,
    directory: true,
  });

  const imagesContainer: HTMLDivElement | null =
    document.querySelector("#images-container");

  if (imagesContainer) {
    // reset the container
    imagesContainer.innerHTML = "";
  }

  const photos: Photo[] = await invoke("scan_folder", {
    path: folder,
  });

  new Gallery(photos);
};

window.addEventListener("DOMContentLoaded", async () => {
  await attachConsole();
  const folderSelector: HTMLButtonElement | null = document.querySelector(
    "#folder-selector-button"
  );

  folderSelector?.addEventListener("click", loadPhotos);

  document.addEventListener("keydown", (e) => {
    if (e.key === "o") {
      loadPhotos();
    }
  });

  const currentYear = document.querySelector(
    "#current-year"
  ) as HTMLSpanElement;

  if (currentYear) {
    currentYear.textContent = new Date().getFullYear().toString();
  }
});

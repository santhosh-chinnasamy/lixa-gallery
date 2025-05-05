import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Gallery, Photo } from './gallery';

window.addEventListener("DOMContentLoaded", () => {
  const folderSelector: HTMLButtonElement | null = document.querySelector(
    "#folder-selector-button"
  );

  folderSelector?.addEventListener("click", async () => {
    const folder = await open({
      multiple: false,
      directory: true,
    });

    const imagesContainer: HTMLDivElement | null = document.querySelector(
      "#images-container"
    );

    if (imagesContainer) { // reset the container
      imagesContainer.innerHTML = "";
    }

    const photos: Photo[] = await invoke("scan_folder", {
      path: folder,
    });

    new Gallery(photos);
  });
});

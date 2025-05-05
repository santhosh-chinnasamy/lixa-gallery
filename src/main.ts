import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

window.addEventListener("DOMContentLoaded", () => {
  const folderSelector: HTMLButtonElement | null = document.querySelector(
    "#folder-selector-button"
  );

  folderSelector?.addEventListener("click", async () => {
    const folder = await open({
      multiple: false,
      directory: true,
      filters: [],
    });

    const imagesContainer: HTMLDivElement | null = document.querySelector(
      "#images-container"
    );

    if (imagesContainer) {
      imagesContainer.innerHTML = "";
    }

    const result: string[] = await invoke("scan_folder", {
      path: folder,
    });

    result.forEach((src: string) => {
      const img = document.createElement("img");
      img.src = `${convertFileSrc(src)}`;
      img.loading = "lazy";
      img.style.width = "15vw";
      img.style.height = "15vh";
      imagesContainer?.appendChild(img);
    });
  });
});

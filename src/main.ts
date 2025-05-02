import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

window.addEventListener("DOMContentLoaded", () => {
  const folderSelector: HTMLButtonElement | null =
    document.querySelector("#folder-selector-button");

  folderSelector?.addEventListener("click", async () => {
    const folder = await open({
      multiple: false,
      directory: true,
      filters: []
    });

    const greetMsgEl: HTMLElement | null = document.querySelector("#greet-msg");
    if (!greetMsgEl) return;

    greetMsgEl.textContent = await invoke("greet", {
      name: folder,
    });
  });
});

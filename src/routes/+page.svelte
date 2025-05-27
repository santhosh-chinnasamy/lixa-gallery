<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import Gallery from "../components/Gallery.svelte";
  import "../main.css";
  import { isLoading, photos } from "../stores/galleryStore";

  const APP_NAME = "Lixa Gallery";

  const loadPhotos = async () => {
    try {
      isLoading.set(true);
      const folder = await open({
        multiple: false,
        directory: true,
      });
      if (!folder) {
        isLoading.set(false);
        photos.set([]);
        return;
      }
      const loadedPhotos: string[] = await invoke("scan_folder", {
        path: folder,
      });
      photos.set(loadedPhotos);
    } catch (error) {
      console.error("Failed to load photos:", error);
    } finally {
      isLoading.set(false);
    }
  };
  // event listener on key press
  document.addEventListener("keydown", (event) => {
    if (event.key === "o") {
      loadPhotos();
    }
  });
</script>

<main class="container">
  <header class="header">
    <div>
      <h1>{APP_NAME}</h1>
      <p>Select your favorite photos and export them</p>
    </div>
    <div>
      <button class="button" on:click={loadPhotos}>Load Photos</button>
      <button class="button"> Export Favourites </button>
    </div>
  </header>

  {#if $isLoading}
    <p class="loading">Select Folder...</p>
  {:else}
    <Gallery />
  {/if}

  <footer class="footer">
    <p>
      <a
        href="http://github.com/santhosh-chinnasamy/lixa-gallery"
        target="_blank"
      >
        {APP_NAME}
      </a>
      &copy; {new Date().getFullYear()}
    </p>
  </footer>
</main>

<style>
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: white;
    box-shadow: var(--shadow-sm);
    padding: var(--space-6) var(--space-4);
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .header h1 {
    font-size: 2rem;
    margin: 0;
  }
  .header p {
    font-size: 1rem;
    margin: 0;
  }
  .header button {
    font-size: 1rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.25rem;
    background-color: var(--color-primary-900);
    color: #fff;
    cursor: pointer;
  }
  .header button:hover {
    background-color: #0069d9;
  }

  .loading {
    display: flex;
    justify-content: center;
    height: 100vh;
    font-size: 2rem;
    font-weight: bold;
    color: var(--color-gray-500);
  }

  .footer {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 1rem;
    position: sticky;
    bottom: 0;
    background-color: none;
  }
  .footer p {
    font-size: 1rem;
    margin: 0;
  }
</style>

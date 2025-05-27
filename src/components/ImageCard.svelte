<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";

  let { path, tabindex } = $props();
  const dispatch = createEventDispatcher();

  function handleClick() {
    dispatch("click");
  }

  function handleKeyPress(event: KeyboardEvent) {
    if (event.key === " " || event.key === "Enter") {
      handleClick();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<main
  class="image-card"
  {tabindex}
  onclick={handleClick}
  onkeypress={handleKeyPress}
>
  <div class="image-container">
    <img src={convertFileSrc(path)} alt="" class="image" loading="lazy" />
  </div>
</main>

<style>
  .image-card {
    display: flex;
    justify-content: center;
    align-items: center;
    margin: auto;
    border-radius: 0.25rem;
    background-color: var(--color-gray-300);
    color: #fff;
  }

  .image-container {
    width: 300px;
    height: 300px;
    display: flex;
    justify-content: center;
    align-items: center;
    overflow: hidden;
  }

  .image {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .image-container:hover {
    transform: scale(1.05);
    transition: transform 0.2s ease-in-out;
    cursor: pointer;
  }

  @media (max-width: 800px) {
    .image-container {
      width: 200px;
      height: 200px;
    }
  }
</style>

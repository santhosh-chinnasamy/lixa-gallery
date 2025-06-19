<script lang="ts">
  const APP_NAME = 'Lixa Gallery';
  const REPO_URL = 'http://github.com/santhosh-chinnasamy/lixa-gallery';
</script>

<footer
  class="sticky bottom-0 flex items-center justify-center bg-transparent p-4"
>
  <p class="m-0 text-base">
    <a
      href={REPO_URL}
      target="_blank"
      class="text-primary transition-colors hover:text-primary/90"
    >
      {APP_NAME}
    </a>
    &copy; {new Date().getFullYear()}
  </p>
</footer>
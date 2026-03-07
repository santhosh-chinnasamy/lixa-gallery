<script lang="ts">
  import { page } from '$app/state';
  import * as Sidebar from '$lib/components/ui/sidebar';
  import GithubIcon from '@lucide/svelte/icons/github';
  import HeartIcon from '@lucide/svelte/icons/heart';
  import LayoutGridIcon from '@lucide/svelte/icons/layout-grid';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import UsersIcon from '@lucide/svelte/icons/users';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import HouseIcon from '@lucide/svelte/icons/house';
  import {
    favorites,
    recentFolders,
    currentFolder,
    clearPhotos,
  } from '../stores/galleryStore';
  import { loadPath } from './common/ImageOperations';
  import { goto } from '$app/navigation';

  let isRecentOpen = $state(true);

  const libraryItems = [
    {
      title: 'All Photos',
      url: '/',
      icon: LayoutGridIcon,
    },
  ];

  const cullingItems = [
    {
      title: 'Favourites',
      url: '/favourites',
      icon: HeartIcon,
    },
    {
      title: 'People',
      url: '#',
      icon: UsersIcon,
    },
  ];

  function isActive(url: string) {
    if (url === '#') return false;
    return page.url.pathname === url;
  }

  function handleHomeClick() {
    clearPhotos();
    goto('/');
  }

  function handleRecentClick(path: string) {
    loadPath(path);
    goto('/');
  }

  function toggleRecent() {
    isRecentOpen = !isRecentOpen;
  }
</script>

<Sidebar.Root variant="sidebar" collapsible="none" class="border-r bg-sidebar">
  <Sidebar.Header class="flex h-16 items-center justify-between border-b px-6">
    <button
      onclick={handleHomeClick}
      class="flex items-center gap-2 transition-opacity hover:opacity-80"
    >
      <span class="text-lg font-semibold tracking-tight">Lixa Gallery</span>
    </button>
    <button
      onclick={handleHomeClick}
      class="rounded-md p-2 text-muted-foreground transition-colors hover:bg-accent"
      title="Go to Home/Landing"
    >
      <HouseIcon size={18} />
    </button>
  </Sidebar.Header>

  <Sidebar.Content>
    <Sidebar.Group>
      <Sidebar.GroupLabel
        class="px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70"
        >Library</Sidebar.GroupLabel
      >
      <Sidebar.GroupContent>
        <Sidebar.Menu>
          {#each libraryItems as item (item.title)}
            <Sidebar.MenuItem>
              <Sidebar.MenuButton
                class={`w-full justify-start gap-3 px-4 py-2 transition-colors ${isActive(item.url) ? 'bg-primary/10 font-medium text-primary' : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'}`}
              >
                {#snippet child({ props })}
                  <a
                    href={item.url}
                    {...props}
                    class="flex w-full items-center gap-3"
                  >
                    <item.icon
                      size={20}
                      class={isActive(item.url)
                        ? 'text-primary'
                        : 'text-muted-foreground/70'}
                    />
                    <span class="text-sm">{item.title}</span>
                  </a>
                {/snippet}
              </Sidebar.MenuButton>
            </Sidebar.MenuItem>
          {/each}

          <!-- Dynamic Recent Folders -->
          <Sidebar.MenuItem>
            <Sidebar.MenuButton
              onclick={toggleRecent}
              class="w-full justify-between gap-3 px-4 py-2 text-muted-foreground hover:bg-accent hover:text-accent-foreground"
            >
              <div class="flex items-center gap-3">
                <FolderIcon size={20} class="text-muted-foreground/70" />
                <span class="text-sm">Recent Folders</span>
              </div>
              <ChevronRightIcon
                size={16}
                class={`transition-transform ${isRecentOpen ? 'rotate-90' : ''}`}
              />
            </Sidebar.MenuButton>

            {#if isRecentOpen}
              <Sidebar.MenuSub>
                {#if $recentFolders.length === 0}
                  <Sidebar.MenuSubItem
                    class="px-9 py-1 text-xs italic text-muted-foreground/50"
                  >
                    No recent folders
                  </Sidebar.MenuSubItem>
                {:else}
                  {#each $recentFolders as path}
                    <Sidebar.MenuSubItem>
                      <button
                        onclick={() => handleRecentClick(path)}
                        class={`flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left text-sm transition-colors ${$currentFolder === path ? 'bg-primary/10 font-medium text-primary' : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'}`}
                        title={path}
                      >
                        <span class="truncate">{path.split('/').pop()}</span>
                      </button>
                    </Sidebar.MenuSubItem>
                  {/each}
                {/if}
              </Sidebar.MenuSub>
            {/if}
          </Sidebar.MenuItem>
        </Sidebar.Menu>
      </Sidebar.GroupContent>
    </Sidebar.Group>

    <Sidebar.Group class="mt-4">
      <Sidebar.GroupLabel
        class="px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70"
        >Culling</Sidebar.GroupLabel
      >
      <Sidebar.GroupContent>
        <Sidebar.Menu>
          {#each cullingItems as item (item.title)}
            <Sidebar.MenuItem>
              <Sidebar.MenuButton
                class={`w-full justify-start gap-3 px-4 py-2 transition-colors ${isActive(item.url) ? 'bg-primary/10 font-medium text-primary' : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'}`}
              >
                {#snippet child({ props })}
                  <a
                    href={item.url}
                    {...props}
                    class="flex w-full items-center gap-3"
                  >
                    <item.icon
                      size={20}
                      class={isActive(item.url)
                        ? 'text-primary'
                        : 'text-muted-foreground/70'}
                    />
                    <span class="text-sm">{item.title}</span>
                    {#if item.title === 'Favourites' && $favorites.size > 0}
                      <Sidebar.MenuBadge
                        class="ml-auto bg-primary font-medium text-primary-foreground"
                        >{$favorites.size}</Sidebar.MenuBadge
                      >
                    {/if}
                  </a>
                {/snippet}
              </Sidebar.MenuButton>
            </Sidebar.MenuItem>
          {/each}
        </Sidebar.Menu>
      </Sidebar.GroupContent>
    </Sidebar.Group>
  </Sidebar.Content>

  <Sidebar.Footer class="border-t p-4">
    <Sidebar.Menu>
      <Sidebar.MenuItem>
        <Sidebar.MenuButton
          class="w-full text-muted-foreground hover:text-foreground"
        >
          {#snippet child({ props })}
            <a
              href="https://github.com/santhosh-chinnasamy/lixa-gallery"
              {...props}
              target="_blank"
              class="flex w-full items-center gap-3"
            >
              <GithubIcon size={20} />
              <span class="text-sm">Github Repository</span>
            </a>
          {/snippet}
        </Sidebar.MenuButton>
      </Sidebar.MenuItem>
    </Sidebar.Menu>
  </Sidebar.Footer>
</Sidebar.Root>

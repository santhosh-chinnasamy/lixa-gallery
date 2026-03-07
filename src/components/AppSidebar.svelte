<script lang="ts">
  import { page } from '$app/state';
  import * as Sidebar from '$lib/components/ui/sidebar';
  import GithubIcon from '@lucide/svelte/icons/github';
  import HeartIcon from '@lucide/svelte/icons/heart';
  import LayoutGridIcon from '@lucide/svelte/icons/layout-grid';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import UsersIcon from '@lucide/svelte/icons/users';
  import { favorites } from '../stores/galleryStore';

  const libraryItems = [
    {
      title: 'All Photos',
      url: '/',
      icon: LayoutGridIcon,
    },
    {
      title: 'Recent Folders',
      url: '#',
      icon: FolderIcon,
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
</script>

<Sidebar.Root variant="sidebar" collapsible="none" class="border-r bg-sidebar">
  <Sidebar.Header class="h-16 border-b px-6 flex items-center">
    <span class="text-lg font-semibold tracking-tight">Lixa Gallery</span>
  </Sidebar.Header>

  <Sidebar.Content>
    <Sidebar.Group>
      <Sidebar.GroupLabel class="px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">Library</Sidebar.GroupLabel>
      <Sidebar.GroupContent>
        <Sidebar.Menu>
          {#each libraryItems as item (item.title)}
            <Sidebar.MenuItem>
              <Sidebar.MenuButton
                class={`w-full justify-start gap-3 px-4 py-2 transition-colors ${isActive(item.url) ? 'bg-primary/10 text-primary font-medium' : 'hover:bg-accent hover:text-accent-foreground text-muted-foreground'}`}
              >
                {#snippet child({ props })}
                  <a href={item.url} {...props} class="flex items-center gap-3 w-full">
                    <item.icon size={20} class={isActive(item.url) ? 'text-primary' : 'text-muted-foreground/70'} />
                    <span class="text-sm">{item.title}</span>
                  </a>
                {/snippet}
              </Sidebar.MenuButton>
            </Sidebar.MenuItem>
          {/each}
        </Sidebar.Menu>
      </Sidebar.GroupContent>
    </Sidebar.Group>

    <Sidebar.Group class="mt-4">
      <Sidebar.GroupLabel class="px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">Culling</Sidebar.GroupLabel>
      <Sidebar.GroupContent>
        <Sidebar.Menu>
          {#each cullingItems as item (item.title)}
            <Sidebar.MenuItem>
              <Sidebar.MenuButton
                class={`w-full justify-start gap-3 px-4 py-2 transition-colors ${isActive(item.url) ? 'bg-primary/10 text-primary font-medium' : 'hover:bg-accent hover:text-accent-foreground text-muted-foreground'}`}
              >
                {#snippet child({ props })}
                  <a href={item.url} {...props} class="flex items-center gap-3 w-full">
                    <item.icon size={20} class={isActive(item.url) ? 'text-primary' : 'text-muted-foreground/70'} />
                    <span class="text-sm">{item.title}</span>
                    {#if item.title === 'Favourites' && $favorites.size > 0}
                      <Sidebar.MenuBadge class="ml-auto bg-primary text-primary-foreground font-medium">{$favorites.size}</Sidebar.MenuBadge>
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
        <Sidebar.MenuButton class="w-full text-muted-foreground hover:text-foreground">
          {#snippet child({ props })}
            <a
              href="https://github.com/santhosh-chinnasamy/lixa-gallery"
              {...props}
              target="_blank"
              class="flex items-center gap-3 w-full"
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

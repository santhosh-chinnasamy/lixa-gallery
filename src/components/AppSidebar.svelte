<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import GithubIcon from '@lucide/svelte/icons/github';
  import InboxIcon from '@lucide/svelte/icons/heart';
  import HouseIcon from '@lucide/svelte/icons/house';
  import { favorites } from '../stores/galleryStore';

  const items = [
    {
      title: 'Home',
      url: '/',
      icon: HouseIcon,
    },
    {
      title: 'Favourites',
      url: '/favourites',
      icon: InboxIcon,
    },
  ];
</script>

<Sidebar.Root variant="sidebar" collapsible="icon">
  <Sidebar.Content>
    <Sidebar.Group>
      <Sidebar.GroupLabel>Lixa Gallery</Sidebar.GroupLabel>
      <Sidebar.GroupContent>
        <Sidebar.Menu>
          {#each items as item (item.title)}
            <Sidebar.MenuItem>
              <Sidebar.MenuButton isActive>
                {#snippet child({ props })}
                  <a href={item.url} {...props} title={item.title}>
                    <item.icon size={32} />
                    <span>{item.title}</span>
                    {#if item.title === 'Favourites' && $favorites.size > 0}
                      <Sidebar.MenuBadge>{$favorites.size}</Sidebar.MenuBadge>
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
  <Sidebar.Footer>
    <Sidebar.Content>
      <Sidebar.Menu>
        <Sidebar.MenuItem>
          <Sidebar.MenuButton>
            {#snippet child({ props })}
              <a
                href="https://github.com/santhosh-chinnasamy/lixa-gallery"
                {...props}
                title="github repo"
                target="_blank"
              >
                <GithubIcon />
                <span>Github</span>
              </a>
            {/snippet}
          </Sidebar.MenuButton>
        </Sidebar.MenuItem>
        <Sidebar.Trigger />
      </Sidebar.Menu>
    </Sidebar.Content>
  </Sidebar.Footer>
</Sidebar.Root>

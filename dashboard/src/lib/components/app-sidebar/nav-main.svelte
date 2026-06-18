<script lang="ts">
  import * as Collapsible from "$lib/components/ui/collapsible/index.js";
  import * as Sidebar from "$lib/components/ui/sidebar/index.js";
  import HomeIcon from "@lucide/svelte/icons/home";
	import { selectedTenant } from "$lib/state/selected-tenant.svelte";
  import ChevronRightIcon from "@lucide/svelte/icons/chevron-right";
  import { page } from "$app/state";
  import { cn } from "$lib/utils.js";

  // Highlight the item matching the current route. Detail pages (e.g.
  // /users/[userId]) keep their parent section ("Users") highlighted, so
  // sub-items match on a path prefix while the exact Home route matches strictly.
  const homeUrl = $derived(`/dashboard/${selectedTenant.tenantId}`);
  const isHomeActive = $derived(page.url.pathname === homeUrl);
  function isActive(url: string) {
    return page.url.pathname === url || page.url.pathname.startsWith(`${url}/`);
  }
  // Selected item adopts the primary color. The `!` important modifier wins over
  // the MenuButton's built-in `data-active:bg-sidebar-accent`, and the hover
  // overrides keep it from reverting to accent on pointer-over.
  const activeClasses =
    "bg-sidebar-primary! text-sidebar-primary-foreground! hover:bg-sidebar-primary! hover:text-sidebar-primary-foreground!";

  let {
    items,
  }: {
    items: {
      title: string;
      url: string;
      // this should be `Component` after @lucide/svelte updates types
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      icon?: any;
      isActive?: boolean;
      items?: {
        title: string;
        url: string;
        icon?: any;
      }[];
    }[];
  } = $props();
</script>

<Sidebar.Group>
  <Sidebar.GroupLabel>Tenant</Sidebar.GroupLabel>

  <Sidebar.MenuItem>
    <Sidebar.MenuButton tooltipContent="Tenant Details" isActive={isHomeActive}>
      {#snippet child({ props })}
        <a href={homeUrl} {...props} class={cn(props.class as string, isHomeActive && activeClasses)}>
          <HomeIcon />
          <span class="truncate">Home</span>
        </a>
      {/snippet}
    </Sidebar.MenuButton>
  </Sidebar.MenuItem>

  <Sidebar.Menu>
    {#each items as item (item.title)}
      {@const groupActive = item.isActive || (item.items ?? []).some((s) => isActive(s.url))}
      <Collapsible.Root open={groupActive} class="group/collapsible">
        {#snippet child({ props })}
          <Sidebar.MenuItem {...props}>
            <Collapsible.Trigger>
              {#snippet child({ props })}
                <Sidebar.MenuButton {...props} tooltipContent={item.title}>
                  {#if item.icon}
                    <item.icon />
                  {/if}

                  <span>{item.title}</span>

                  <ChevronRightIcon
                    class="ms-auto transition-transform duration-200 group-data-[state=open]/collapsible:rotate-90"
                  />
                </Sidebar.MenuButton>
              {/snippet}
            </Collapsible.Trigger>

            <Collapsible.Content>
              <Sidebar.MenuSub>
                {#each item.items ?? [] as subItem (subItem.title)}
                  {@const active = isActive(subItem.url)}
                  <Sidebar.MenuSubItem>
                    <Sidebar.MenuSubButton class="flex" isActive={active}>
                      {#snippet child({ props })}
                        <a
                          href={subItem.url}
                          {...props}
                          class={cn(
                            "flex items-center gap-3 rounded-md px-2 py-1",
                            active ? activeClasses : "hover:bg-accent"
                          )}
                        >
                          {#if subItem.icon}
                            <subItem.icon size={16} />
                          {/if}

                          <span>{subItem.title}</span>
                        </a>
                      {/snippet}
                    </Sidebar.MenuSubButton>
                  </Sidebar.MenuSubItem>
                {/each}
              </Sidebar.MenuSub>
            </Collapsible.Content>
          </Sidebar.MenuItem>
        {/snippet}
      </Collapsible.Root>
    {/each}
  </Sidebar.Menu>
</Sidebar.Group>
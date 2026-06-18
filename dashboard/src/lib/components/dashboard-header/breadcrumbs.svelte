<script lang="ts">
  import * as Breadcrumb from "$lib/components/ui/breadcrumb/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import { SidebarTrigger } from "$lib/components/ui/sidebar/index.js";
  import { cn } from "$lib/utils.js";

  type Item = {
    name: string;
    path?: string;
  };

  let {
    items,
    disableSideBar = false,
    maxItems = 4,
  }: {
    items: Item[];
    disableSideBar?: boolean;
    maxItems?: number;
  } = $props();
</script>

{#snippet itemLink(item: Item, isLast: boolean)}
  {#if item.path}
    <Breadcrumb.Link
      href={item.path}
      class={cn(
        isLast
          ? "text-foreground font-bold"
          : "text-muted-foreground font-normal",
      )}
    >
      {item.name}
    </Breadcrumb.Link>
  {:else}
    <span class={cn("text-foreground", isLast ? "font-bold" : "font-normal")}>
      {item.name}
    </span>
  {/if}
{/snippet}

<div class="flex items-center gap-4">
  {#if !disableSideBar}
    <SidebarTrigger />
  {/if}

  <Breadcrumb.Root>
    <Breadcrumb.List>
      {#if items.length <= maxItems}
        <!-- Scenario 1: item count within the limit -->
        {#each items as item, index (index)}
          {@const isLast = index === items.length - 1}
          <Breadcrumb.Item>
            {@render itemLink(item, isLast)}
          </Breadcrumb.Item>
          {#if !isLast}
            <Breadcrumb.Separator />
          {/if}
        {/each}
      {:else}
        <!-- Scenario 2: collapse the middle items into a dropdown -->
        <Breadcrumb.Item>
          {@render itemLink(items[0], false)}
        </Breadcrumb.Item>
        <Breadcrumb.Separator />

        <Breadcrumb.Item>
          <DropdownMenu.Root>
            <DropdownMenu.Trigger
              class="hover:text-foreground flex items-center gap-1"
            >
              <Breadcrumb.Ellipsis class="size-4" />
              <span class="sr-only">Toggle menu</span>
            </DropdownMenu.Trigger>

            <DropdownMenu.Content align="start">
              {#each items.slice(1, -2) as item, index (index)}
                <DropdownMenu.Item>
                  {#snippet child({ props })}
                    {#if item.path}
                      <a href={item.path} {...props}>
                        {index + 1}. {item.name}
                      </a>
                    {:else}
                      <span {...props}>
                        {index + 1}. {item.name}
                      </span>
                    {/if}
                  {/snippet}
                </DropdownMenu.Item>
              {/each}
            </DropdownMenu.Content>
          </DropdownMenu.Root>
        </Breadcrumb.Item>
        <Breadcrumb.Separator />

        <!-- Last 2 items -->
        {#each items.slice(-2) as item, index (index)}
          {@const isLast = index === 1}
          <Breadcrumb.Item>
            {@render itemLink(item, isLast)}
          </Breadcrumb.Item>
          {#if !isLast}
            <Breadcrumb.Separator />
          {/if}
        {/each}
      {/if}
    </Breadcrumb.List>
  </Breadcrumb.Root>
</div>

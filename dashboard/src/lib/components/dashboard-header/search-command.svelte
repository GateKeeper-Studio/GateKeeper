<script lang="ts">
  import { goto } from "$app/navigation";
  import LayoutPanelLeftIcon from "@lucide/svelte/icons/layout-panel-left";
  import SearchIcon from "@lucide/svelte/icons/search";

  import { Button } from "$lib/components/ui/button/index.js";
  import * as Command from "$lib/components/ui/command/index.js";

  let open = $state(false);

  const sidebarItems = [
    {
      name: "Applications",
      icon: LayoutPanelLeftIcon,
      path: "/dashboard/applications",
    },
  ];

  function navigate(path: string) {
    goto(path);
    open = false;
  }
</script>

<div class="ml-auto flex flex-col gap-4">
  <Button onclick={() => (open = true)} variant="outline" class="w-fit">
    <SearchIcon class="mr-2 size-4" />
    <span class="text-muted-foreground font-medium">
      Search for something...
    </span>
  </Button>

  <Command.Dialog bind:open>
    <Command.Input placeholder="Type a command or search..." />
    <Command.List>
      <Command.Empty>No results found.</Command.Empty>
      <Command.Group heading="Navigation">
        {#each sidebarItems as item (item.name)}
          <Command.Item onSelect={() => navigate(item.path)}>
            <item.icon />
            <span>{item.name}</span>
          </Command.Item>
        {/each}
      </Command.Group>
    </Command.List>
  </Command.Dialog>
</div>

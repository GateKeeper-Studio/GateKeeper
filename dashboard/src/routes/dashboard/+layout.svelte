<script lang="ts">
	import { Toaster } from "svelte-sonner";
  import * as Sidebar from "$lib/components/ui/sidebar/index.js";
  import AppSidebar from "$lib/components/app-sidebar/index.svelte";

  // The selected tenant now lives in the shared rune store
  // ($lib/state/selected-tenant.svelte). The sidebar switcher writes to it and
  // any dashboard component reads `selectedTenant.current` reactively.
  let { children } = $props();
</script>

<div class="flex h-screen w-full">
  <Sidebar.Provider>
    <AppSidebar/>

    <div class="flex-1 min-w-0">
      <Toaster richColors />

      {@render children()}
    </div>
  </Sidebar.Provider>
</div>
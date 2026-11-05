<script lang="ts">
	import { Toaster } from "svelte-sonner";
  import { createQuery } from "@tanstack/svelte-query";
  import { page } from "$app/state";
  import * as Sidebar from "$lib/components/ui/sidebar/index.js";
  import AppSidebar from "$lib/components/app-sidebar/index.svelte";
  import SettingsDialog from "$lib/components/settings-dialog/index.svelte";
  import SplashScreen from "$lib/components/splash-screen.svelte";
  import { onMount } from "svelte";
  import { listTenants } from "$lib/services/admin/tenants";
  import { selectedTenant } from "$lib/state/selected-tenant.svelte";
  import { appTheme } from "$lib/state/app-theme.svelte";
  import { bootProgress } from "$lib/state/boot-progress.svelte";

  // The selected tenant now lives in the shared rune store
  // ($lib/state/selected-tenant.svelte). This layout owns the tenant-list fetch and
  // any dashboard component reads `selectedTenant.current` reactively.
  let { children } = $props();

  // Fetch the tenant list HERE, at the layout, not in the sidebar. The sidebar's
  // tenant switcher only mounts on desktop — on mobile it lives inside an off-canvas
  // Sheet that isn't rendered until opened. Owning the query at the always-mounted
  // layout guarantees the active org (and its saved theme) resolves on every viewport;
  // otherwise `selectedTenant.loading` would stay true on mobile and the DB theme would
  // never apply. The switcher just reads the shared store.
  const tenantsQuery = createQuery(() => ({
    queryKey: ["tenants"],
    queryFn: async () => {
      const [data, err] = await listTenants();
      if (err) throw err;
      return data.items;
    }
  }));

  $effect(() => {
    selectedTenant.tenants = tenantsQuery.data ?? [];
    selectedTenant.loading = tenantsQuery.isPending;
    selectedTenant.error = tenantsQuery.isError ? tenantsQuery.error : null;
    // The tenant list is the network gate that resolves the org theme; mark it done
    // (success or error) so the splash can progress past it.
    if (!tenantsQuery.isPending) bootProgress.mark("tenants");
  });

  onMount(() => {
    bootProgress.mark("app");
    // The +layout.ts guard already redirected here only with a valid token.
    bootProgress.mark("session");
  });

  // Remember whichever tenant the URL points at so tenant-scoped navigation keeps
  // working on global pages such as Organization and after a reload.
  $effect(() => {
    if (page.params.tenantId) selectedTenant.setActive(page.params.tenantId);
  });

  // Apply the active organization's saved theme (main color + component styles) from
  // the database once the tenant list has loaded — the splash covers this window, so
  // the app is revealed already themed (no flash). The DB is the single source of
  // truth; nothing about the theme is persisted in the browser. Re-runs on org switch.
  $effect(() => {
    if (selectedTenant.loading) return;
    const appearance = selectedTenant.current?.appearance ?? null;
    appTheme.apply(appearance);
    if (appearance) appTheme.applyDefaultMode(appearance.defaultMode);
  });
</script>

<SplashScreen />

<!-- `app-reveal` plays once when the splash hands off: the shell scales + fades in
     behind the dissolving overlay for a smooth, premium reveal. -->
<div class="dash-shell flex h-screen w-full" class:app-reveal={bootProgress.revealed}>
  <Sidebar.Provider>
    <AppSidebar/>

    <div class="flex-1 min-w-0">
      <Toaster richColors />
      <SettingsDialog />

      {@render children()}
    </div>
  </Sidebar.Provider>
</div>

<style>
  .app-reveal {
    animation: app-reveal 0.62s cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes app-reveal {
    0% {
      opacity: 0.5;
      transform: scale(0.986) translateY(6px);
    }
    100% {
      opacity: 1;
      transform: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .app-reveal {
      animation: none;
    }
  }
</style>

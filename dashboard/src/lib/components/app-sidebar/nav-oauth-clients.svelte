<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';

	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { listOAuthClients } from '$lib/services/admin/oauth-clients';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { page } from '$app/state';
	import { cn } from '$lib/utils.js';
	import { m } from '$lib/paraglide/messages.js';

	// The currently-viewed client adopts the primary color so the selection is obvious.
	const activeClasses =
		'bg-sidebar-primary! text-sidebar-primary-foreground! hover:bg-sidebar-primary! hover:text-sidebar-primary-foreground!';
	function clientHref(id: string) {
		return `/tenants/${selectedTenant.tenantId}/oauth-clients/${id}`;
	}
	function isActive(id: string) {
		const url = clientHref(id);
		return page.url.pathname === url || page.url.pathname.startsWith(`${url}/`);
	}

	// Re-fetches whenever the URL-selected tenant changes (tenantId is in the key)
	// and stays idle until a tenant is selected.
	const clientsQuery = createQuery(() => ({
		queryKey: ['oauth-clients', selectedTenant.tenantId],
		enabled: !!selectedTenant.tenantId,
		queryFn: async () => {
			const [data, err] = await listOAuthClients(selectedTenant.tenantId!);
			if (err) throw err;
			return data.items;
		}
	}));
</script>

{#if selectedTenant.tenantId}
<Sidebar.Group class="group-data-[collapsible=icon]:hidden">
	<Sidebar.GroupLabel>{m.nav_oauth_clients()}</Sidebar.GroupLabel>
	<Sidebar.Menu>
		{#if clientsQuery.isPending}
			<div class="text-sidebar-foreground/70 px-2 py-1.5 text-xs">{m.tenant_switcher_loading()}</div>
		{:else if clientsQuery.isError}
			<div class="text-destructive px-2 py-1.5 text-xs">{clientsQuery.error.message}</div>
		{:else if clientsQuery.data.length === 0}
			<div class="text-sidebar-foreground/70 px-2 py-1.5 text-xs">{m.oauth_clients_sidebar_empty()}</div>
		{:else}
			{#each clientsQuery.data as client (client.id)}
					{@const active = isActive(client.id)}
				<Sidebar.MenuItem>
					<Sidebar.MenuButton tooltipContent={client.name} isActive={active}>
						{#snippet child({ props })}
							<a href={clientHref(client.id)} {...props} class={cn(props.class as string, active && activeClasses)}>
								<KeyRoundIcon />
								<span class="truncate">{client.name}</span>
							</a>
						{/snippet}
					</Sidebar.MenuButton>
				</Sidebar.MenuItem>
			{/each}
		{/if}
	</Sidebar.Menu>
</Sidebar.Group>
{/if}

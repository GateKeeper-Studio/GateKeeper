<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import Building2Icon from '@lucide/svelte/icons/building-2';

	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { useSidebar } from '$lib/components/ui/sidebar/index.js';
	import { listTenants } from '$lib/services/admin/tenants';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';

	const sidebar = useSidebar();

	const tenantsQuery = createQuery(() => ({
		queryKey: ['tenants'],
		queryFn: async () => {
			const [data, err] = await listTenants();
			if (err) throw err;
			return data.items;
		}
	}));

	// Hand the fetched list to the shared store so it can resolve the URL's
	// `[tenantId]` param into a full tenant for every dashboard component.
	$effect(() => {
		selectedTenant.tenants = tenantsQuery.data ?? [];
	});

	// Selecting a tenant just changes the URL — the store derives `current` from it.
	// Swap only the `[tenantId]` segment so the current sub-section (e.g.
	// /email-providers) and any query string carry over to the new tenant.
	function select(tenantId: string) {
		const current = page.params.tenantId;
		const path = current
			? page.url.pathname.replace(`/dashboard/${current}`, `/dashboard/${tenantId}`)
			: `/dashboard/${tenantId}`;
		goto(`${path}${page.url.search}`);
	}
</script>

<Sidebar.Menu>
	<Sidebar.MenuItem>
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Sidebar.MenuButton
						{...props}
						size="lg"
						class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
					>
						<div
							class="bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-8 items-center justify-center rounded-lg"
						>
							<Building2Icon class="size-4" />
						</div>

						<div class="grid flex-1 text-start text-sm leading-tight">
							<span class="truncate font-medium">
								{#if selectedTenant.current}
									{selectedTenant.current.name}
								{:else if tenantsQuery.isPending}
									Loading tenants…
								{:else if tenantsQuery.isError}
									Failed to load
								{:else if selectedTenant.tenantId}
									Unknown tenant
								{:else}
									Select a tenant
								{/if}
							</span>

							<span class="truncate text-xs text-muted-foreground">
								{selectedTenant.current ? 'Tenant' : 'No tenant selected'}
							</span>
						</div>

						<ChevronsUpDownIcon class="ms-auto" />
					</Sidebar.MenuButton>
				{/snippet}
			</DropdownMenu.Trigger>

			<DropdownMenu.Content
				class="w-(--bits-dropdown-menu-anchor-width) min-w-56 rounded-lg"
				align="start"
				side={sidebar.isMobile ? 'bottom' : 'right'}
				sideOffset={4}
			>
				<DropdownMenu.Label class="text-muted-foreground text-xs">Tenants</DropdownMenu.Label>

				{#if tenantsQuery.isPending}
					<div class="text-muted-foreground px-2 py-1.5 text-sm">Loading…</div>
				{:else if tenantsQuery.isError}
					<div class="text-destructive px-2 py-1.5 text-sm">
						{tenantsQuery.error.message}
					</div>
				{:else if tenantsQuery.data.length === 0}
					<div class="text-muted-foreground px-2 py-1.5 text-sm">No tenants yet</div>
				{:else}
					{#each tenantsQuery.data as tenant, index (tenant.tenantId)}
						<DropdownMenu.Item
							onSelect={() => select(tenant.tenantId)}
							class="gap-2 p-2"
							data-active={tenant.tenantId === selectedTenant.tenantId}
						>
							<div class="flex size-6 items-center justify-center rounded-md border">
								<Building2Icon class="size-3.5 shrink-0" />
							</div>
							{tenant.name}
							{#if index < 9}
								<DropdownMenu.Shortcut>⌘{index + 1}</DropdownMenu.Shortcut>
							{/if}
						</DropdownMenu.Item>
					{/each}
				{/if}

				<DropdownMenu.Separator />

				<DropdownMenu.Item class="gap-2 p-2">
					{#snippet child({ props })}
						<a href="/dashboard/create-tenant" {...props}>
							<div
								class="flex size-6 items-center justify-center rounded-md border bg-transparent"
							>
								<PlusIcon class="size-4" />
							</div>
							<div class="text-muted-foreground font-medium">Add tenant</div>
						</a>
					{/snippet}
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</Sidebar.MenuItem>
</Sidebar.Menu>

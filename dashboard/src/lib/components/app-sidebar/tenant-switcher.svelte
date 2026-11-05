<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import Building2Icon from '@lucide/svelte/icons/building-2';

	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { useSidebar } from '$lib/components/ui/sidebar/index.js';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const sidebar = useSidebar();

	// The tenant list is fetched by the dashboard +layout (always mounted, so it works
	// on mobile where this switcher lives in an off-canvas Sheet). Read the shared store.

	// Selecting a tenant changes the URL; the store derives `current` from it. Remember
	// the choice immediately so it survives navigating to a global page or a reload.
	// Swap only the `[tenantId]` segment so the current sub-section (e.g.
	// /email-providers) and any query string carry over to the new tenant.
	function select(tenantId: string) {
		selectedTenant.setActive(tenantId);
		const current = page.params.tenantId;
		const path = current
			? page.url.pathname.replace(`/tenants/${current}`, `/tenants/${tenantId}`)
			: `/tenants/${tenantId}`;
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
								{:else if selectedTenant.loading}
									{m.tenant_switcher_loading_tenants()}
								{:else if selectedTenant.error}
									{m.tenant_switcher_failed_to_load()}
								{:else if selectedTenant.tenantId}
									{m.tenant_switcher_unknown_tenant()}
								{:else}
									{m.tenant_switcher_select_tenant()}
								{/if}
							</span>

							<span class="truncate text-xs text-muted-foreground">
								{selectedTenant.current ? m.sidebar_tenant() : m.tenant_switcher_no_tenant_selected()}
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
				<DropdownMenu.Label class="text-muted-foreground text-xs">{m.tenant_switcher_tenants()}</DropdownMenu.Label>

				{#if selectedTenant.loading}
					<div class="text-muted-foreground px-2 py-1.5 text-sm">{m.tenant_switcher_loading()}</div>
				{:else if selectedTenant.error}
					<div class="text-destructive px-2 py-1.5 text-sm">
						{selectedTenant.error.message}
					</div>
				{:else if selectedTenant.tenants.length === 0}
					<div class="text-muted-foreground px-2 py-1.5 text-sm">{m.dashboard_home_empty_title()}</div>
				{:else}
					{#each selectedTenant.tenants as tenant, index (tenant.tenantId)}
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
						<a href="/tenants/new" {...props}>
							<div
								class="flex size-6 items-center justify-center rounded-md border bg-transparent"
							>
								<PlusIcon class="size-4" />
							</div>
							<div class="text-muted-foreground font-medium">{m.tenant_switcher_add_tenant()}</div>
						</a>
					{/snippet}
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</Sidebar.MenuItem>
</Sidebar.Menu>

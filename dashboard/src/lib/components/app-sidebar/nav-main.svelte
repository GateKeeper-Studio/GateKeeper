<script lang="ts">
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import HomeIcon from '@lucide/svelte/icons/home';
	import Building2Icon from '@lucide/svelte/icons/building-2';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import { page } from '$app/state';
	import { cn } from '$lib/utils.js';
	import { m } from '$lib/paraglide/messages.js';

	// Highlight the item matching the current route. Detail pages (e.g.
	// /users/[userId]) keep their parent section ("Users") highlighted, so
	// sub-items match on a path prefix while the exact Home route matches strictly.
	// Some sub-item URLs are filesystem-parents of sibling URLs (e.g. Security
	// "Overview" is /security, while "Login Events" is /security/login-events),
	// so a naive prefix match would light up both. Pick the longest matching URL
	// among all sub-items so the most specific one wins.
	const homeUrl = $derived(`/tenants/${selectedTenant.tenantId}`);
	const isHomeActive = $derived(page.url.pathname === homeUrl);

	let {
		items
	}: {
		items: {
			title: string;
			url: string;
			// this should be `Component` after @lucide/svelte updates types
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			icon?: any;
			isActive?: boolean;
			// Tenant-scoped groups are hidden until a tenant is active (their links would
			// otherwise point at /tenants/undefined/...). Org-level groups stay visible.
			tenantScoped?: boolean;
			items?: {
				title: string;
				url: string;
				icon?: any;
			}[];
		}[];
	} = $props();

	const activeSubUrl = $derived.by(() => {
		const candidates = items
			.flatMap((item) => item.items ?? [])
			.filter(
				(sub) => page.url.pathname === sub.url || page.url.pathname.startsWith(`${sub.url}/`)
			);
		if (candidates.length === 0) return null;
		return candidates.reduce((longest, c) => (c.url.length > longest.url.length ? c : longest))
			.url;
	});
	function isActive(url: string) {
		return url === activeSubUrl;
	}
	// Selected item adopts the primary color. The `!` important modifier wins over
	// the MenuButton's built-in `data-active:bg-sidebar-accent`, and the hover
	// overrides keep it from reverting to accent on pointer-over.
	const activeClasses =
		'bg-sidebar-primary! text-sidebar-primary-foreground! hover:bg-sidebar-primary! hover:text-sidebar-primary-foreground!';
</script>

<Sidebar.Group>
	<Sidebar.GroupLabel>{m.sidebar_tenant()}</Sidebar.GroupLabel>

	{#if selectedTenant.tenantId}
		<Sidebar.MenuItem>
			<Sidebar.MenuButton tooltipContent={m.sidebar_tenant_details()} isActive={isHomeActive}>
				{#snippet child({ props })}
					<a
						href={homeUrl}
						{...props}
						class={cn(props.class as string, isHomeActive && activeClasses)}
					>
						<HomeIcon />
						<span class="truncate">{m.sidebar_home()}</span>
					</a>
				{/snippet}
			</Sidebar.MenuButton>
		</Sidebar.MenuItem>
	{/if}

	<Sidebar.Menu>
		{#each items as item (item.title)}
			{#if !item.tenantScoped || selectedTenant.tenantId}
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
															'flex items-center gap-3 rounded-md px-2 py-1',
															active ? activeClasses : 'hover:bg-accent'
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
			{/if}
		{/each}
	</Sidebar.Menu>

	{#if !selectedTenant.tenantId}
		<!-- No tenant on the account yet: guide the admin to create one instead of
         rendering /tenants/undefined/... links. -->
		<div class="px-2 py-3 text-center group-data-[collapsible=icon]:hidden">
			<div
				class="mx-auto mb-2 flex size-9 items-center justify-center rounded-lg border bg-sidebar-accent text-sidebar-accent-foreground"
			>
				<Building2Icon class="size-4" />
			</div>
			<p class="text-sm font-medium text-sidebar-foreground">{m.sidebar_no_tenant_title()}</p>
			<p class="mt-0.5 text-xs text-sidebar-foreground/70">
				{m.sidebar_no_tenant_desc()}
			</p>
			<Button href="/tenants/new" size="sm" class="mt-3 w-full">
				<PlusIcon class="size-4" />
				{m.sidebar_create_tenant()}
			</Button>
		</div>

		<!-- Icon-collapsed rail: a single create affordance with a tooltip. -->
		<Sidebar.Menu class="hidden group-data-[collapsible=icon]:flex">
			<Sidebar.MenuItem>
				<Sidebar.MenuButton tooltipContent={m.sidebar_create_tenant()}>
					{#snippet child({ props })}
						<a href="/tenants/new" {...props}>
							<PlusIcon />
							<span>{m.sidebar_create_tenant()}</span>
						</a>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	{/if}
</Sidebar.Group>

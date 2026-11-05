<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { createQuery } from '@tanstack/svelte-query';

	import InfoIcon from '@lucide/svelte/icons/info';
	import UsersIcon from '@lucide/svelte/icons/users';

	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { getTenantRole } from '$lib/services/admin/roles';
	import RolePageHeader from './role-page-header.svelte';
	import OverviewTab from './overview-tab.svelte';
	import UsersTab from './users-tab.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(page.params.tenantId!);
	const roleId = $derived(page.params.roleId!);
	const listHref = $derived(`/tenants/${tenantId}/roles`);

	const roleQuery = createQuery(() => ({
		queryKey: ['tenant-roles', tenantId, roleId],
		enabled: !!tenantId && !!roleId,
		queryFn: async () => {
			const [data, err] = await getTenantRole(tenantId, roleId);
			if (err) throw err;
			return data;
		}
	}));

	const role = $derived(roleQuery.data);

	const TABS = ['overview', 'users'] as const;
	type Tab = (typeof TABS)[number];

	const currentTab = $derived<Tab>(
		TABS.includes(page.url.searchParams.get('tab') as Tab)
			? (page.url.searchParams.get('tab') as Tab)
			: 'overview'
	);

	function setTab(tab: Tab) {
		const url = new URL(page.url);
		url.searchParams.set('tab', tab);
		goto(url, { replaceState: true, noScroll: true, keepFocus: true });
	}

	const TAB_LABELS: Record<Tab, string> = {
		overview: m.nav_overview(),
		users: m.nav_users()
	};

	const TAB_ICONS: Record<Tab, typeof InfoIcon> = {
		overview: InfoIcon,
		users: UsersIcon
	};
</script>

<svelte:head>
	<title>{role ? m.roles_detail_page_title_with_name({ name: role.name }) : m.roles_page_title()} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.roles_tenant_fallback(), path: `/tenants/${tenantId}` },
			{ name: m.roles_page_title(), path: listHref },
			{ name: role?.name ?? '…', path: `${listHref}/${roleId}` }
		]
	}}
/>

<MainContent goBack={{ message: m.roles_back_to_roles(), href: listHref }}>
	{#if roleQuery.isPending}
		<div class="text-muted-foreground flex items-center gap-2">
			<LoadingSpinner /> {m.roles_loading_role()}
		</div>
	{:else if roleQuery.isError}
		<ErrorAlert title={m.roles_load_role_error_title()} message={roleQuery.error.message} />
	{:else if role}
		<RolePageHeader {role} {tenantId} {roleId} {listHref} />

		<Tabs.Root class="mt-4" value={currentTab} onValueChange={(tab) => setTab(tab as Tab)}>
			<Tabs.List>
				{#each TABS as tab (tab)}
					{@const Icon = TAB_ICONS[tab]}
					<Tabs.Trigger value={tab}>
						<Icon class="size-4" />
						{TAB_LABELS[tab]}
					</Tabs.Trigger>
				{/each}
			</Tabs.List>

			<Tabs.Content value="overview">
				<OverviewTab {role} />
			</Tabs.Content>

			<Tabs.Content value="users">
				<UsersTab {tenantId} {roleId} />
			</Tabs.Content>
		</Tabs.Root>
	{/if}
</MainContent>

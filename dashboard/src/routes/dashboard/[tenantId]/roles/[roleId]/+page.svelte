<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { createQuery } from '@tanstack/svelte-query';

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

	const tenantId = $derived(page.params.tenantId!);
	const roleId = $derived(page.params.roleId!);
	const listHref = $derived(`/dashboard/${tenantId}/roles`);

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
		overview: 'Overview',
		users: 'Users'
	};
</script>

<svelte:head>
	<title>{role ? `${role.name} - Role` : 'Role'} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Roles', path: listHref },
			{ name: role?.name ?? '…', path: `${listHref}/${roleId}` }
		]
	}}
/>

<MainContent goBack={{ message: 'Back to roles', href: listHref }}>
	{#if roleQuery.isPending}
		<div class="text-muted-foreground flex items-center gap-2">
			<LoadingSpinner /> Loading role…
		</div>
	{:else if roleQuery.isError}
		<ErrorAlert title="Couldn't load this role" message={roleQuery.error.message} />
	{:else if role}
		<RolePageHeader {role} {tenantId} {roleId} {listHref} />

		<Tabs.Root class="mt-4" value={currentTab} onValueChange={(tab) => setTab(tab as Tab)}>
			<Tabs.List>
				{#each TABS as tab (tab)}
					<Tabs.Trigger value={tab}>{TAB_LABELS[tab]}</Tabs.Trigger>
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

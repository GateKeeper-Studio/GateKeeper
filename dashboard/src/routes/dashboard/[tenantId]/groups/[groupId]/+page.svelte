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
	import { getTenantGroup } from '$lib/services/admin/groups';
	import GroupPageHeader from './group-page-header.svelte';
	import OverviewTab from './overview-tab.svelte';
	import MembersTab from './members-tab.svelte';

	const tenantId = $derived(page.params.tenantId!);
	const groupId = $derived(page.params.groupId!);
	const listHref = $derived(`/dashboard/${tenantId}/groups`);

	const groupQuery = createQuery(() => ({
		queryKey: ['tenant-groups', tenantId, groupId],
		enabled: !!tenantId && !!groupId,
		queryFn: async () => {
			const [data, err] = await getTenantGroup(tenantId, groupId);
			if (err) throw err;
			return data;
		}
	}));

	const group = $derived(groupQuery.data);

	const TABS = ['overview', 'members'] as const;
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
		members: 'Members'
	};
</script>

<svelte:head>
	<title>{group ? `${group.name} - Group` : 'Group'} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Groups', path: listHref },
			{ name: group?.name ?? '…', path: `${listHref}/${groupId}` }
		]
	}}
/>

<MainContent goBack={{ message: 'Back to groups', href: listHref }}>
	{#if groupQuery.isPending}
		<div class="text-muted-foreground flex items-center gap-2">
			<LoadingSpinner /> Loading group…
		</div>
	{:else if groupQuery.isError}
		<ErrorAlert title="Couldn't load this group" message={groupQuery.error.message} />
	{:else if group}
		<GroupPageHeader {group} {tenantId} {groupId} {listHref} />

		<Tabs.Root class="mt-4" value={currentTab} onValueChange={(tab) => setTab(tab as Tab)}>
			<Tabs.List>
				{#each TABS as tab (tab)}
					<Tabs.Trigger value={tab}>{TAB_LABELS[tab]}</Tabs.Trigger>
				{/each}
			</Tabs.List>

			<Tabs.Content value="overview">
				<OverviewTab {group} />
			</Tabs.Content>

			<Tabs.Content value="members">
				<MembersTab {tenantId} {groupId} />
			</Tabs.Content>
		</Tabs.Root>
	{/if}
</MainContent>

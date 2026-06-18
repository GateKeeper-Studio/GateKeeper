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
	import { getTenantUser } from '$lib/services/admin/users';
	import { userFullName } from '../display.js';
	import UserPageHeader from './user-page-header.svelte';
	import OverviewTab from './overview-tab.svelte';
	import AccessTab from './access-tab.svelte';
	import PrivacyTab from './privacy-tab.svelte';

	const tenantId = $derived(page.params.tenantId!);
	const userId = $derived(page.params.userId!);
	const listHref = $derived(`/dashboard/${tenantId}/users`);

	const userQuery = createQuery(() => ({
		queryKey: ['tenant-users', tenantId, userId],
		enabled: !!tenantId && !!userId,
		queryFn: async () => {
			const [data, err] = await getTenantUser(tenantId, userId);
			if (err) throw err;
			return data;
		}
	}));

	const user = $derived(userQuery.data);

	// Tab state lives in the URL (?tab=) so it survives refresh and back/forward.
	const TABS = ['overview', 'access', 'privacy'] as const;
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
		access: 'Roles & groups',
		privacy: 'Privacy'
	};
</script>

<svelte:head>
	<title>{user ? `${userFullName(user)} - User` : 'User'} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Users', path: listHref },
			{ name: user ? userFullName(user) : '…', path: `${listHref}/${userId}` }
		]
	}}
/>

<MainContent goBack={{ message: 'Back to users', href: listHref }}>
	{#if userQuery.isPending}
		<div class="text-muted-foreground flex items-center gap-2">
			<LoadingSpinner /> Loading user…
		</div>
	{:else if userQuery.isError}
		<ErrorAlert title="Couldn't load this user" message={userQuery.error.message} />
	{:else if user}
		<UserPageHeader {user} {tenantId} {userId} {listHref} />

		<Tabs.Root class="mt-4" value={currentTab} onValueChange={(tab) => setTab(tab as Tab)}>
			<Tabs.List>
				{#each TABS as tab (tab)}
					<Tabs.Trigger value={tab}>{TAB_LABELS[tab]}</Tabs.Trigger>
				{/each}
			</Tabs.List>

			<Tabs.Content value="overview">
				<OverviewTab {user} />
			</Tabs.Content>

			<Tabs.Content value="access">
				<AccessTab {tenantId} {userId} />
			</Tabs.Content>

			<Tabs.Content value="privacy">
				<PrivacyTab {tenantId} {userId} />
			</Tabs.Content>
		</Tabs.Root>
	{/if}
</MainContent>

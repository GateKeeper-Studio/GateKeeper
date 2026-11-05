<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { SvelteURLSearchParams } from 'svelte/reactivity';
	import { createQuery } from '@tanstack/svelte-query';

	import InfoIcon from '@lucide/svelte/icons/info';
	import ShieldIcon from '@lucide/svelte/icons/shield';
	import LockIcon from '@lucide/svelte/icons/lock';
	import MapPinIcon from '@lucide/svelte/icons/map-pin';
	import ListChecksIcon from '@lucide/svelte/icons/list-checks';

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
	import SecurityTab from './security-tab.svelte';
	import CustomFieldsTab from './custom-fields-tab.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(page.params.tenantId!);
	const userId = $derived(page.params.userId!);
	const listHref = $derived(`/tenants/${tenantId}/users`);

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
	const TABS = ['overview', 'access', 'privacy', 'security', 'custom-fields'] as const;
	type Tab = (typeof TABS)[number];

	const currentTab = $derived<Tab>(
		TABS.includes(page.url.searchParams.get('tab') as Tab)
			? (page.url.searchParams.get('tab') as Tab)
			: 'overview'
	);

	function setTab(tab: Tab) {
		const params = new SvelteURLSearchParams(page.url.searchParams);
		params.set('tab', tab);
		goto(
			resolve(`/(dashboard)/tenants/[tenantId]/users/[userId]?${params.toString()}`, {
				tenantId,
				userId
			}),
			{ replaceState: true, noScroll: true, keepFocus: true }
		);
	}

	const TAB_LABELS: Record<Tab, string> = {
		overview: m.users_tab_overview(),
		access: m.users_tab_access(),
		privacy: m.users_tab_privacy(),
		security: m.users_tab_security(),
		'custom-fields': m.users_tab_custom_fields()
	};

	const TAB_ICONS: Record<Tab, typeof InfoIcon> = {
		overview: InfoIcon,
		access: ShieldIcon,
		privacy: LockIcon,
		security: MapPinIcon,
		'custom-fields': ListChecksIcon
	};
</script>

<svelte:head>
	<title>{user ? `${userFullName(user)} - User` : 'User'} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{
				name: selectedTenant.current?.name ?? m.users_tenant_fallback(),
				path: `/tenants/${tenantId}`
			},
			{ name: m.users_page_title(), path: listHref },
			{ name: user ? userFullName(user) : '…', path: `${listHref}/${userId}` }
		]
	}}
/>

<MainContent goBack={{ message: m.users_back_to_list(), href: listHref }}>
	{#if userQuery.isPending}
		<div class="flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner />
			{m.users_loading_user()}
		</div>
	{:else if userQuery.isError}
		<ErrorAlert title={m.users_load_user_error_title()} message={userQuery.error.message} />
	{:else if user}
		<UserPageHeader {user} {tenantId} {userId} {listHref} />

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
				<OverviewTab {user} />
			</Tabs.Content>

			<Tabs.Content value="access">
				<AccessTab {tenantId} {userId} />
			</Tabs.Content>

			<Tabs.Content value="privacy">
				<PrivacyTab {tenantId} {userId} />
			</Tabs.Content>

			<Tabs.Content value="security">
				<SecurityTab {tenantId} {userId} />
			</Tabs.Content>

			<Tabs.Content value="custom-fields">
				<CustomFieldsTab {tenantId} {userId} customFieldValues={user.custom_field_values ?? null} />
			</Tabs.Content>
		</Tabs.Root>
	{/if}
</MainContent>

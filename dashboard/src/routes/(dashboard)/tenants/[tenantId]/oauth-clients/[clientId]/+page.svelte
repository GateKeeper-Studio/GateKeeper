<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { createQuery } from '@tanstack/svelte-query';

	import InfoIcon from '@lucide/svelte/icons/info';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import LogInIcon from '@lucide/svelte/icons/log-in';
	import PaletteIcon from '@lucide/svelte/icons/palette';

	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { getOAuthClient } from '$lib/services/admin/oauth-clients';
	import ClientPageHeader from './client-page-header.svelte';
	import OverviewTab from './overview-tab.svelte';
	import CredentialsTab from './credentials-tab.svelte';
	import ProvidersTab from './providers-tab.svelte';
	import AppearanceTab from './appearance-tab.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(page.params.tenantId!);
	const clientId = $derived(page.params.clientId!);
	const listHref = $derived(`/tenants/${tenantId}/oauth-clients`);

	const clientQuery = createQuery(() => ({
		queryKey: ['oauth-clients', tenantId, clientId],
		enabled: !!tenantId && !!clientId,
		queryFn: async () => {
			const [data, err] = await getOAuthClient(tenantId, clientId);
			if (err) throw err;
			return data;
		}
	}));

	const client = $derived(clientQuery.data);

	// Tab state lives in the URL (?tab=) so it survives refresh and back/forward.
	const TABS = ['overview', 'credentials', 'providers', 'appearance'] as const;
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
		credentials: 'Credentials',
		providers: 'Sign-in providers',
		appearance: 'Login page'
	};

	const TAB_ICONS: Record<Tab, typeof InfoIcon> = {
		overview: InfoIcon,
		credentials: KeyRoundIcon,
		providers: LogInIcon,
		appearance: PaletteIcon
	};
</script>

<svelte:head>
	<title>{client ? `${client.name} - OAuth Client` : 'OAuth Client'} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/tenants/${tenantId}` },
			{ name: 'OAuth Clients', path: listHref },
			{ name: client?.name ?? '…', path: `${listHref}/${clientId}` }
		]
	}}
/>

<MainContent goBack={{ message: m.oauth_clients_back_to_clients(), href: listHref }}>
	{#if clientQuery.isPending}
		<div class="flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner /> {m.oauth_clients_loading_client()}
		</div>
	{:else if clientQuery.isError}
		<ErrorAlert title={m.oauth_clients_load_error_title()} message={clientQuery.error.message} />
	{:else if client}
		<ClientPageHeader {client} {tenantId} {clientId} {listHref} />

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
				<OverviewTab {client} {tenantId} />
			</Tabs.Content>

			<Tabs.Content value="credentials">
				<CredentialsTab {client} {tenantId} {clientId} />
			</Tabs.Content>

			<Tabs.Content value="providers">
				<ProvidersTab {tenantId} {clientId} externalLoginOnly={client.external_login_only} />
			</Tabs.Content>

			<Tabs.Content value="appearance">
				<AppearanceTab {client} {tenantId} {clientId} />
			</Tabs.Content>
		</Tabs.Root>
	{/if}
</MainContent>

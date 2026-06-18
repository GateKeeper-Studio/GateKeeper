<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { listOAuthClients } from '$lib/services/admin/oauth-clients';
	import { clientBadges } from './display.js';

	const tenantId = $derived(selectedTenant.tenantId!);

	// Client-side Svelte Query, keyed by tenant so it refetches on tenant switch.
	const clientsQuery = createQuery(() => ({
		queryKey: ['oauth-clients', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listOAuthClients(tenantId);
			if (err) throw err;
			return data.items;
		}
	}));
</script>

<svelte:head><title>OAuth Clients - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'OAuth Clients', path: `/dashboard/${tenantId}/oauth-clients` }
		]
	}}
/>

<MainContent>
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div class="space-y-1">
			<h1 class="text-3xl font-bold tracking-tight">OAuth Clients</h1>
			<p class="text-muted-foreground max-w-2xl text-sm">
				Applications registered with the identity provider — each defines its redirect URIs,
				scopes, grant types, and PKCE / consent policy.
			</p>
		</div>

		<Button href={`/dashboard/${tenantId}/oauth-clients/create-oauth-client`}>
			<PlusIcon />
			New client
		</Button>
	</div>

	<div class="mt-6">
		{#if clientsQuery.isPending}
			<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
				{#each Array(3) as _, i (i)}
					<Skeleton class="h-28 w-full rounded-4xl" />
				{/each}
			</div>
		{:else if clientsQuery.isError}
			<ErrorAlert title="Couldn't load clients" message={clientsQuery.error.message} />
		{:else if clientsQuery.data.length === 0}
			<div class="flex flex-col items-center rounded-4xl border border-dashed p-12 text-center">
				<KeyRoundIcon class="text-muted-foreground mb-3 size-8" />
				<p class="font-medium">No OAuth clients yet</p>
				<p class="text-muted-foreground mt-1 text-sm">This tenant has no registered clients.</p>
			</div>
		{:else}
			<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
				{#each clientsQuery.data as client (client.id)}
					<a
						href={`/dashboard/${tenantId}/oauth-clients/${client.id}`}
						class="group focus-visible:outline-none"
					>
						<Card.Root
							size="sm"
							class="group-hover:border-primary/40 group-focus-visible:ring-ring h-full gap-3 transition-all group-hover:shadow-lg group-focus-visible:ring-2"
						>
							<Card.Header>
								<div class="flex items-center justify-between gap-2">
									<Card.Title class="flex min-w-0 items-center gap-2">
										<span
											class="bg-primary/10 text-primary flex size-7 shrink-0 items-center justify-center rounded-lg"
										>
											<KeyRoundIcon class="size-4" />
										</span>
										<span class="truncate">{client.name}</span>
									</Card.Title>
									{#if client.is_active}
										<Badge variant="default">Active</Badge>
									{:else}
										<Badge variant="secondary">Inactive</Badge>
									{/if}
								</div>
								<Card.Description class="truncate pl-9 font-mono text-xs">
									{client.client_id}
								</Card.Description>
							</Card.Header>

							<Card.Content class="flex flex-col gap-2 pl-9">
								<p class="text-muted-foreground text-xs">
									{client.redirect_uris.length} redirect URI{client.redirect_uris.length === 1
										? ''
										: 's'} · {client.allowed_scopes.length} scope{client.allowed_scopes.length === 1
										? ''
										: 's'}
								</p>
								<div class="flex flex-wrap gap-1.5">
									{#each clientBadges(client) as badge (badge.label)}
										<Badge variant={badge.variant}>{badge.label}</Badge>
									{/each}
								</div>
							</Card.Content>
						</Card.Root>
					</a>
				{/each}
			</div>
		{/if}
	</div>
</MainContent>

<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { getMyOrganization } from '$lib/services/admin/organizations';
	import OrganizationForm from './organization-form.svelte';

	const organizationQuery = createQuery(() => ({
		queryKey: ['organization'],
		queryFn: async () => {
			const [data, err] = await getMyOrganization();
			if (err) throw err;
			return data;
		}
	}));

	const organization = $derived(organizationQuery.data);
</script>

<svelte:head><title>Organization - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{ items: [{ name: 'Organization', path: '/dashboard/organization' }] }}
/>

<MainContent title="Organization" description="Manage your organization's details.">
	{#if organizationQuery.isPending}
		<div class="mt-6 flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner /> Loading organization…
		</div>
	{:else if organizationQuery.isError}
		<ErrorAlert
			title="Couldn't load your organization"
			message={organizationQuery.error.message}
		/>
	{:else if organization}
		<div class="mt-6 flex flex-wrap items-center gap-3">
			<h2 class="text-xl font-semibold tracking-tight">{organization.name}</h2>
			{#if organization.isActive}
				<Badge variant="default">Active</Badge>
			{:else}
				<Badge variant="secondary">Inactive</Badge>
			{/if}
		</div>

		<Card.Root class="mt-6 max-w-[640px]">
			<Card.Header>
				<Card.Title>Details</Card.Title>
				<Card.Description>Update your organization's name, description, and status.</Card.Description>
			</Card.Header>
			<Card.Content>
				<OrganizationForm {organization} />
			</Card.Content>
		</Card.Root>

		<Card.Root class="mt-6 max-w-[640px]">
			<Card.Header>
				<Card.Title>Metadata</Card.Title>
			</Card.Header>
			<Card.Content class="grid gap-3 text-sm">
				<div class="grid grid-cols-3 gap-2">
					<span class="text-muted-foreground">Organization ID</span>
					<span class="col-span-2 font-mono text-xs">{organization.organizationId}</span>
				</div>
				<div class="grid grid-cols-3 gap-2">
					<span class="text-muted-foreground">Created</span>
					<span class="col-span-2">{new Date(organization.createdAt).toLocaleString()}</span>
				</div>
				{#if organization.updatedAt}
					<div class="grid grid-cols-3 gap-2">
						<span class="text-muted-foreground">Updated</span>
						<span class="col-span-2">{new Date(organization.updatedAt).toLocaleString()}</span>
					</div>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
</MainContent>

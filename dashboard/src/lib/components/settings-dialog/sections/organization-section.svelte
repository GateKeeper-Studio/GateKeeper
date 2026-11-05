<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import { getMyOrganization } from '$lib/services/admin/organizations';
	import OrganizationForm from './organization-form.svelte';

	// Fires only when this section is first viewed (see AccountSection for the rationale).
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

<header>
	<h1 class="text-lg font-semibold tracking-tight">{m.settings_organization_title()}</h1>
	<p class="text-sm text-muted-foreground">{m.settings_organization_description()}</p>
</header>

{#if organizationQuery.isPending}
	<div class="mt-6 flex items-center gap-2 text-muted-foreground">
		<LoadingSpinner /> {m.settings_organization_loading()}
	</div>
{:else if organizationQuery.isError}
	<div class="mt-6">
		<ErrorAlert title={m.settings_organization_load_error_title()} message={organizationQuery.error.message} />
	</div>
{:else if organization}
	<div class="mt-6 flex flex-wrap items-center gap-3">
		<h2 class="text-xl font-semibold tracking-tight">{organization.name}</h2>
		{#if organization.isActive}
			<Badge variant="default">{m.settings_organization_status_active()}</Badge>
		{:else}
			<Badge variant="secondary">{m.settings_organization_status_inactive()}</Badge>
		{/if}
	</div>

	<Card.Root class="mt-6 max-w-[640px]">
		<Card.Header>
			<Card.Title>{m.settings_organization_details_title()}</Card.Title>
			<Card.Description>{m.settings_organization_details_description()}</Card.Description>
		</Card.Header>
		<Card.Content>
			<OrganizationForm {organization} />
		</Card.Content>
	</Card.Root>

	<Card.Root class="mt-6 max-w-[640px]">
		<Card.Header>
			<Card.Title>{m.settings_organization_metadata_title()}</Card.Title>
		</Card.Header>
		<Card.Content class="grid gap-3 text-sm">
			<div class="grid grid-cols-3 gap-2">
				<span class="text-muted-foreground">{m.settings_organization_metadata_org_id()}</span>
				<span class="col-span-2 font-mono text-xs">{organization.organizationId}</span>
			</div>
			<div class="grid grid-cols-3 gap-2">
				<span class="text-muted-foreground">{m.settings_organization_metadata_created()}</span>
				<span class="col-span-2">{new Date(organization.createdAt).toLocaleString()}</span>
			</div>
			{#if organization.updatedAt}
				<div class="grid grid-cols-3 gap-2">
					<span class="text-muted-foreground">{m.settings_organization_metadata_updated()}</span>
					<span class="col-span-2">{new Date(organization.updatedAt).toLocaleString()}</span>
				</div>
			{/if}
		</Card.Content>
	</Card.Root>
{/if}

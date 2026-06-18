<script lang="ts">
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';

	import * as Card from '$lib/components/ui/card/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { getTenant } from '$lib/services/admin/tenants';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import TenantEditForm from './tenant-edit-form.svelte';

	const tenantId = $derived(page.params.tenantId!);

	const tenantQuery = createQuery(() => ({
		queryKey: ['tenant', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await getTenant(tenantId);
			if (err) throw err;
			return data;
		}
	}));

	const tenant = $derived(tenantQuery.data);
	const fallbackName = $derived(selectedTenant.current?.name ?? 'Tenant');
</script>

<svelte:head><title>Edit {tenant?.name ?? fallbackName} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: tenant?.name ?? fallbackName, path: `/dashboard/${tenantId}` },
			{ name: 'Edit', path: `/dashboard/${tenantId}/edit` }
		]
	}}
/>

<MainContent
	goBack={{ message: 'Back to tenant', href: `/dashboard/${tenantId}` }}
	title="Edit tenant"
	description="Update this tenant's name, status, and self-service settings."
>
	{#if tenantQuery.isPending}
		<div class="mt-6 flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner /> Loading tenant…
		</div>
	{:else if tenantQuery.isError}
		<ErrorAlert title="Couldn't load this tenant" message={tenantQuery.error.message} />
	{:else if tenant}
		<Card.Root class="mt-6 max-w-[640px]">
			<Card.Content class="pt-6">
				<TenantEditForm {tenant} />
			</Card.Content>
		</Card.Root>
	{/if}
</MainContent>

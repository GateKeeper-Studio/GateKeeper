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
	import { m } from '$lib/paraglide/messages.js';

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
	const fallbackName = $derived(selectedTenant.current?.name ?? m.sidebar_tenant());
</script>

<svelte:head><title>Edit {tenant?.name ?? fallbackName} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: tenant?.name ?? fallbackName, path: `/tenants/${tenantId}` },
			{ name: m.tenant_edit_button(), path: `/tenants/${tenantId}/edit` }
		]
	}}
/>

<MainContent
	goBack={{ message: m.tenant_back_to_tenant(), href: `/tenants/${tenantId}` }}
	title={m.tenant_edit_tenant_title()}
	description={m.tenant_edit_tenant_description()}
>
	{#if tenantQuery.isPending}
		<div class="mt-6 flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner /> {m.tenant_loading()}
		</div>
	{:else if tenantQuery.isError}
		<ErrorAlert title={m.tenant_load_error_title()} message={tenantQuery.error.message} />
	{:else if tenant}
		<Card.Root class="mt-6 max-w-[640px]">
			<Card.Content class="pt-6">
				<TenantEditForm {tenant} />
			</Card.Content>
		</Card.Root>
	{/if}
</MainContent>

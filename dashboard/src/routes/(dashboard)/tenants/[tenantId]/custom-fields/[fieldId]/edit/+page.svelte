<script lang="ts">
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';

	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { getCustomField } from '$lib/services/admin/custom-fields';
	import { resolveLocalized } from '$lib/i18n/resolve-localized';
	import { m } from '$lib/paraglide/messages.js';
	import CustomFieldForm from '../../custom-field-form.svelte';

	const tenantId = $derived(page.params.tenantId!);
	const fieldId = $derived(page.params.fieldId!);
	const listHref = $derived(`/tenants/${tenantId}/custom-fields`);
	const tenantDefaultLanguage = $derived(selectedTenant.current?.defaultLanguage ?? 'en');

	const fieldQuery = createQuery(() => ({
		queryKey: ['custom-fields', tenantId, fieldId],
		enabled: !!tenantId && !!fieldId,
		queryFn: async () => {
			const [data, err] = await getCustomField(tenantId, fieldId);
			if (err) throw err;
			return data;
		}
	}));

	const field = $derived(fieldQuery.data);
	const fieldLabel = $derived(
		field ? resolveLocalized(field.label, tenantDefaultLanguage, tenantDefaultLanguage) : ''
	);
</script>

<svelte:head>
	<title
		>{field
			? m.custom_fields_edit_page_title_with_name({ label: fieldLabel })
			: m.custom_fields_edit_page_title()} - GateKeeper</title
	>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.custom_fields_title(), path: listHref },
			{ name: fieldLabel || '…', path: `${listHref}/${fieldId}/edit` }
		]
	}}
/>

<MainContent
	title={m.custom_fields_edit_page_title()}
	description={m.custom_fields_edit_page_description()}
	goBack={{ message: m.custom_fields_back_to_list(), href: listHref }}
>
	{#if fieldQuery.isPending}
		<div class="mt-4 flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner />
			{m.custom_fields_loading_field()}
		</div>
	{:else if fieldQuery.isError}
		<div class="mt-4">
			<ErrorAlert
				title={m.custom_fields_load_field_error_title()}
				message={fieldQuery.error.message}
			/>
		</div>
	{:else if field}
		{#key field.id}
			<CustomFieldForm mode="edit" {tenantId} {field} />
		{/key}
	{/if}
</MainContent>

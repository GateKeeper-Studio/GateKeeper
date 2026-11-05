<script lang="ts">
	import { page } from '$app/state';

	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import CustomFieldForm from '../custom-field-form.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(page.params.tenantId!);
	const listHref = $derived(`/tenants/${tenantId}/custom-fields`);
</script>

<svelte:head><title>{m.custom_fields_create_page_title()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.custom_fields_title(), path: listHref },
			{ name: m.custom_fields_create_breadcrumb(), path: `${listHref}/new` }
		]
	}}
/>

<MainContent
	title={m.custom_fields_create_page_title()}
	description={m.custom_fields_create_page_description()}
	goBack={{ message: m.custom_fields_back_to_list(), href: listHref }}
>
	<CustomFieldForm mode="create" {tenantId} />
</MainContent>

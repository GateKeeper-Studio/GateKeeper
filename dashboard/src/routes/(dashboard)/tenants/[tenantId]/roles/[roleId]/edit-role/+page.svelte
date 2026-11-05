<script lang="ts">
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { getTenantRole } from '$lib/services/admin/roles';
	import RoleForm from '../../role-form.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(page.params.tenantId!);
	const roleId = $derived(page.params.roleId!);
	const listHref = $derived(`/tenants/${tenantId}/roles`);

	const roleQuery = createQuery(() => ({
		queryKey: ['tenant-roles', tenantId, roleId],
		enabled: !!tenantId && !!roleId,
		queryFn: async () => {
			const [data, err] = await getTenantRole(tenantId, roleId);
			if (err) throw err;
			return data;
		}
	}));

	const role = $derived(roleQuery.data);
</script>

<svelte:head>
	<title>{role ? m.roles_edit_page_title_with_name({ name: role.name }) : m.roles_edit_role_title()} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.roles_tenant_fallback(), path: `/tenants/${tenantId}` },
			{ name: m.roles_page_title(), path: listHref },
			{ name: role?.name ?? '…', path: `${listHref}/${roleId}` },
			{ name: m.roles_edit_label(), path: `${listHref}/${roleId}/edit-role` }
		]
	}}
/>

<MainContent
	goBack={{ message: m.roles_back_to_roles(), href: listHref }}
	title={m.roles_edit_role_title()}
	description={m.roles_edit_role_description()}
>
	{#if roleQuery.isPending}
		<div class="text-muted-foreground mt-4 flex items-center gap-2">
			<LoadingSpinner /> {m.roles_loading_role()}
		</div>
	{:else if roleQuery.isError}
		<ErrorAlert title={m.roles_load_role_error_title()} message={roleQuery.error.message} />
	{:else if role}
		{#key role.role_id}
			<RoleForm mode="edit" {tenantId} {role} />
		{/key}
	{/if}
</MainContent>

<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import Building2Icon from '@lucide/svelte/icons/building-2';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import ShieldOffIcon from '@lucide/svelte/icons/shield-off';
	import UserIcon from '@lucide/svelte/icons/user';
	import GroupIcon from '@lucide/svelte/icons/group';
	import UserKeyIcon from '@lucide/svelte/icons/user-key';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import CheckIcon from '@lucide/svelte/icons/check';
	import CalendarPlusIcon from '@lucide/svelte/icons/calendar-plus';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import UserPlusIcon from '@lucide/svelte/icons/user-plus';
	import KeyIcon from '@lucide/svelte/icons/key';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import Separator from '$lib/components/ui/separator/separator.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { getTenant, deleteTenant } from '$lib/services/admin/tenants';
	import { listTenantUsers } from '$lib/services/admin/users';
	import { listOAuthClients } from '$lib/services/admin/oauth-clients';
	import { listTenantRoles } from '$lib/services/admin/roles';
	import { listTenantGroups } from '$lib/services/admin/groups';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(page.params.tenantId!);

	// Source of truth for the page. `selectedTenant.current` gives an instant name
	// from the sidebar list, but we fetch the full record so a hard refresh works too.
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

	// ---- resource counts (each independent so the cards fill in as they resolve) ----
	const usersCount = createQuery(() => ({
		queryKey: ['tenant-users-count', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listTenantUsers(tenantId, { page: 1, pageSize: 1 });
			if (err) throw err;
			return data.total;
		}
	}));

	const clientsCount = createQuery(() => ({
		queryKey: ['tenant-clients-count', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listOAuthClients(tenantId);
			if (err) throw err;
			return data.total;
		}
	}));

	const rolesCount = createQuery(() => ({
		queryKey: ['tenant-roles-count', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listTenantRoles(tenantId, { page: 1, pageSize: 1 });
			if (err) throw err;
			return data.total;
		}
	}));

	const groupsCount = createQuery(() => ({
		queryKey: ['tenant-groups-count', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listTenantGroups(tenantId, { page: 1, pageSize: 1 });
			if (err) throw err;
			return data.total;
		}
	}));

	function countValue(q: { isPending: boolean; isError: boolean; data?: number }): string {
		if (q.isPending) return '…';
		if (q.isError) return '—';
		return String(q.data ?? 0);
	}

	// ---- helpers -----------------------------------------------------------------
	function formatDate(iso: string | null | undefined): string {
		if (!iso) return '—';
		const date = new Date(iso);
		if (Number.isNaN(date.getTime())) return iso;
		return date.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
	}

	let tenantIdCopied = $state(false);

	async function copyTenantId() {
		try {
			await navigator.clipboard.writeText(tenantId);
			tenantIdCopied = true;
			setTimeout(() => (tenantIdCopied = false), 1500);
			toast.success(m.tenant_id_copied());
		} catch {
			toast.error(m.tenant_id_copy_failed());
		}
	}

	// ---- delete -------------------------------------------------------------------
	const queryClient = useQueryClient();
	let deleteOpen = $state(false);
	let deleting = $state(false);

	async function confirmDelete() {
		deleting = true;
		const [err] = await deleteTenant(tenantId);
		deleting = false;

		if (err) {
			toast.error(err.data?.message || err.data?.error || err.message);
			return;
		}

		// Drop it from the sidebar list, then reconcile with the server. Selection
		// follows the URL, so navigate to
		// another remaining tenant if there is one, otherwise the dashboard root.
		const remaining = selectedTenant.tenants.filter((t) => t.tenantId !== tenantId);
		selectedTenant.tenants = remaining;
		queryClient.invalidateQueries({ queryKey: ['tenants'] });

		// Move to another tenant, or forget the active one and land on the empty state.
		toast.success(m.tenant_deleted());
		deleteOpen = false;
		if (remaining.length > 0) {
			await goto(`/tenants/${remaining[0].tenantId}`);
		} else {
			selectedTenant.clearActive();
			await goto('/');
		}
	}
</script>

<svelte:head>
	<title>{tenant?.name ?? fallbackName} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [{ name: tenant?.name ?? fallbackName, path: `/tenants/${tenantId}` }]
	}}
/>

{#snippet statTile(label: string, value: string, Icon: any, chip: string, href: string)}
	<a
		{href}
		class="group flex items-center gap-3 rounded-xl border bg-card p-4 transition-shadow hover:shadow-sm"
	>
		<span class={cn('flex size-10 shrink-0 items-center justify-center rounded-lg', chip)}>
			<Icon class="size-5" />
		</span>
		<div class="min-w-0 flex-1">
			<p class="text-xs font-medium text-muted-foreground">{label}</p>
			<p class="truncate text-lg font-semibold">{value}</p>
		</div>
		<ChevronRightIcon
			class="size-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5"
		/>
	</a>
{/snippet}

{#snippet settingRow(label: string, description: string, enabled: boolean, Icon: any)}
	<div class="flex items-center gap-3">
		<span
			class={cn(
				'flex size-9 shrink-0 items-center justify-center rounded-lg',
				enabled
					? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
					: 'bg-muted text-muted-foreground'
			)}
		>
			<Icon class="size-4" />
		</span>
		<div class="min-w-0 flex-1">
			<p class="text-sm font-medium">{label}</p>
			<p class="text-xs text-muted-foreground">{description}</p>
		</div>
		{#if enabled}
			<Badge
				variant="outline"
				class="shrink-0 border-emerald-500/20 bg-emerald-500/5 text-emerald-700 dark:text-emerald-400"
			>
				{m.oauth_clients_overview_enabled()}
			</Badge>
		{:else}
			<Badge variant="outline" class="shrink-0 text-muted-foreground"
				>{m.oauth_clients_overview_disabled()}</Badge
			>
		{/if}
	</div>
{/snippet}

{#snippet infoRow(label: string, value: string, Icon: any, chip: string)}
	<div class="flex items-center gap-3">
		<span class={cn('flex size-9 shrink-0 items-center justify-center rounded-lg', chip)}>
			<Icon class="size-4" />
		</span>
		<div class="min-w-0">
			<p class="text-xs text-muted-foreground">{label}</p>
			<p class="truncate text-sm font-medium">{value}</p>
		</div>
	</div>
{/snippet}

<MainContent>
	{#if tenantQuery.isPending}
		<div class="flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner />
			{m.tenant_loading()}
		</div>
	{:else if tenantQuery.isError}
		<ErrorAlert title={m.tenant_load_error_title()} message={tenantQuery.error.message} />
	{:else if tenant}
		<div class="space-y-6">
			<!-- Header -->
			<div class="flex flex-wrap items-start justify-between gap-4">
				<div class="flex items-start gap-4">
					<span
						class="flex size-12 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary"
					>
						<Building2Icon class="size-6" />
					</span>
					<div class="space-y-1.5">
						<div class="flex flex-wrap items-center gap-2">
							<h1 class="text-2xl font-bold tracking-tight">{tenant.name}</h1>
							{#if tenant.is_active}
								<Badge
									variant="outline"
									class="gap-1 border-emerald-500/20 bg-emerald-500/5 text-emerald-700 dark:text-emerald-400"
								>
									<ShieldCheckIcon class="size-3" />
									{m.tenant_active_label()}
								</Badge>
							{:else}
								<Badge variant="outline" class="gap-1 text-muted-foreground">
									<ShieldOffIcon class="size-3" />
									{m.tenant_inactive_badge()}
								</Badge>
							{/if}
						</div>
						{#if tenant.description}
							<p class="max-w-prose text-sm text-muted-foreground">{tenant.description}</p>
						{/if}
						<div class="flex items-center gap-2 pt-1">
							<code class="font-mono text-xs text-muted-foreground">{tenantId}</code>
							<Button
								variant="ghost"
								size="icon"
								class="size-6 shrink-0 text-muted-foreground hover:text-foreground"
								onclick={copyTenantId}
							>
								{#if tenantIdCopied}<CheckIcon
										class="text-emerald-600 dark:text-emerald-400"
									/>{:else}<CopyIcon />{/if}
							</Button>
						</div>
					</div>
				</div>

				<div class="flex flex-wrap items-center gap-2">
					<Button variant="outline" href={`/tenants/${tenantId}/edit`}>
						<PencilIcon />
						{m.tenant_edit_button()}
					</Button>
					<Button variant="destructive" onclick={() => (deleteOpen = true)}>
						<Trash2Icon />
						{m.tenant_delete_button()}
					</Button>
					<Button variant="outline" href={`/tenants/${tenantId}/users`}>
						<UserIcon />
						{m.tenant_manage_users_button()}
					</Button>
				</div>
			</div>

			<!-- Resource counts -->
			<div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
				{@render statTile(
					m.nav_users(),
					countValue(usersCount),
					UserIcon,
					'bg-sky-500/10 text-sky-600 dark:text-sky-400',
					`/tenants/${tenantId}/users`
				)}
				{@render statTile(
					m.nav_oauth_clients(),
					countValue(clientsCount),
					KeyRoundIcon,
					'bg-violet-500/10 text-violet-600 dark:text-violet-400',
					`/tenants/${tenantId}/oauth-clients`
				)}
				{@render statTile(
					m.nav_roles(),
					countValue(rolesCount),
					UserKeyIcon,
					'bg-amber-500/10 text-amber-600 dark:text-amber-400',
					`/tenants/${tenantId}/roles`
				)}
				{@render statTile(
					m.nav_groups(),
					countValue(groupsCount),
					GroupIcon,
					'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400',
					`/tenants/${tenantId}/groups`
				)}
			</div>

			<div class="grid items-start gap-4 lg:grid-cols-2">
				<!-- Self-service settings -->
				<Card.Root>
					<Card.Header>
						<Card.Title class="flex items-center gap-2">
							<ShieldCheckIcon class="size-4 text-muted-foreground" />
							{m.tenant_self_service_title()}
						</Card.Title>
						<Card.Description>{m.tenant_self_service_description()}</Card.Description>
					</Card.Header>

					<Card.Content class="space-y-4">
						{@render settingRow(
							m.tenant_signup_label(),
							m.tenant_signup_description(),
							tenant.can_self_register,
							UserPlusIcon
						)}
						<Separator />
						{@render settingRow(
							m.tenant_password_reset_label(),
							m.tenant_password_reset_description(),
							tenant.can_self_forgot_password,
							KeyIcon
						)}
					</Card.Content>
				</Card.Root>

				<!-- Lifecycle -->
				<Card.Root>
					<Card.Header>
						<Card.Title class="flex items-center gap-2">
							<ClockIcon class="size-4 text-muted-foreground" />
							{m.tenant_lifecycle_title()}
						</Card.Title>
						<Card.Description>{m.tenant_lifecycle_description()}</Card.Description>
					</Card.Header>
					<Card.Content class="space-y-3">
						{@render infoRow(
							m.tenant_created_label(),
							formatDate(tenant.created_at),
							CalendarPlusIcon,
							'bg-sky-500/10 text-sky-600 dark:text-sky-400'
						)}
						{@render infoRow(
							m.tenant_last_updated_label(),
							formatDate(tenant.updated_at),
							ClockIcon,
							'bg-violet-500/10 text-violet-600 dark:text-violet-400'
						)}
					</Card.Content>
				</Card.Root>
			</div>
		</div>
	{/if}
</MainContent>

<Dialog.Root bind:open={deleteOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>{m.tenant_delete_dialog_title({ name: tenant?.name ?? '' })}</Dialog.Title>
			<Dialog.Description>
				{m.tenant_delete_dialog_description()}
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" disabled={deleting} onclick={() => (deleteOpen = false)}>
				{m.tenant_cancel_button()}
			</Button>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{#if deleting}<LoadingSpinner />{/if}
				{m.tenant_delete_tenant_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

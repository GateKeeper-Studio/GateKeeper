<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import { adminLogout } from '$lib/services/auth/admin-login';
	import { getMyProfile, deactivateMyAccount } from '$lib/services/admin/account';
	import AccountForm from './account-form.svelte';
	import PasswordForm from './password-form.svelte';

	// Fires only when this section is first viewed — the settings shell renders the
	// active section lazily, so no request runs on dialog open or for unviewed sections.
	const profileQuery = createQuery(() => ({
		queryKey: ['account'],
		queryFn: async () => {
			const [data, err] = await getMyProfile();
			if (err) throw err;
			return data;
		}
	}));

	const profile = $derived(profileQuery.data);

	// ---- deactivation -------------------------------------------------------------
	let deactivateOpen = $state(false);
	let deactivating = $state(false);

	async function confirmDeactivate() {
		deactivating = true;
		const [err] = await deactivateMyAccount();

		if (err) {
			deactivating = false;
			toast.error(err.data?.message || err.message || m.settings_account_deactivate_error());
			return;
		}

		// The account is now disabled and every session revoked server-side. Mirror the
		// sidebar's sign-out: best-effort revoke, drop the session cookie, return to
		// /login.
		await adminLogout();
		await goto(resolve('/login'));
	}
</script>

<header>
	<h1 class="text-lg font-semibold tracking-tight">{m.settings_account_title()}</h1>
	<p class="text-sm text-muted-foreground">{m.settings_account_description()}</p>
</header>

{#if profileQuery.isPending}
	<div class="mt-6 flex items-center gap-2 text-muted-foreground">
		<LoadingSpinner />
		{m.settings_account_loading()}
	</div>
{:else if profileQuery.isError}
	<div class="mt-6">
		<ErrorAlert
			title={m.settings_account_load_error_title()}
			message={profileQuery.error.message}
		/>
	</div>
{:else if profile}
	<div class="mt-6 flex flex-wrap items-center gap-3">
		<h2 class="text-xl font-semibold tracking-tight">{profile.name || profile.email}</h2>
		{#if profile.isActive}
			<Badge variant="default">{m.settings_account_status_active()}</Badge>
		{:else}
			<Badge variant="secondary">{m.settings_account_status_inactive()}</Badge>
		{/if}
	</div>

	<Card.Root class="mt-6 max-w-[640px]">
		<Card.Header>
			<Card.Title>{m.settings_account_profile_title()}</Card.Title>
			<Card.Description>{m.settings_account_profile_description()}</Card.Description>
		</Card.Header>
		<Card.Content>
			<AccountForm {profile} />
		</Card.Content>
	</Card.Root>

	<Card.Root class="mt-6 max-w-[640px]">
		<Card.Header>
			<Card.Title>{m.settings_password_title()}</Card.Title>
			<Card.Description>{m.settings_password_description()}</Card.Description>
		</Card.Header>
		<Card.Content>
			<PasswordForm />
		</Card.Content>
	</Card.Root>

	<Card.Root class="mt-6 max-w-[640px]">
		<Card.Header>
			<Card.Title>{m.settings_account_metadata_title()}</Card.Title>
		</Card.Header>
		<Card.Content class="grid gap-3 text-sm">
			<div class="grid grid-cols-3 gap-2">
				<span class="text-muted-foreground">{m.settings_account_metadata_admin_id()}</span>
				<span class="col-span-2 font-mono text-xs">{profile.id}</span>
			</div>
			<div class="grid grid-cols-3 gap-2">
				<span class="text-muted-foreground">{m.settings_account_metadata_org_id()}</span>
				<span class="col-span-2 font-mono text-xs">{profile.organizationId}</span>
			</div>
			<div class="grid grid-cols-3 gap-2">
				<span class="text-muted-foreground">{m.settings_account_metadata_created()}</span>
				<span class="col-span-2">{new Date(profile.createdAt).toLocaleString()}</span>
			</div>
			{#if profile.updatedAt}
				<div class="grid grid-cols-3 gap-2">
					<span class="text-muted-foreground">{m.settings_account_metadata_updated()}</span>
					<span class="col-span-2">{new Date(profile.updatedAt).toLocaleString()}</span>
				</div>
			{/if}
		</Card.Content>
	</Card.Root>

	<Card.Root class="mt-6 max-w-[640px] border-destructive/40">
		<Card.Header>
			<Card.Title class="text-destructive">{m.settings_account_deactivate_title()}</Card.Title>
			<Card.Description>
				{m.settings_account_deactivate_description()}
			</Card.Description>
		</Card.Header>
		<Card.Content>
			<Button variant="destructive" onclick={() => (deactivateOpen = true)}>
				{m.settings_account_deactivate_button()}
			</Button>
		</Card.Content>
	</Card.Root>
{/if}

<Dialog.Root bind:open={deactivateOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>{m.settings_account_deactivate_confirm_title()}</Dialog.Title>
			<Dialog.Description>
				{m.settings_account_deactivate_confirm_description()}
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" disabled={deactivating} onclick={() => (deactivateOpen = false)}>
				{m.settings_account_deactivate_cancel()}
			</Button>
			<Button
				variant="destructive"
				disabled={deactivating}
				onclick={confirmDeactivate}
				class="relative"
			>
				{#if deactivating}<LoadingSpinner class="absolute left-3" />{/if}
				{deactivating ? m.settings_account_deactivating() : m.settings_account_deactivate_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

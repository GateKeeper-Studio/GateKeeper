<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import SaveIcon from '@lucide/svelte/icons/save';
	import { m } from '$lib/paraglide/messages.js';

	import { updateMyProfile, type AdminProfileView } from '$lib/services/admin/account';

	let { profile }: { profile: AdminProfileView } = $props();

	const queryClient = useQueryClient();

	let name = $state(profile.name);
	let email = $state(profile.email);

	let error = $state('');
	let isLoading = $state(false);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		if (name.trim().length === 0) {
			error = m.settings_account_name_required();
			return;
		}
		if (email.trim().length === 0) {
			error = m.settings_account_email_required();
			return;
		}
		error = '';
		isLoading = true;

		const [, err] = await updateMyProfile({ name: name.trim(), email: email.trim() });
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || err.message || m.settings_account_update_error());
			return;
		}

		// The sidebar reads name/email from the access token, so it reflects this change
		// only after the next token refresh or sign-in. The account section re-fetches.
		toast.success(m.settings_account_update_success());
		queryClient.invalidateQueries({ queryKey: ['account'] });
	}
</script>

<form onsubmit={onSubmit} class="grid max-w-[640px] gap-6">
	<div class="space-y-2">
		<Label for="name" class="flex gap-1">
			{m.settings_account_name_label()}
			<span class="text-red-500">*</span>
		</Label>
		<Input id="name" type="text" placeholder={m.settings_account_name_placeholder()} bind:value={name} />
	</div>

	<div class="space-y-2">
		<Label for="email" class="flex gap-1">
			{m.settings_account_email_label()}
			<span class="text-red-500">*</span>
		</Label>
		<Input id="email" type="email" placeholder={m.settings_account_email_placeholder()} bind:value={email} />
		<p class="text-xs text-muted-foreground">{m.settings_account_email_helper()}</p>
		{#if error}<p class="text-sm font-medium text-destructive">{error}</p>{/if}
	</div>

	<div class="flex justify-end">
		<Button type="submit" disabled={isLoading} class="w-fit">
			{#if isLoading}<LoadingSpinner class="size-4" />{:else}<SaveIcon />{/if}
			{isLoading ? m.settings_account_saving() : m.settings_account_save_button()}
		</Button>
	</div>
</form>

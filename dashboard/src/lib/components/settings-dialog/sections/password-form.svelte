<script lang="ts">
	import { toast } from 'svelte-sonner';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import { changeMyPassword } from '$lib/services/admin/account';

	// Keep in sync with the server's `new_password` length validation.
	const MIN_LENGTH = 8;

	let currentPassword = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');

	let error = $state('');
	let isLoading = $state(false);

	function reset() {
		currentPassword = '';
		newPassword = '';
		confirmPassword = '';
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		if (currentPassword.length === 0) {
			error = m.settings_password_current_required();
			return;
		}
		if (newPassword.length < MIN_LENGTH) {
			error = m.settings_password_min_length({ minLength: MIN_LENGTH });
			return;
		}
		if (newPassword !== confirmPassword) {
			error = m.settings_password_mismatch();
			return;
		}
		error = '';
		isLoading = true;

		const [err] = await changeMyPassword({ currentPassword, newPassword });
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || err.message || m.settings_password_change_error());
			return;
		}

		toast.success(m.settings_password_change_success());
		reset();
	}
</script>

<form onsubmit={onSubmit} class="grid max-w-[640px] gap-6">
	<div class="space-y-2">
		<Label for="current-password">{m.settings_password_current_label()}</Label>
		<Input
			id="current-password"
			type="password"
			autocomplete="current-password"
			bind:value={currentPassword}
		/>
	</div>

	<div class="space-y-2">
		<Label for="new-password">{m.settings_password_new_label()}</Label>
		<Input id="new-password" type="password" autocomplete="new-password" bind:value={newPassword} />
	</div>

	<div class="space-y-2">
		<Label for="confirm-password">{m.settings_password_confirm_label()}</Label>
		<Input
			id="confirm-password"
			type="password"
			autocomplete="new-password"
			bind:value={confirmPassword}
		/>
		{#if error}<p class="text-sm font-medium text-destructive">{error}</p>{/if}
	</div>

	<div class="flex justify-end">
		<Button type="submit" disabled={isLoading} class="relative w-fit">
			{#if isLoading}<LoadingSpinner class="absolute left-3" />{/if}
			{isLoading ? m.settings_password_saving() : m.settings_password_change_button()}
		</Button>
	</div>
</form>

<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';
	import CheckCircleIcon from '@lucide/svelte/icons/circle-check';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import { formSchema } from './schema';
	import { resetPasswordApi } from '$lib/services/auth/reset-password';
	import { m } from '$lib/paraglide/messages.js';

	const clientId = page.params.clientId as string;

	const passwordResetToken = page.url.searchParams.get('token');
	const passwordResetId = page.url.searchParams.get('id');
	const alreadyChanged = $derived(page.url.searchParams.get('changed') === 'true');

	let password = $state('');
	let confirmPassword = $state('');
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(formSchema, { password, confirmPassword });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};

		if (!clientId) {
			toast.error(m.auth_change_password_client_id_required());
			return;
		}
		if (!passwordResetToken) {
			toast.error(m.auth_change_password_token_required());
			return;
		}
		if (!passwordResetId) {
			toast.error(m.auth_change_password_reset_id_required());
			return;
		}

		isLoading = true;

		const [err] = await resetPasswordApi({
			clientId,
			newPassword: result.data.password,
			passwordResetToken,
			passwordResetId
		});

		if (err) {
			console.error(err);
			toast.error(err.data?.message || err.message);
			isLoading = false;
			return;
		}

		isLoading = false;
		toast.success(m.auth_change_password_reset_success());

		await goto(resolve(`/auth/[clientId]/change-password?changed=true`, { clientId }));
	}
</script>

{#if alreadyChanged}
	<div class="grid gap-4">
		<Alert.Root variant="default" class="bg-green-100 dark:bg-green-700">
			<CheckCircleIcon class="h-5 w-5 text-green-500 dark:text-green-400" />
			<Alert.Title class="font-bold">{m.auth_change_password_success_title()}</Alert.Title>
			<Alert.Description class="dark:text-gray-100">
				{m.auth_change_password_success_description()} <br />
			</Alert.Description>
		</Alert.Root>

		<span class="mt-1 text-center font-semibold">
			{m.auth_change_password_success_hint()}
		</span>
	</div>
{:else}
	<div class="grid gap-4">
		<form onsubmit={onSubmit} class="space-y-3">
			<div class="space-y-2">
				<Label for="password">{m.auth_password_label()}</Label>
				<Input
					id="password"
					type="password"
					autocomplete="new-password"
					placeholder="********"
					bind:value={password}
				/>
				{#if errors.password}
					<p class="text-sm font-medium text-destructive">{errors.password}</p>
				{/if}
			</div>

			<div class="space-y-2">
				<Label for="confirmPassword">{m.auth_change_password_confirm_password_label()}</Label>
				<Input
					id="confirmPassword"
					type="password"
					autocomplete="new-password"
					placeholder="********"
					bind:value={confirmPassword}
				/>
				{#if errors.confirmPassword}
					<p class="text-sm font-medium text-destructive">{errors.confirmPassword}</p>
				{/if}
			</div>

			<Button type="submit" disabled={isLoading} class="relative w-full">
				{#if isLoading}
					<LoadingSpinner class="absolute left-4" />
				{/if}
				{m.auth_change_password_submit()}
			</Button>
		</form>
	</div>
{/if}

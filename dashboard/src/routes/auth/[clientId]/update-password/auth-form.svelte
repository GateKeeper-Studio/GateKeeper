<script lang="ts">
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import { formSchema } from './schema';
	import { changePasswordApi } from '$lib/services/auth/change-password';
	import { authorizeApi } from '$lib/services/auth/authorize';

	const clientId = page.params.clientId as string;
	const searchParams = page.url.searchParams;

	const redirectUri = searchParams.get('redirect_uri') || '/';
	const codeChallengeMethod = searchParams.get('code_challenge_method') || '';
	const responseType = searchParams.get('response_type') || '';
	const scope = searchParams.get('scope') || '';
	const oidcState = searchParams.get('state') || '';
	const email = searchParams.get('email') || '';
	const codeChallenge = searchParams.get('code_challenge') || '';
	const sessionCode = searchParams.get('session_code') || '';
	const changePasswordCode = searchParams.get('change_password_code') || '';
	const userId = searchParams.get('user_id') || '';

	let password = $state('');
	let confirmPassword = $state('');
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);
	let error = $state<string | null>(null);

	function showError(message: string) {
		error = message;
		isLoading = false;
		setTimeout(() => (error = null), 6000);
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(formSchema, { password, confirmPassword });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};

		if (!clientId) {
			toast.error('Application ID is required');
			return;
		}
		if (!changePasswordCode) {
			toast.error('Change password code is required');
			return;
		}
		if (!userId) {
			toast.error('User ID is required');
			return;
		}

		isLoading = true;

		const [err] = await changePasswordApi({
			clientId,
			newPassword: result.data.password,
			changePasswordCode,
			userId
		});

		if (err) {
			console.error(err);
			toast.error(err.data?.message || err.message);
			isLoading = false;
			return;
		}

		const [authorizeData, authorizeErr] = await authorizeApi({
			email: email.trim(),
			sessionCode,
			clientId,
			redirectUri,
			responseType,
			scope,
			codeChallengeMethod,
			codeChallenge,
			state: oidcState
		});

		if (authorizeErr) {
			console.error(authorizeErr);
			showError(authorizeErr.data?.message || 'An error occurred');
			return;
		}

		if (!authorizeData) {
			showError('An error occurred');
			return;
		}

		isLoading = false;
		toast.success('Password was changed successfully');

		window.location.href = `${redirectUri}?code=${authorizeData.authorizationCode}&state=${oidcState}&redirect_uri=${redirectUri}&client_id=${clientId}`;
	}
</script>

<div class="grid gap-4">
	{#if error}
		<ErrorAlert message={error} title="An error occurred..." />
	{/if}

	<form onsubmit={onSubmit} class="space-y-3">
		<div class="space-y-2">
			<Label for="password">Password</Label>
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
			<Label for="confirmPassword">Confirm Password</Label>
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
			Save Password
		</Button>
	</form>
</div>

<script lang="ts">
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as InputOTP from '$lib/components/ui/input-otp/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import { formSchema } from './schema';
	import { confirmEmailApi } from '$lib/services/auth/confirm-email';
	import { m } from '$lib/paraglide/messages.js';

	const clientId = page.params.clientId as string;
	const searchParams = page.url.searchParams;

	const scope = searchParams.get('scope') || '';
	const oidcState = searchParams.get('state') || '';
	const email = searchParams.get('email') || '/';
	const redirectUri = searchParams.get('redirect_uri') || '/';
	const responseType = searchParams.get('response_type') || '';
	const codeChallenge = searchParams.get('code_challenge') || '';
	const codeChallengeMethod = searchParams.get('code_challenge_method') || '';
	const nonce = searchParams.get('nonce') || '';

	let code = $state('');
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

		const result = validate(formSchema, { code });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const [data, err] = await confirmEmailApi({
			clientId,
			token: result.data.code,
			email,
			codeChallengeMethod,
			responseType,
			scope,
			state: oidcState,
			codeChallenge,
			redirectUri,
			nonce
		});

		if (err) {
			console.error(err);
			toast.error(err.data?.message || err.message);
			isLoading = false;
			return;
		}

		if (!data) {
			showError(m.auth_generic_error());
			return;
		}

		isLoading = false;
		toast.success(m.auth_confirm_email_success());

		window.location.href = `${redirectUri}?code=${data.authorizationCode}&state=${oidcState}&redirect_uri=${redirectUri}&client_id=${clientId}`;
	}
</script>

<div class="grid gap-4">
	{#if error}
		<ErrorAlert message={error} title={m.auth_error_title()} />
	{/if}

	<form onsubmit={onSubmit} class="flex w-full flex-col items-center space-y-3">
		<div class="space-y-2">
			<InputOTP.Root maxlength={6} bind:value={code}>
				{#snippet children({ cells })}
					<InputOTP.Group>
						{#each cells.slice(0, 3) as cell, i (i)}
							<InputOTP.Slot {cell} />
						{/each}
					</InputOTP.Group>

					<InputOTP.Separator />

					<InputOTP.Group>
						{#each cells.slice(3, 6) as cell, i (i)}
							<InputOTP.Slot {cell} />
						{/each}
					</InputOTP.Group>
				{/snippet}
			</InputOTP.Root>
			{#if errors.code}
				<p class="text-sm font-medium text-destructive">{errors.code}</p>
			{/if}
		</div>

		<Button type="submit" disabled={isLoading} class="relative w-full">
			{#if isLoading}
				<LoadingSpinner class="absolute left-4" />
			{/if}
			{m.auth_confirm_email_submit()}
		</Button>
	</form>
</div>

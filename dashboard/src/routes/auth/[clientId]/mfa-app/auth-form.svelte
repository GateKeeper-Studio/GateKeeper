<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as InputOTP from '$lib/components/ui/input-otp/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MfaModal from '$lib/components/auth/mfa-modal.svelte';

	import { validate, buildOidcParams } from '$lib/utils/form';
	import { formSchema } from './schema';
	import { authorizeApi } from '$lib/services/auth/authorize';
	import { verifyAppMfaApi } from '$lib/services/auth/verify-app-mfa';
	import { EMfaType } from '$lib/services/auth/login';

	const clientId = page.params.clientId as string;
	const searchParams = page.url.searchParams;

	const redirectUri = searchParams.get('redirect_uri') || '/';
	const codeChallengeMethod = searchParams.get('code_challenge_method') || '';
	const responseType = searchParams.get('response_type') || '';
	const scope = searchParams.get('scope') || '';
	const oidcState = searchParams.get('state') || '';
	const email = searchParams.get('email') || '';
	const codeChallenge = searchParams.get('code_challenge') || '';
	const changePasswordCode = searchParams.get('change_password_code') || '';
	const mfaAuthAppRequired = searchParams.get('mfa_auth_app_required') || '';
	const userId = searchParams.get('user_id') || '';
	const mfaId = searchParams.get('mfa_id') || '';
	const nonce = searchParams.get('nonce') || '';

	const urlParams = buildOidcParams(searchParams, {
		mfa_auth_app_required: mfaAuthAppRequired,
		email,
		mfa_id: mfaId
	});

	let code = $state('');
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);
	let error = $state<string | null>(null);

	function showError(message: string) {
		error = message;
		isLoading = false;
		setTimeout(() => (error = null), 6000);
	}

	function getSessionCode() {
		return sessionStorage.getItem(`auth_session_${clientId}`) || '';
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
		const sessionCode = getSessionCode();
		if (!sessionCode) {
			showError('Your sign-in session expired. Please sign in again.');
			return;
		}

		const [verifyMfaData, verifyMfaErr] = await verifyAppMfaApi({
			clientId,
			sessionCode,
			code: result.data.code,
			mfaId: mfaId || undefined
		});

		if (verifyMfaErr) {
			console.error(verifyMfaErr);
			showError(verifyMfaErr.data?.message || 'An error occurred');
			return;
		}

		if (!verifyMfaData) {
			showError('An error occurred');
			return;
		}

		if (changePasswordCode && userId) {
			urlParams.set('session_code', verifyMfaData.sessionCode);
			urlParams.set('change_password_code', changePasswordCode);
			urlParams.set('user_id', userId);

			await goto(resolve(`/auth/[clientId]/update-password?${urlParams.toString()}`, { clientId }));
			return;
		}

		const [authorizeData, authorizeErr] = await authorizeApi({
			email: email.trim(),
			sessionCode: verifyMfaData.sessionCode,
			clientId,
			redirectUri,
			responseType,
			scope,
			codeChallengeMethod,
			codeChallenge,
			state: oidcState,
			mfaId,
			nonce: nonce || undefined
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
		toast.success('You have successfully signed in');
		sessionStorage.removeItem(`auth_session_${clientId}`);

		window.location.href = `${redirectUri}?code=${authorizeData.authorizationCode}&state=${oidcState}&redirect_uri=${redirectUri}&client_id=${clientId}`;
	}
</script>

<div class="grid gap-4">
	{#if error}
		<ErrorAlert message={error} title="An error occurred..." />
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
			Confirm Code
		</Button>
	</form>

	<MfaModal currentMethod={EMfaType.MfaApp} />
</div>

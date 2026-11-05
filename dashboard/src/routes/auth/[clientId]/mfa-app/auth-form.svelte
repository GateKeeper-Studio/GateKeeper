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
	import SessionExpiryGuard from '$lib/components/auth/session-expiry-guard.svelte';

	import { validate, buildOidcParams } from '$lib/utils/form';
	import { formSchema } from './schema';
	import type { ClientAuthData } from '$lib/services/auth/get-client-auth-data';
	import { authorizeApi } from '$lib/services/auth/authorize';
	import { verifyAppMfaApi } from '$lib/services/auth/verify-app-mfa';
	import { EMfaType } from '$lib/services/auth/login';
	import {
		SessionClock,
		clearSession,
		readSessionCode
	} from '$lib/state/auth-session-expiry.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let { client }: { client: ClientAuthData | null } = $props();

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

	// Past the password step the clock is our own AuthSession, whose deadlines were
	// stored alongside the session code at login. Rehydrating rather than restarting is
	// what keeps the countdown honest across the hop from the sign-in page.
	const clock = new SessionClock();
	clock.hydrateFromStorage(clientId);

	let errorTimeout: ReturnType<typeof setTimeout> | undefined;

	function showError(message: string) {
		error = message;
		isLoading = false;
		clearTimeout(errorTimeout);
		errorTimeout = setTimeout(() => (error = null), 6000);
	}

	$effect(() => () => clearTimeout(errorTimeout));

	function onExpire() {
		code = '';
		isLoading = false;
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		if (clock.phase === 'expired') return;

		const result = validate(formSchema, { code });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;
		// Read late, never cached: a refresh rotates the code out from under us.
		const sessionCode = readSessionCode(clientId);
		if (!sessionCode) {
			clock.expireNow();
			isLoading = false;
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
			// The server returns a single generic "invalid or expired session" (401) when the
			// session itself is gone — that is unrecoverable here, so hand it to the guard,
			// which offers a way back instead of an alert the user can't act on.
			if (verifyMfaErr.message === 'invalid or expired session') {
				clock.expireNow();
				return;
			}
			// "invalid MFA challenge" (also 401) is the recoverable case: a wrong or stale
			// 6-digit code. `ApiError` resolves the `{ error }` body into `.message` — note
			// `.data.message` is undefined for this body shape.
			showError(
				verifyMfaErr.message === 'invalid MFA challenge'
					? m.auth_mfa_app_invalid_code()
					: verifyMfaErr.message || m.auth_mfa_app_verify_error()
			);
			code = '';
			return;
		}

		if (!verifyMfaData) {
			showError(m.auth_generic_error());
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
			if (authorizeErr.status === 401) {
				clock.expireNow();
				return;
			}
			showError(authorizeErr.message || m.auth_generic_error());
			return;
		}

		if (!authorizeData) {
			showError(m.auth_generic_error());
			return;
		}

		isLoading = false;
		toast.success(m.signin_success());
		clearSession(clientId);

		window.location.href = `${redirectUri}?code=${authorizeData.authorizationCode}&state=${oidcState}&redirect_uri=${redirectUri}&client_id=${clientId}`;
	}
</script>

<div class="grid gap-4">
	<SessionExpiryGuard
		{clock}
		{clientId}
		mode="session"
		initiateLoginUri={client?.initiateLoginUri ?? null}
		{onExpire}
	/>

	{#if error}
		<ErrorAlert message={error} title={m.auth_mfa_app_error_title()} />
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
			{m.auth_mfa_app_submit()}
		</Button>
	</form>

	<MfaModal currentMethod={EMfaType.MfaApp} />
</div>

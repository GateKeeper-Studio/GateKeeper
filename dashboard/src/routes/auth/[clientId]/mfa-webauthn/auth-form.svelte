<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';
	import { startAuthentication } from '@simplewebauthn/browser';

	import { Button } from '$lib/components/ui/button/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MfaModal from '$lib/components/auth/mfa-modal.svelte';
	import SessionExpiryGuard from '$lib/components/auth/session-expiry-guard.svelte';

	import { buildOidcParams } from '$lib/utils/form';
	import type { ClientAuthData } from '$lib/services/auth/get-client-auth-data';
	import { authorizeApi } from '$lib/services/auth/authorize';
	import { startWebAuthnMfaApi } from '$lib/services/auth/start-webauthn-mfa';
	import { verifyWebAuthnMfaApi } from '$lib/services/auth/verify-webauthn-mfa';
	import { EMfaType } from '$lib/services/auth/login';
	import {
		SessionClock,
		clearSession,
		readSessionCode
	} from '$lib/state/auth-session-expiry.svelte';
	import { m } from '$lib/paraglide/messages.js';

	type StartAuthOptions = Parameters<typeof startAuthentication>[0]['optionsJSON'];

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
	let currentMfaId = $state(searchParams.get('mfa_id') || '');
	const nonce = searchParams.get('nonce') || '';
	// Passwordless login: the passkey is the only factor, not a second one.
	const passwordless = searchParams.get('passwordless') === '1';

	let isLoading = $state(false);
	let error = $state<string | null>(null);

	// Past the password step the clock is our own AuthSession, whose deadlines were
	// stored alongside the session code at login.
	const clock = new SessionClock();
	clock.hydrateFromStorage(clientId);

	function onExpire() {
		isLoading = false;
		error = null;
	}

	async function handleWebAuthn() {
		if (clock.phase === 'expired') return;

		isLoading = true;
		error = null;
		// Read late, never cached: a refresh rotates the code out from under us.
		const sessionCode = readSessionCode(clientId);
		if (!sessionCode) {
			clock.expireNow();
			isLoading = false;
			return;
		}

		// Check sessionStorage first, then fall back to URL search params
		// (the OAuth callback flow passes webauthn_options as a query param).
		let storedOptions = currentMfaId
			? sessionStorage.getItem(`webauthn_options_${currentMfaId}`)
			: null;

		// Only while we still hold the challenge id they belong to: after a failed
		// verify the id is cleared, and stale URL options would just fail again.
		if (!storedOptions && currentMfaId) {
			const urlOptions = searchParams.get('webauthn_options');
			if (urlOptions) {
				storedOptions = urlOptions;
				sessionStorage.setItem(`webauthn_options_${currentMfaId}`, urlOptions);
			}
		}

		if (!storedOptions) {
			const [startData, startErr] = await startWebAuthnMfaApi({ clientId, sessionCode });
			if (startErr || !startData) {
				if (startErr?.status === 401) {
					clock.expireNow();
					isLoading = false;
					return;
				}
				error = startErr?.message || m.auth_mfa_webauthn_start_error();
				isLoading = false;
				return;
			}

			currentMfaId = startData.mfaId;
			storedOptions = JSON.stringify(startData.options);
			sessionStorage.setItem(`webauthn_options_${currentMfaId}`, storedOptions);
		}

		let assertionOptions: StartAuthOptions;
		try {
			const parsed = JSON.parse(storedOptions);
			assertionOptions = (parsed?.publicKey ?? parsed) as StartAuthOptions;
		} catch (err) {
			console.error('Failed to parse WebAuthn options:', storedOptions, err);
			error = m.auth_mfa_webauthn_invalid_options();
			isLoading = false;
			return;
		}

		let assertionResponse: Awaited<ReturnType<typeof startAuthentication>>;
		try {
			assertionResponse = await startAuthentication({ optionsJSON: assertionOptions });
		} catch (err: unknown) {
			console.error('WebAuthn authentication error:', err);
			error = err instanceof Error ? err.message : m.auth_mfa_webauthn_cancelled();
			isLoading = false;
			return;
		}

		const [verifyData, verifyErr] = await verifyWebAuthnMfaApi({
			clientId,
			sessionCode,
			mfaId: currentMfaId,
			assertionData: assertionResponse
		});

		if (verifyErr) {
			console.error('WebAuthn verification error:', verifyErr);
			// The challenge behind these options may be spent (it dies after a few failed
			// attempts) — drop them so the next try asks the server for a fresh one.
			sessionStorage.removeItem(`webauthn_options_${currentMfaId}`);
			currentMfaId = '';
			// A dead session is unrecoverable here; hand it to the guard rather than showing
			// an error the user can only stare at.
			if (verifyErr.message === 'invalid or expired session') {
				clock.expireNow();
				isLoading = false;
				return;
			}
			error = verifyErr.message || m.auth_mfa_webauthn_verify_failed();
			isLoading = false;
			return;
		}

		if (!verifyData) {
			error = m.auth_mfa_webauthn_verification_error();
			isLoading = false;
			return;
		}

		sessionStorage.removeItem(`webauthn_options_${currentMfaId}`);

		const [authorizeData, authorizeErr] = await authorizeApi({
			email: email.trim(),
			sessionCode: verifyData.sessionCode,
			clientId,
			redirectUri,
			responseType,
			scope,
			codeChallengeMethod,
			codeChallenge,
			state: oidcState,
			nonce: nonce || undefined
		});

		if (authorizeErr) {
			if (authorizeErr.status === 401) {
				clock.expireNow();
				isLoading = false;
				return;
			}
			error = authorizeErr.message || m.auth_mfa_webauthn_authorization_failed();
			isLoading = false;
			return;
		}

		if (!authorizeData) {
			error = m.auth_mfa_webauthn_authorization_error();
			isLoading = false;
			return;
		}

		isLoading = false;
		toast.success(m.signin_success());
		clearSession(clientId);

		window.location.href = `${redirectUri}?code=${authorizeData.authorizationCode}&state=${oidcState}&redirect_uri=${redirectUri}&client_id=${clientId}`;
	}

	function handleSignInAgain() {
		const urlParams = buildOidcParams(searchParams);
		goto(resolve(`/auth/[clientId]/sign-in?${urlParams.toString()}`, { clientId }));
	}

	onMount(() => {
		handleWebAuthn();
	});
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
		<ErrorAlert message={error} title={m.auth_error_title()} />
	{/if}

	<div class="flex flex-col items-center gap-3">
		{#if isLoading}
			<div class="flex items-center gap-2 text-muted-foreground">
				<LoadingSpinner />
				<span>{m.auth_mfa_webauthn_waiting()}</span>
			</div>
		{:else}
			{#if !error}
				<p class="text-center text-sm text-muted-foreground">
					{passwordless
						? m.auth_mfa_webauthn_passwordless_hint()
						: m.auth_mfa_webauthn_prompt_hint()}
				</p>
			{/if}

			<Button type="button" disabled={isLoading} class="relative w-full" onclick={handleWebAuthn}>
				{#if isLoading}
					<LoadingSpinner class="absolute left-4" />
				{/if}
				{m.auth_mfa_webauthn_use_passkey()}
			</Button>

			<Button type="button" variant="ghost" class="w-full" onclick={handleSignInAgain}>
				{m.auth_mfa_webauthn_sign_in_again()}
			</Button>
		{/if}
	</div>

	<MfaModal currentMethod={EMfaType.MfaWebauthn} />
</div>

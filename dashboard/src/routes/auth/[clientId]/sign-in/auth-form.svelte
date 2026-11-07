<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import MailIcon from '@lucide/svelte/icons/mail';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import GithubLogo from '$lib/components/icons/github-logo.svelte';
	import GoogleLogo from '$lib/components/icons/google-logo.svelte';
	import SessionExpiryGuard from '$lib/components/auth/session-expiry-guard.svelte';

	import { validate, buildOidcParams } from '$lib/utils/form';
	import { formSchema, passkeySchema } from './schema';
	import type { ClientAuthData } from '$lib/services/auth/get-client-auth-data';
	import { EMfaType, loginApi } from '$lib/services/auth/login';
	import { requestMagicLinkApi } from '$lib/services/auth/magic-link';
	import { oauthProviderLoginApi } from '$lib/services/auth/oauth-provider-login';
	import { SessionClock, storeSession } from '$lib/state/auth-session-expiry.svelte';
	import { saveMagicLinkBinding } from '$lib/state/magic-link-binding';
	import { continueAfterLogin, type ContinueLoginOutcome } from '$lib/utils/continue-login';
	import { m } from '$lib/paraglide/messages.js';

	let { client }: { client: ClientAuthData | null } = $props();

	// This client signs in through its external providers only — the IdP refuses
	// /v1/auth/login for it, so rendering the credential form would be a dead end.
	const externalOnly = $derived(client?.externalLoginOnly ?? false);
	// External-only with nothing configured: the admin has left no way in at all.
	// The IdP guards against reaching this state, but a client configured before the
	// flag existed can still land here.
	const noSignInMethod = $derived(externalOnly && (client?.oauthProviders.length ?? 0) === 0);
	// Passwordless login: an account with no password signs in with its passkey alone.
	const passkeyEnabled = $derived(!externalOnly && (client?.passkeyLoginEnabled ?? false));
	// Magic link login: "email me a sign-in link".
	const magicLinkEnabled = $derived(!externalOnly && (client?.magicLinkLoginEnabled ?? false));

	const clientId = page.params.clientId as string;
	const searchParams = page.url.searchParams;

	const redirectUri = searchParams.get('redirect_uri') || '/';
	const responseType = searchParams.get('response_type') || '';
	const scope = searchParams.get('scope') || '';
	const oidcState = searchParams.get('state') || '';
	const codeChallenge = searchParams.get('code_challenge') || '';
	const codeChallengeMethod = searchParams.get('code_challenge_method') || '';
	const nonce = searchParams.get('nonce') || '';
	const mfaMethods = (searchParams.get('mfa_methods') || '')
		.split(',')
		.map((method) => method.trim())
		.filter((method): method is EMfaType =>
			[EMfaType.MfaWebauthn, EMfaType.MfaApp, EMfaType.MfaEmail].includes(method as EMfaType)
		);
	const mfaDefaultMethod = searchParams.get('mfa_default_method') || '';

	const urlParams = buildOidcParams(searchParams);

	// Prefilled when the RP bounced us back here to restart an expired request — the user
	// already typed this once and shouldn't have to again.
	let email = $state(searchParams.get('login_hint') ?? '');
	let password = $state('');
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);
	let error = $state<string | null>(null);

	// The clock on this page belongs to the relying party: its `state` and PKCE verifier
	// cookies die on a schedule we don't control and can't extend. The RP tells us when
	// via `request_expires_at`; if it doesn't (a third-party RP that hasn't adopted the
	// hint), the guard stays dormant and behaves exactly as before.
	const clock = new SessionClock();
	clock.adoptRequestExpiryHint(searchParams.get('request_expires_at'));

	let errorTimeout: ReturnType<typeof setTimeout> | undefined;

	function showError(message: string) {
		error = message;
		isLoading = false;
		clearTimeout(errorTimeout);
		errorTimeout = setTimeout(() => (error = null), 6000);
	}

	$effect(() => () => clearTimeout(errorTimeout));

	// Expiry clears whatever the user has typed. Leaving a password sitting in a form
	// that is now guaranteed to fail helps nobody, and it shouldn't outlive the flow it
	// was typed for.
	function onExpire() {
		password = '';
		isLoading = false;
	}

	// The frozen authorization request every login attempt carries. No `password`
	// means passwordless login.
	function buildLoginRequest(loginEmail: string, loginPassword?: string) {
		return {
			email: loginEmail,
			password: loginPassword,
			clientId,
			redirectUri,
			responseType,
			scope,
			codeChallengeMethod,
			codeChallenge,
			state: oidcState,
			nonce: nonce || undefined,
			mfaMethods: mfaMethods.length ? mfaMethods : undefined,
			mfaDefaultMethod: [EMfaType.MfaWebauthn, EMfaType.MfaApp, EMfaType.MfaEmail].includes(
				mfaDefaultMethod as EMfaType
			)
				? (mfaDefaultMethod as EMfaType)
				: undefined
		};
	}

	// Hand the WebAuthn options to the passkey page (it reads them back by mfa id).
	async function goToWebauthn(mfaId: string, webAuthnOptions: unknown) {
		sessionStorage.setItem(`webauthn_options_${mfaId}`, JSON.stringify(webAuthnOptions));
		await goto(
			resolve(`/auth/[clientId]/mfa-webauthn?${urlParams.toString()}&mfa_id=${mfaId}`, {
				clientId
			})
		);
	}

	// Passwordless login: the email alone. The IdP answers with a WebAuthn challenge
	// when the account has no password and a passkey; the passkey page then signs
	// the user in exactly as it does for a second factor.
	async function onPasskeySignIn() {
		if (clock.phase === 'expired') return;

		const result = validate(passkeySchema, { email });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const trimmedEmail = result.data.email.trim();
		const [loginData, loginErr] = await loginApi(buildLoginRequest(trimmedEmail));

		if (loginErr) {
			console.error(loginErr);
			// Every account that cannot sign in with a passkey alone (unknown, has a
			// password, no passkey) gets the same 401 a wrong password does — say what
			// to try instead of echoing "email or password is incorrect".
			showError(
				loginErr.status === 401
					? m.signin_passkey_unavailable()
					: loginErr.message || m.auth_generic_error()
			);
			return;
		}

		if (!loginData || loginData.mfaType !== EMfaType.MfaWebauthn || !loginData.mfaId) {
			showError(m.auth_generic_error());
			return;
		}

		isLoading = false;
		storeSession(
			clientId,
			loginData.sessionCode,
			loginData.sessionExpiresAt,
			loginData.sessionAbsoluteExpiresAt
		);

		urlParams.set('email', trimmedEmail);
		urlParams.set('passwordless', '1');
		urlParams.set('available_mfa_methods', EMfaType.MfaWebauthn);
		await goToWebauthn(loginData.mfaId, loginData.webAuthnOptions);
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		// The authorization request this page was built from is already dead — the server
		// would mint a code the RP will refuse. Refusing here is the whole point: it is
		// what stops the password being spent on a doomed flow.
		if (clock.phase === 'expired') return;

		const result = validate(formSchema, { email, password });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const trimmedEmail = result.data.email.trim();

		const [loginData, loginErr] = await loginApi(
			buildLoginRequest(trimmedEmail, result.data.password.trim())
		);

		if (loginErr && loginErr.data?.title === 'E-mail not confirmed') {
			window.location.href = `/auth/${clientId}/confirm-email?${urlParams.toString()}&email=${encodeURIComponent(
				trimmedEmail
			)}&trying_to_sign_in=true`;
			return;
		}

		if (loginErr) {
			console.error(loginErr);
			// `ApiError.message` already picks the right field out of whichever body shape
			// came back (`{error}`, `{message}`, `{errors:[{message}]}`). Reading
			// `.data.error` directly only handles one of the three.
			showError(loginErr.message || m.auth_generic_error());
			return;
		}

		if (!loginData) {
			showError(m.auth_generic_error());
			return;
		}

		toast.success(m.signin_success());

		const outcome = await continueAfterLogin(loginData, {
			clientId,
			email: trimmedEmail,
			oidcParams: urlParams
		});
		handleOutcome(outcome);
	}

	// What `continueAfterLogin` could not do by itself: tell the user.
	function handleOutcome(outcome: ContinueLoginOutcome) {
		if (outcome.kind !== 'navigated') isLoading = false;
		if (outcome.kind === 'enrollment-required') {
			showError(m.signin_mfa_enrollment_required());
		} else if (outcome.kind === 'session-expired') {
			// The server says the session is gone: show the recovery dialog rather than a
			// red alert the user can do nothing with.
			clock.expireNow();
		} else if (outcome.kind === 'error') {
			showError(outcome.message || m.auth_generic_error());
		}
	}

	// Magic link login: the email alone. The IdP answers the same for every address and
	// mails a link only to an eligible account. The session code it returns is kept in
	// this browser — the link works only together with it — and the waiting page takes
	// over from there.
	async function onMagicLinkSignIn() {
		if (clock.phase === 'expired') return;

		const result = validate(passkeySchema, { email });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const trimmedEmail = result.data.email.trim();
		const [data, err] = await requestMagicLinkApi({
			email: trimmedEmail,
			clientId,
			redirectUri,
			responseType,
			scope,
			state: oidcState,
			codeChallenge,
			codeChallengeMethod,
			nonce: nonce || undefined
		});

		if (err || !data) {
			console.error(err);
			showError(err?.message || m.auth_generic_error());
			return;
		}

		saveMagicLinkBinding(clientId, {
			sessionCode: data.sessionCode,
			email: trimmedEmail,
			oidcSearch: urlParams.toString(),
			expiresAt: data.sessionExpiresAt
		});

		isLoading = false;
		const sentSearch = `${urlParams.toString()}&email=${encodeURIComponent(trimmedEmail)}`;
		await goto(resolve(`/auth/[clientId]/magic-link-sent?${sentSearch}`, { clientId }));
	}

	async function handleOAuthLogin(provider: ClientAuthData['oauthProviders'][number]) {
		const [data, err] = await oauthProviderLoginApi({
			providerName: provider.name,
			oauthProviderId: provider.id,
			clientCodeChallenge: codeChallenge,
			clientCodeChallengeMethod: codeChallengeMethod,
			clientRedirectUri: redirectUri,
			clientState: oidcState,
			clientResponseType: responseType,
			clientScope: scope,
			clientNonce: nonce || undefined
		});

		if (err || !data) {
			toast.error(err?.message || m.signin_oauth_start_error());
			return;
		}

		window.location.href = data.url;
	}

	function capitalize(value: string) {
		return value.charAt(0).toUpperCase() + value.slice(1);
	}
</script>

<div class="grid gap-4">
	<SessionExpiryGuard
		{clock}
		{clientId}
		mode="pre-auth"
		initiateLoginUri={client?.initiateLoginUri ?? null}
		loginHint={email.trim()}
		{onExpire}
	/>

	{#if error}
		<ErrorAlert message={error} title={m.auth_error_title()} />
	{/if}

	{#if noSignInMethod}
		<ErrorAlert message={m.signin_no_method_configured()} title={m.auth_error_title()} />
	{/if}

	{#if !externalOnly}
		<form onsubmit={onSubmit} class="space-y-3">
			<div class="space-y-2">
				<Label for="email">{m.auth_email_label()}</Label>
				<Input
					id="email"
					type="email"
					autocomplete="email"
					placeholder={m.auth_email_placeholder()}
					bind:value={email}
				/>
				{#if errors.email}
					<p class="text-sm font-medium text-destructive">{errors.email}</p>
				{/if}
			</div>

			<div class="space-y-2">
				<Label for="password">{m.auth_password_label()}</Label>
				<Input
					id="password"
					type="password"
					autocomplete="current-password"
					placeholder="********"
					bind:value={password}
				/>
				{#if errors.password}
					<p class="text-sm font-medium text-destructive">{errors.password}</p>
				{/if}
			</div>

			{#if client?.canSelfSignUp || client?.canSelfForgotPass}
				<div class="flex items-center justify-between gap-2">
					{#if client?.canSelfForgotPass}
						<a
							href={resolve(`/auth/[clientId]/forgot-password?${urlParams.toString()}`, {
								clientId
							})}
							class="text-md mx-auto text-center hover:underline"
						>
							{m.signin_forgot_password()}
						</a>
					{/if}

					{#if client?.canSelfSignUp}
						<a
							href={resolve(`/auth/[clientId]/sign-up?${urlParams.toString()}`, {
								clientId
							})}
							class="text-md mx-auto text-center font-semibold hover:underline"
						>
							{m.signin_create_account()}
						</a>
					{/if}
				</div>
			{/if}

			<Button
				type="submit"
				disabled={isLoading || clock.phase === 'expired'}
				class="relative w-full"
			>
				{#if isLoading}
					<LoadingSpinner class="absolute left-4" />
				{/if}
				{m.signin_submit()}
			</Button>

			{#if passkeyEnabled}
				<Button
					type="button"
					variant="outline"
					class="flex w-full items-center justify-center gap-2"
					disabled={isLoading || clock.phase === 'expired'}
					onclick={onPasskeySignIn}
				>
					<KeyRoundIcon class="size-4" />
					{m.signin_with_passkey()}
				</Button>
			{/if}

			{#if magicLinkEnabled}
				<Button
					type="button"
					variant="outline"
					class="flex w-full items-center justify-center gap-2"
					disabled={isLoading || clock.phase === 'expired'}
					onclick={onMagicLinkSignIn}
				>
					<MailIcon class="size-4" />
					{m.signin_with_magic_link()}
				</Button>
			{/if}
		</form>
	{/if}

	{#if client && client.oauthProviders.length > 0}
		<!-- The divider separates the credential form from the provider buttons. With
		     no form above it there is nothing to separate, so it is dropped. -->
		{#if !externalOnly}
			<div class="relative">
				<div class="absolute inset-0 flex items-center">
					<span class="w-full border-t"></span>
				</div>

				<div class="relative flex justify-center text-xs uppercase">
					<span class="bg-background px-2 text-muted-foreground">
						{m.auth_or_continue_with()}
					</span>
				</div>
			</div>
		{/if}

		{#each client.oauthProviders as provider (provider.id)}
			<div class="flex flex-col gap-1">
				<Button
					variant="outline"
					type="button"
					class="flex items-center justify-center gap-2"
					disabled={isLoading || clock.phase === 'expired'}
					onclick={() => handleOAuthLogin(provider)}
					title={m.signin_with_provider({ provider: capitalize(provider.name) })}
				>
					{#if provider.name === 'github'}
						<GithubLogo size={24} />
					{/if}
					{#if provider.name === 'google'}
						<GoogleLogo size={24} />
					{/if}

					{capitalize(provider.name)}
				</Button>
			</div>
		{/each}
	{/if}
</div>

<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import GithubLogo from '$lib/components/icons/github-logo.svelte';
	import GoogleLogo from '$lib/components/icons/google-logo.svelte';

	import { validate, buildOidcParams } from '$lib/utils/form';
	import { formSchema } from './schema';
	import type { ClientAuthData } from '$lib/services/auth/get-client-auth-data';
	import { EMfaType, loginApi } from '$lib/services/auth/login';
	import { authorizeApi } from '$lib/services/auth/authorize';
	import { oauthProviderLoginApi } from '$lib/services/auth/oauth-provider-login';

	let { client }: { client: ClientAuthData | null } = $props();

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

	let email = $state('');
	let password = $state('');
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

		const result = validate(formSchema, { email, password });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const trimmedEmail = result.data.email.trim();

		const [loginData, loginErr] = await loginApi({
			email: trimmedEmail,
			password: result.data.password.trim(),
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
		});

		if (loginErr && loginErr.data?.title === 'E-mail not confirmed') {
			window.location.href = `/auth/${clientId}/confirm-email?${urlParams.toString()}&email=${encodeURIComponent(
				trimmedEmail
			)}&trying_to_sign_in=true`;
			return;
		}

		if (loginErr) {
			console.error(loginErr);
			console.log('Login error data:', loginErr.data);
			showError(loginErr.data?.error || 'An error occurred');
			return;
		}

		if (!loginData) {
			showError('An error occurred');
			return;
		}

		isLoading = false;
		toast.success('You have successfully signed in');

		sessionStorage.setItem(`auth_session_${clientId}`, loginData.sessionCode);

		urlParams.set('mfa_auth_app_required', 'true');
		urlParams.set('email', trimmedEmail);

		if (loginData.changePasswordCode) {
			urlParams.set('change_password_code', loginData.changePasswordCode);
		}
		if (loginData.userId) {
			urlParams.set('user_id', loginData.userId);
		}
		if (loginData.availableMfaMethods?.length) {
			urlParams.set('available_mfa_methods', loginData.availableMfaMethods.join(','));
		}

		const mfaSuffix = loginData.mfaId ? `&mfa_id=${loginData.mfaId}` : '';

		if (loginData.mfaEnrollmentRequired) {
			showError('This application requires MFA enrollment before sign-in can continue.');
			return;
		}

		if (loginData.mfaType === EMfaType.MfaEmail) {
			await goto(
				resolve(`/auth/[clientId]/mfa-mail?${urlParams.toString()}${mfaSuffix}`, {
					clientId
				})
			);
			return;
		}

		if (loginData.mfaType === EMfaType.MfaApp) {
			await goto(
				resolve(`/auth/[clientId]/mfa-app?${urlParams.toString()}${mfaSuffix}`, {
					clientId
				})
			);
			return;
		}

		if (loginData.mfaType === EMfaType.MfaWebauthn && loginData.mfaId) {
			sessionStorage.setItem(
				`webauthn_options_${loginData.mfaId}`,
				JSON.stringify(loginData.webAuthnOptions)
			);
			await goto(
				resolve(`/auth/[clientId]/mfa-webauthn?${urlParams.toString()}&mfa_id=${loginData.mfaId}`, {
					clientId
				})
			);
			return;
		}

		if (loginData.changePasswordCode && loginData.userId) {
			urlParams.set('session_code', loginData.sessionCode);
			await goto(
				resolve(`/auth/[clientId]/update-password?${urlParams.toString()}${mfaSuffix}`, {
					clientId
				})
			);
			return;
		}

		isLoading = true;

		const [authorizeData, authorizeErr] = await authorizeApi({
			email: trimmedEmail,
			sessionCode: loginData.sessionCode,
			clientId,
			redirectUri,
			responseType,
			scope,
			codeChallengeMethod,
			codeChallenge,
			state: oidcState,
			nonce: nonce || undefined,
			mfaId: loginData.mfaId
		});

		if (authorizeErr) {
			console.error(authorizeErr);
			showError(authorizeErr.data?.error || 'An error occurred');
			return;
		}

		if (!authorizeData) {
			showError('An error occurred');
			return;
		}

		const params = new URLSearchParams({
			code: authorizeData.authorizationCode,
			state: oidcState,
			redirect_uri: redirectUri,
			client_id: clientId
		});

		window.location.href = `${redirectUri}?${params.toString()}`;
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
			toast.error(err?.data?.error || 'Could not start OAuth login');
			return;
		}

		window.location.href = data.url;
	}

	function capitalize(value: string) {
		return value.charAt(0).toUpperCase() + value.slice(1);
	}
</script>

<div class="grid gap-4">
	{#if error}
		<ErrorAlert message={error} title="An error occurred..." />
	{/if}

	<form onsubmit={onSubmit} class="space-y-3">
		<div class="space-y-2">
			<Label for="email">E-mail</Label>
			<Input
				id="email"
				type="email"
				autocomplete="email"
				placeholder="example@email.com"
				bind:value={email}
			/>
			{#if errors.email}
				<p class="text-sm font-medium text-destructive">{errors.email}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label for="password">Password</Label>
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
						Forgot password?
					</a>
				{/if}

				{#if client?.canSelfSignUp}
					<a
						href={resolve(`/auth/[clientId]/sign-up?${urlParams.toString()}`, {
							clientId
						})}
						class="text-md mx-auto text-center font-semibold hover:underline"
					>
						Create an account
					</a>
				{/if}
			</div>
		{/if}

		<Button type="submit" disabled={isLoading} class="relative w-full">
			{#if isLoading}
				<LoadingSpinner class="absolute left-4" />
			{/if}
			Sign In with Email
		</Button>
	</form>

	{#if client && client.oauthProviders.length > 0}
		<div class="relative">
			<div class="absolute inset-0 flex items-center">
				<span class="w-full border-t"></span>
			</div>

			<div class="relative flex justify-center text-xs uppercase">
				<span class="bg-background px-2 text-muted-foreground"> Or continue with </span>
			</div>
		</div>

		{#each client.oauthProviders as provider (provider.id)}
			<div class="flex flex-col gap-1">
				<Button
					variant="outline"
					type="button"
					class="flex items-center justify-center gap-2"
					disabled={isLoading}
					onclick={() => handleOAuthLogin(provider)}
					title={`Sign in with ${capitalize(provider.name)}`}
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

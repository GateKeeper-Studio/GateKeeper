<script lang="ts">
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import SessionExpiryGuard from '$lib/components/auth/session-expiry-guard.svelte';

	import { validate } from '$lib/utils/form';
	import { formSchema } from './schema';
	import type { ClientAuthData } from '$lib/services/auth/get-client-auth-data';
	import { changePasswordApi } from '$lib/services/auth/change-password';
	import { authorizeApi } from '$lib/services/auth/authorize';
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
	const urlSessionCode = searchParams.get('session_code') || '';
	const changePasswordCode = searchParams.get('change_password_code') || '';
	const userId = searchParams.get('user_id') || '';

	let password = $state('');
	let confirmPassword = $state('');
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);
	let error = $state<string | null>(null);

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
		password = '';
		confirmPassword = '';
		isLoading = false;
	}

	/**
	 * sessionStorage wins over the `session_code` in the URL: refreshing the session
	 * rotates the code, and the URL — baked when this page was navigated to — would still
	 * carry the dead one. The query param stays as a fallback for a cold landing where
	 * storage was never populated.
	 */
	function currentSessionCode() {
		return readSessionCode(clientId) || urlSessionCode;
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		if (clock.phase === 'expired') return;

		const result = validate(formSchema, { password, confirmPassword });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};

		if (!clientId) {
			toast.error(m.auth_update_password_client_id_required());
			return;
		}
		if (!changePasswordCode) {
			toast.error(m.auth_update_password_change_code_required());
			return;
		}
		if (!userId) {
			toast.error(m.auth_update_password_user_id_required());
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
			toast.error(err.message);
			isLoading = false;
			return;
		}

		const [authorizeData, authorizeErr] = await authorizeApi({
			email: email.trim(),
			sessionCode: currentSessionCode(),
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
		toast.success(m.auth_update_password_success());
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
		<ErrorAlert message={error} title={m.auth_error_title()} />
	{/if}

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
			<Label for="confirmPassword">{m.auth_update_password_confirm_password_label()}</Label>
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
			{m.auth_update_password_submit()}
		</Button>
	</form>
</div>

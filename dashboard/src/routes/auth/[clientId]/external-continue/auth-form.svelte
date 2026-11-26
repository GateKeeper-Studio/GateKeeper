<!--
  Where a social sign-in (Google, GitHub) lands when a second factor is due.

  The IdP's callback did not issue a code: it opened an MFA-pending session and sent
  the browser here with `#payload=<base64url JSON>`: the same answer a password
  sign-in gets (`login`), the account's email, and the relying party's authorization
  request (`oidc`). From there the sign-in continues exactly as after a password.

  Security: the fragment can be written by anyone who makes a link. Nothing in it is
  trusted for the outcome: the session code only works if the IdP issued it, the second
  factor is still checked by the IdP, and the final redirect goes to the address the IdP
  kept in the session (`redirectToClient`), not to the one in the payload.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { replaceState } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';

	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import type { LoginResponse } from '$lib/services/auth/login';
	import { continueAfterLogin } from '$lib/utils/continue-login';
	import { m } from '$lib/paraglide/messages.js';

	type Continuation = {
		email: string;
		login: LoginResponse;
		oidc: Record<string, string>;
	};

	const clientId = page.params.clientId as string;

	let error = $state<string | null>(null);
	// The OIDC query string, to start over from the sign-in page.
	let retrySearch = $state<string | null>(null);

	function readContinuation(): Continuation | null {
		try {
			const payload = new URLSearchParams(window.location.hash.slice(1)).get('payload');
			if (!payload) return null;
			const base64 = payload.replace(/-/g, '+').replace(/_/g, '/');
			const bytes = Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
			const parsed = JSON.parse(new TextDecoder().decode(bytes)) as Partial<Continuation>;
			if (
				typeof parsed.email !== 'string' ||
				typeof parsed.login?.sessionCode !== 'string' ||
				typeof parsed.oidc !== 'object' ||
				parsed.oidc === null
			) {
				return null;
			}
			return parsed as Continuation;
		} catch {
			return null;
		}
	}

	// Drop the session code from the address bar and the history entry. Deferred a tick:
	// the router refuses shallow navigation during the very first hydration pass.
	function forgetFragment() {
		setTimeout(() => {
			try {
				replaceState(page.url.pathname, {});
			} catch (err) {
				console.error(err);
			}
		}, 0);
	}

	onMount(() => {
		const continuation = readContinuation();
		forgetFragment();
		void proceed(continuation);
	});

	async function proceed(continuation: Continuation | null) {
		if (!continuation) {
			error = m.magic_link_invalid();
			return;
		}

		const oidcParams = new URLSearchParams(
			Object.entries(continuation.oidc).filter(
				(entry): entry is [string, string] => typeof entry[1] === 'string'
			)
		);
		retrySearch = oidcParams.toString();

		try {
			const outcome = await continueAfterLogin(continuation.login, {
				clientId,
				email: continuation.email,
				oidcParams
			});
			if (outcome.kind === 'navigated') return;
			if (outcome.kind === 'enrollment-required') {
				error = m.signin_mfa_enrollment_required();
			} else if (outcome.kind === 'session-expired') {
				error = m.magic_link_invalid();
			} else {
				error = outcome.message || m.auth_generic_error();
			}
		} catch (err) {
			console.error(err);
			error = m.auth_generic_error();
		}
	}
</script>

<div class="grid gap-4">
	{#if error === null}
		<div class="flex items-center justify-center gap-2 text-sm text-muted-foreground">
			<LoadingSpinner />
			{m.magic_link_signing_in()}
		</div>
	{:else}
		<ErrorAlert message={error} title={m.auth_error_title()} />
		{#if retrySearch !== null}
			<a
				href={resolve(`/auth/[clientId]/sign-in?${retrySearch}`, { clientId })}
				class="text-center text-sm font-semibold hover:underline"
			>
				{m.magic_link_sent_back()}
			</a>
		{/if}
	{/if}
</div>

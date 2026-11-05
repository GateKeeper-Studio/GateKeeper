<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { toast } from 'svelte-sonner';

	import ClockAlertIcon from '@lucide/svelte/icons/clock-alert';
	import ShieldXIcon from '@lucide/svelte/icons/shield-x';

	import * as Alert from '$lib/components/ui/alert/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import {
		SessionClock,
		clearSession,
		readSessionCode,
		storeSession
	} from '$lib/state/auth-session-expiry.svelte';
	import { refreshSessionApi } from '$lib/services/auth/refresh-session';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Warns the user before their sign-in dies, and gives them a way out when it does.
	 *
	 * Which clock is running depends on where we are in the flow, and the two are not
	 * recoverable the same way:
	 *
	 *   `pre-auth`  — the sign-in page. The clock belongs to the *relying party* (its
	 *                 OAuth state and PKCE verifier). We cannot extend it: the
	 *                 code_verifier lives in the RP's cookie and we have never seen it.
	 *                 Recovery is to ask the RP to start a fresh authorization request.
	 *                 Without this guard, a user who idles here types their password
	 *                 successfully and only *then* hits a dead-state error at the RP —
	 *                 the password spent for nothing.
	 *
	 *   `session`   — the MFA / forced-password-change screens. The clock is our own
	 *                 AuthSession, so "keep me signed in" genuinely extends it (up to a
	 *                 server-enforced ceiling).
	 *
	 * Design rule: stay quiet until it matters. A ticking countdown next to a password
	 * field rushes people through credential entry, so nothing renders until the warning
	 * threshold. And nothing here ever auto-extends or auto-navigates — an idle timeout
	 * that renews itself in the background is not an idle timeout, and a page that
	 * redirects itself steals the user's decision.
	 */
	let {
		clock,
		clientId,
		mode,
		initiateLoginUri = null,
		loginHint = '',
		onExpire
	}: {
		clock: SessionClock;
		clientId: string;
		mode: 'pre-auth' | 'session';
		initiateLoginUri?: string | null;
		/** Email the user already typed, carried across a restart so they don't retype it. */
		loginHint?: string;
		/** Called once when the clock dies — clear the session code, wipe the password. */
		onExpire?: () => void;
	} = $props();

	let isRefreshing = $state(false);
	// Deliberately NOT `$state`: it only guards the effect below, and making it reactive
	// would have the effect re-trigger on its own write.
	let hasExpired = false;

	// The clock is hydrated from sessionStorage, which does not exist on the server: SSR
	// always sees a dormant clock, while the client may already know the session is nearly
	// dead. Rendering nothing until after mount keeps the two from disagreeing.
	let mounted = $state(false);
	$effect(() => {
		mounted = true;
	});

	// Fire `onExpire` exactly once, whether the timer ran out or a 401 forced the clock
	// dead. Guarded because `phase` is derived and re-evaluates on every tick.
	$effect(() => {
		if (clock.phase === 'expired' && !hasExpired) {
			hasExpired = true;
			clearSession(clientId);
			onExpire?.();
		}
	});

	// The one interval driving the countdown, torn down with the component.
	$effect(() => clock.start());

	/**
	 * Put the user back on a sign-in they can actually complete.
	 *
	 * What "recovery" means depends on which clock died, and the difference is not
	 * cosmetic — get it wrong and the button looks like it works while changing nothing:
	 *
	 *   * Going to the RP's `initiateLoginUri` is always correct. It mints a brand-new
	 *     state/nonce/PKCE and bounces back with a live request. We prefer it whenever a
	 *     client has registered one. That URI comes from the server-side client
	 *     registration and never from a query parameter — which is exactly what stops
	 *     "restart my sign-in" from being an open redirect.
	 *
	 *   * Without one, reloading our own sign-in page only helps in `session` mode. There,
	 *     the RP's state/PKCE cookies are still alive (they outlive our AuthSession); the
	 *     current URL's OIDC params are still good, so carrying them over gives the user a
	 *     working form. In `pre-auth` mode those very params are what expired — reloading
	 *     with them would hand the user an identically dead page. That case has no local
	 *     recovery, `canRecover` is false, and we tell them to go back to the app instead
	 *     of offering a button that quietly does nothing.
	 */
	function restartSignIn() {
		if (initiateLoginUri) {
			// Built through the URL API rather than string concatenation: a registered
			// initiate_login_uri may legitimately already carry a query string.
			const target = new URL(initiateLoginUri);
			if (loginHint) target.searchParams.set('login_hint', loginHint);
			window.location.href = target.toString();
			return;
		}

		if (mode === 'session') {
			const params = new URLSearchParams(page.url.search);
			// Session-scoped junk from the MFA hop; the OIDC params are what matter.
			for (const key of ['session_code', 'change_password_code', 'mfa_id', 'webauthn_options']) {
				params.delete(key);
			}
			if (loginHint) params.set('login_hint', loginHint);
			window.location.href = resolve(`/auth/[clientId]/sign-in?${params.toString()}`, {
				clientId
			});
		}
	}

	async function keepAlive() {
		const sessionCode = readSessionCode(clientId);
		if (!sessionCode) {
			clock.expireNow();
			return;
		}

		isRefreshing = true;
		const [data, err] = await refreshSessionApi({ sessionCode, clientId });
		isRefreshing = false;

		if (err || !data) {
			// The server is the source of truth. If it says the session is gone, it is
			// gone — whatever our local clock believed.
			clock.expireNow();
			return;
		}

		// The code rotated: the one we just sent is already dead. Persist the replacement
		// before anything else can read it.
		storeSession(clientId, data.sessionCode, data.sessionExpiresAt, data.sessionAbsoluteExpiresAt);
		clock.set(data.sessionExpiresAt, data.sessionAbsoluteExpiresAt);
		toast.success('You have more time to finish signing in');
	}

	// In `session` mode we can always rebuild a working sign-in from the still-live OIDC
	// params. In `pre-auth` mode those params are the thing that died, so only the RP can
	// help — and if it hasn't registered an initiate_login_uri, nobody can.
	const canRecover = $derived(mode === 'session' || initiateLoginUri !== null);
</script>

<!--
	The visible m:ss ticks every second, so it is hidden from assistive tech; the live
	region below carries a coarse phrase that only changes a few times instead.
-->
{#if mounted && clock.phase === 'warning'}
	<Alert.Root class="border-amber-500/40 bg-amber-500/10">
		<ClockAlertIcon class="h-4 w-4 text-amber-600 dark:text-amber-500" />
		<Alert.Title class="font-bold">
			{#if mode === 'session'}
				Still there?
			{:else}
				This sign-in page is about to expire
			{/if}
		</Alert.Title>
		<Alert.Description>
			{#if mode === 'session' && clock.canExtend}
				{m.session_expiry_alert_session_warning({ time: clock.formatted })}
			{:else if mode === 'session'}
				{m.session_expiry_alert_session_final({ time: clock.formatted })}
			{:else if canRecover}
				{m.session_expiry_alert_pre_auth_recoverable({ time: clock.formatted })}
			{:else}
				{m.session_expiry_alert_pre_auth_final({ time: clock.formatted })}
			{/if}
		</Alert.Description>

		{#if mode === 'session' && clock.canExtend}
			<Alert.Action>
				<Button size="sm" variant="outline" disabled={isRefreshing} onclick={keepAlive}>
					{#if isRefreshing}
						<LoadingSpinner class="mr-1" />
					{/if}
					{m.session_expiry_keep_signed_in()}
				</Button>
			</Alert.Action>
		{:else if mode === 'pre-auth' && canRecover}
			<Alert.Action>
				<Button size="sm" variant="outline" onclick={restartSignIn}>{m.session_expiry_refresh()}</Button>
			</Alert.Action>
		{/if}
	</Alert.Root>
{/if}

{#if mounted && clock.phase !== 'dormant' && clock.phase !== 'ok'}
	<div class="sr-only" role="status" aria-live="polite">{clock.announcement}</div>
{/if}

<!--
	Expiry is a hard stop, not a nudge: the form behind this is now guaranteed to fail, so
	the dialog is non-dismissible. Letting the user click it away would only let them
	spend a password on a flow that is already dead — the exact failure this replaces.
-->
<Dialog.Root open={mounted && clock.phase === 'expired'}>
	<Dialog.Content
		showCloseButton={false}
		escapeKeydownBehavior="ignore"
		interactOutsideBehavior="ignore"
		aria-live="assertive"
	>
		<Dialog.Header>
			<div class="mb-2 flex justify-center">
				<div class="rounded-full bg-destructive/10 p-3">
					<ShieldXIcon class="h-6 w-6 text-destructive" />
				</div>
			</div>
			<Dialog.Title class="text-center">{m.session_expiry_dialog_title()}</Dialog.Title>
			<Dialog.Description class="text-center">
				{#if mode === 'session'}
					{m.session_expiry_dialog_session_description()}
				{:else if canRecover}
					{m.session_expiry_dialog_recoverable_description()}
				{:else}
					{m.session_expiry_dialog_final_description()}
				{/if}
			</Dialog.Description>
		</Dialog.Header>

		{#if canRecover}
			<Button class="w-full" onclick={restartSignIn}>{m.session_expiry_start_again()}</Button>
		{/if}
	</Dialog.Content>
</Dialog.Root>

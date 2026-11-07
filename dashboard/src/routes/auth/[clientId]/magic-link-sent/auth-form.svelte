<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import MailCheckIcon from '@lucide/svelte/icons/mail-check';

	import { Button } from '$lib/components/ui/button/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { requestMagicLinkApi } from '$lib/services/auth/magic-link';
	import {
		clearMagicLinkBinding,
		loadMagicLinkBinding,
		openMagicLinkChannel,
		saveMagicLinkBinding,
		type MagicLinkMessage
	} from '$lib/state/magic-link-binding';
	import { buildOidcParams } from '$lib/utils/form';
	import { continueAfterLogin } from '$lib/utils/continue-login';
	import { m } from '$lib/paraglide/messages.js';

	// Matches the IdP's per-account cooldown: a sooner request would be silently dropped.
	const RESEND_COOLDOWN_S = 60;

	const clientId = page.params.clientId as string;
	const email = page.url.searchParams.get('email') ?? '';
	const oidcParams = buildOidcParams(page.url.searchParams);

	// The code of the session our link belongs to. Tells our hand-off apart from that
	// of another tab that is also waiting for a link.
	let requestCode = '';
	let cooldown = $state(RESEND_COOLDOWN_S);
	let isResending = $state(false);
	let isContinuing = $state(false);
	let resent = $state(false);
	let error = $state<string | null>(null);

	onMount(() => {
		requestCode = loadMagicLinkBinding(clientId)?.sessionCode ?? '';

		const timer = setInterval(() => {
			if (cooldown > 0) cooldown -= 1;
		}, 1000);

		// The link opens in another tab, which redeems it and hands the result back here:
		// the sign-in must finish in THIS tab, where the relying party keeps its PKCE
		// verifier and state.
		const channel = openMagicLinkChannel(clientId);
		if (channel) {
			channel.onmessage = (event: MessageEvent<MagicLinkMessage>) => onMessage(channel, event.data);
		}

		return () => {
			clearInterval(timer);
			channel?.close();
		};
	});

	async function onMessage(channel: BroadcastChannel, message: MagicLinkMessage) {
		if (message.type !== 'redeemed' || isContinuing) return;
		if (!requestCode || message.requestCode !== requestCode) return;

		channel.postMessage({ type: 'ack', requestCode } satisfies MagicLinkMessage);
		isContinuing = true;
		error = null;
		clearMagicLinkBinding(clientId);

		const outcome = await continueAfterLogin(message.response, {
			clientId,
			email: message.email,
			oidcParams
		});
		if (outcome.kind === 'navigated') return;

		isContinuing = false;
		if (outcome.kind === 'enrollment-required') {
			error = m.signin_mfa_enrollment_required();
		} else if (outcome.kind === 'session-expired') {
			error = m.magic_link_invalid();
		} else {
			error = outcome.message || m.auth_generic_error();
		}
	}

	async function onResend() {
		isResending = true;
		error = null;
		resent = false;

		const [data, err] = await requestMagicLinkApi({
			email,
			clientId,
			redirectUri: oidcParams.get('redirect_uri') || '/',
			responseType: oidcParams.get('response_type') || '',
			scope: oidcParams.get('scope') || '',
			state: oidcParams.get('state') || '',
			codeChallenge: oidcParams.get('code_challenge') || '',
			codeChallengeMethod: oidcParams.get('code_challenge_method') || '',
			nonce: oidcParams.get('nonce') || undefined
		});
		isResending = false;

		if (err || !data) {
			console.error(err);
			error = err?.message || m.auth_generic_error();
			return;
		}

		// Only the newest link works: it is tied to this new session.
		saveMagicLinkBinding(clientId, {
			sessionCode: data.sessionCode,
			email,
			oidcSearch: oidcParams.toString(),
			expiresAt: data.sessionExpiresAt
		});
		requestCode = data.sessionCode;
		cooldown = RESEND_COOLDOWN_S;
		resent = true;
	}
</script>

<div class="grid gap-4">
	{#if error}
		<ErrorAlert message={error} title={m.auth_error_title()} />
	{/if}

	<div class="flex flex-col items-center gap-3 text-center">
		<MailCheckIcon class="size-10 text-primary" />
		<p class="text-sm">{m.magic_link_sent_description({ email })}</p>
		<p class="text-sm text-muted-foreground">{m.magic_link_sent_expiry()}</p>
	</div>

	{#if isContinuing}
		<div class="flex items-center justify-center gap-2 text-sm text-muted-foreground">
			<LoadingSpinner />
			{m.magic_link_signing_in()}
		</div>
	{:else}
		{#if resent}
			<p class="text-center text-sm text-muted-foreground">{m.magic_link_sent_resent()}</p>
		{/if}
		<Button
			type="button"
			variant="outline"
			class="relative w-full"
			disabled={isResending || cooldown > 0}
			onclick={onResend}
		>
			{#if isResending}
				<LoadingSpinner class="absolute left-4" />
			{/if}
			{cooldown > 0
				? m.magic_link_sent_resend_wait({ seconds: String(cooldown) })
				: m.magic_link_sent_resend()}
		</Button>
	{/if}
</div>

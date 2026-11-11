<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import QRCode from 'qrcode';
	import { toast } from 'svelte-sonner';
	import CircleCheckIcon from '@lucide/svelte/icons/circle-check';
	import CircleXIcon from '@lucide/svelte/icons/circle-x';
	import ClockIcon from '@lucide/svelte/icons/clock';

	import * as Alert from '$lib/components/ui/alert/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import SessionExpiryGuard from '$lib/components/auth/session-expiry-guard.svelte';

	import type { ClientAuthData } from '$lib/services/auth/get-client-auth-data';
	import {
		pollQrLoginApi,
		startQrLoginApi,
		type PollQrLoginResponse,
		type StartQrLoginResponse
	} from '$lib/services/auth/qr-login';
	import { SessionClock } from '$lib/state/auth-session-expiry.svelte';
	import { continueAfterLogin, mfaHintsFrom } from '$lib/utils/continue-login';
	import { buildOidcParams } from '$lib/utils/form';
	import { m } from '$lib/paraglide/messages.js';

	let { client }: { client: ClientAuthData | null } = $props();

	// A code that nobody scans is replaced by a new one while the page is in view, up
	// to this many in a row (about ten minutes). Then the user asks for one.
	const MAX_CODES = 5;

	type Phase = 'loading' | 'pending' | 'scanned' | 'signing-in' | 'expired' | 'denied' | 'error';

	const clientId = page.params.clientId as string;
	const searchParams = page.url.searchParams;
	const urlParams = buildOidcParams(searchParams);

	// The relying party's clock, as on the sign-in page: its state and PKCE verifier
	// die on their own schedule, and a code approved after that would be refused.
	const clock = new SessionClock();
	clock.adoptRequestExpiryHint(searchParams.get('request_expires_at'));

	let phase = $state<Phase>('loading');
	let error = $state<string | null>(null);
	let qrDataUrl = $state<string | null>(null);
	let expiresAt = $state(0);
	let now = $state(Date.now());

	let current: StartQrLoginResponse | null = null;
	let codesShown = 0;
	let pollTimer: ReturnType<typeof setTimeout> | undefined;
	let stopped = false;

	const secondsLeft = $derived(Math.max(0, Math.floor((expiresAt - now) / 1000)));
	const timeLeft = $derived(
		`${Math.floor(secondsLeft / 60)}:${String(secondsLeft % 60).padStart(2, '0')}`
	);

	async function showCode() {
		clearTimeout(pollTimer);
		if (stopped || clock.phase === 'expired') return;
		phase = 'loading';
		error = null;

		const [data, err] = await startQrLoginApi({
			clientId,
			redirectUri: searchParams.get('redirect_uri') || '/',
			responseType: searchParams.get('response_type') || '',
			scope: searchParams.get('scope') || '',
			state: searchParams.get('state') || '',
			codeChallenge: searchParams.get('code_challenge') || '',
			codeChallengeMethod: searchParams.get('code_challenge_method') || '',
			nonce: searchParams.get('nonce') || undefined
		});
		if (stopped) return;
		if (err || !data) {
			console.error(err);
			phase = 'error';
			error = err?.message || m.auth_generic_error();
			return;
		}

		codesShown += 1;
		current = data;
		expiresAt = Date.parse(data.expiresAt);
		// A QR code needs dark modules on a light background to scan, so it is drawn
		// black on white in either theme.
		qrDataUrl = await QRCode.toDataURL(data.url, {
			errorCorrectionLevel: 'M',
			margin: 2,
			width: 240
		});
		phase = 'pending';
		schedulePoll();
	}

	function schedulePoll() {
		if (stopped || !current) return;
		pollTimer = setTimeout(poll, current.pollIntervalSeconds * 1000);
	}

	async function poll() {
		if (stopped || !current) return;
		const [data, err] = await pollQrLoginApi({
			clientId,
			requestId: current.requestId,
			sessionCode: current.sessionCode,
			...mfaHintsFrom(urlParams)
		});
		if (stopped) return;

		if (err || !data) {
			if (err?.status === 401) {
				// The waiting session is gone (or another tab collected the sign-in).
				ended('expired');
			} else if (err && err.status >= 400 && err.status < 500 && err.status !== 429) {
				phase = 'error';
				error = err.message || m.auth_generic_error();
			} else {
				// A network blip or a busy server: ask again on the next tick.
				schedulePoll();
			}
			return;
		}

		switch (data.status) {
			case 'pending':
			case 'scanned':
				phase = data.status;
				if (data.expiresAt) expiresAt = Date.parse(data.expiresAt);
				schedulePoll();
				return;
			case 'expired':
				ended('expired');
				return;
			case 'denied':
				ended('denied');
				return;
			case 'approved':
				await signIn(data);
		}
	}

	function ended(kind: 'expired' | 'denied') {
		const wasUnscanned = phase === 'pending';
		current = null;
		qrDataUrl = null;
		if (
			kind === 'expired' &&
			wasUnscanned &&
			codesShown < MAX_CODES &&
			document.visibilityState === 'visible'
		) {
			void showCode();
			return;
		}
		phase = kind;
	}

	// Approved on the other device: from here this browser continues exactly as after
	// a password (a second factor, a forced password change, or straight to the app).
	async function signIn(data: PollQrLoginResponse) {
		stopped = true;
		if (!data.login) {
			phase = 'error';
			error = m.auth_generic_error();
			return;
		}
		phase = 'signing-in';
		toast.success(m.signin_success());
		const outcome = await continueAfterLogin(data.login, {
			clientId,
			email: data.email ?? '',
			oidcParams: urlParams
		});
		if (outcome.kind === 'navigated') return;
		phase = 'error';
		if (outcome.kind === 'enrollment-required') {
			error = m.signin_mfa_enrollment_required();
		} else if (outcome.kind === 'session-expired') {
			clock.expireNow();
			error = m.auth_generic_error();
		} else {
			error = outcome.message || m.auth_generic_error();
		}
	}

	function newCode() {
		stopped = false;
		codesShown = 0;
		void showCode();
	}

	onMount(() => {
		void showCode();
		const tick = setInterval(() => (now = Date.now()), 1000);
		// Back to a tab whose code ran out while it was hidden: show a fresh one.
		const onVisible = () => {
			if (document.visibilityState === 'visible' && phase === 'expired') newCode();
		};
		document.addEventListener('visibilitychange', onVisible);
		return () => {
			stopped = true;
			clearTimeout(pollTimer);
			clearInterval(tick);
			document.removeEventListener('visibilitychange', onVisible);
		};
	});

	function onExpire() {
		stopped = true;
		clearTimeout(pollTimer);
		current = null;
		qrDataUrl = null;
	}
</script>

<div class="grid gap-4">
	<SessionExpiryGuard
		{clock}
		{clientId}
		mode="pre-auth"
		initiateLoginUri={client?.initiateLoginUri ?? null}
		{onExpire}
	/>

	{#if phase === 'error'}
		<ErrorAlert message={error ?? m.auth_generic_error()} title={m.auth_error_title()} />
	{/if}

	{#if phase === 'loading' || phase === 'pending' || phase === 'scanned'}
		<div class="flex flex-col items-center gap-3">
			<div
				class="relative grid size-[240px] place-items-center overflow-hidden rounded-xl border bg-white"
			>
				{#if qrDataUrl}
					<img
						src={qrDataUrl}
						alt={m.qr_sign_in_qr_alt()}
						width="240"
						height="240"
						class="size-[240px] transition-[filter,opacity] {phase === 'scanned'
							? 'opacity-20 blur-[2px]'
							: ''}"
					/>
				{:else}
					<LoadingSpinner />
				{/if}
				{#if phase === 'scanned'}
					<div
						class="absolute inset-0 flex flex-col items-center justify-center gap-2 p-4 text-center text-neutral-900"
					>
						<CircleCheckIcon class="size-10 text-green-600" />
						<p class="font-semibold">{m.qr_sign_in_scanned_title()}</p>
					</div>
				{/if}
			</div>

			{#if phase === 'scanned'}
				<p class="text-center text-sm text-muted-foreground" aria-live="polite">
					{m.qr_sign_in_scanned_description({ time: timeLeft })}
				</p>
			{:else if phase === 'pending'}
				<p
					class="flex items-center gap-1.5 text-xs text-muted-foreground tabular-nums"
					aria-live="off"
				>
					<ClockIcon class="size-3.5" />
					{m.qr_sign_in_expires_in({ time: timeLeft })}
				</p>
			{/if}
		</div>

		{#if phase !== 'scanned'}
			<ol class="list-decimal space-y-1 pl-5 text-sm text-muted-foreground">
				<li>{m.qr_sign_in_step_camera()}</li>
				<li>{m.qr_sign_in_step_open()}</li>
				<li>{m.qr_sign_in_step_approve()}</li>
			</ol>
		{/if}
	{:else if phase === 'signing-in'}
		<div class="flex items-center justify-center gap-2 text-sm text-muted-foreground">
			<LoadingSpinner />
			{m.qr_sign_in_signing_in()}
		</div>
	{:else if phase === 'expired' || phase === 'denied'}
		<Alert.Root>
			{#if phase === 'denied'}
				<CircleXIcon class="size-5" />
			{:else}
				<ClockIcon class="size-5" />
			{/if}
			<Alert.Title class="font-bold">
				{phase === 'denied' ? m.qr_sign_in_denied_title() : m.qr_sign_in_expired_title()}
			</Alert.Title>
			<Alert.Description>
				{phase === 'denied'
					? m.qr_sign_in_denied_description()
					: m.qr_sign_in_expired_description()}
			</Alert.Description>
		</Alert.Root>
	{/if}

	{#if phase === 'expired' || phase === 'denied' || phase === 'error'}
		<Button onclick={newCode} disabled={clock.phase === 'expired'}>
			{m.qr_sign_in_new_code()}
		</Button>
	{/if}

	{#if phase !== 'signing-in'}
		<a
			href={resolve(`/auth/[clientId]/sign-in?${urlParams.toString()}`, { clientId })}
			class="text-center text-sm font-semibold hover:underline"
		>
			{m.qr_sign_in_other_way()}
		</a>
	{/if}
</div>

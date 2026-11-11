<script lang="ts">
	import { onMount } from 'svelte';
	import { goto, replaceState } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import CircleCheckIcon from '@lucide/svelte/icons/circle-check';
	import CircleXIcon from '@lucide/svelte/icons/circle-x';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import MapPinIcon from '@lucide/svelte/icons/map-pin';
	import MonitorIcon from '@lucide/svelte/icons/monitor';
	import ScanQrCodeIcon from '@lucide/svelte/icons/scan-qr-code';
	import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';

	import * as Alert from '$lib/components/ui/alert/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { denyQrLoginApi, scanQrLoginApi } from '$lib/services/auth/qr-login';
	import {
		clearQrApproval,
		loadQrApproval,
		saveQrApproval,
		type QrApproval
	} from '$lib/state/qr-approval';
	import { approvalSignInParams } from '$lib/utils/qr-approval';
	import { describeUserAgent } from '$lib/utils/user-agent';
	import { m } from '$lib/paraglide/messages.js';

	// QR code sign-in, on the device that scanned the code. The camera opened
	// `/auth/<client>/qr-approve#<requestId>.<token>`: claim the code, show the user
	// what is asking, and on "continue" sign them in on this device; that sign-in
	// approves the other one (utils/qr-approval). The sign-in pages send the user
	// back here with `?result=` when it is over.
	type Phase =
		| 'loading'
		| 'review'
		| 'continuing'
		| 'approved'
		| 'denied'
		| 'failed'
		| 'invalid'
		| 'error';

	const clientId = page.params.clientId as string;

	let phase = $state<Phase>('loading');
	let error = $state<string | null>(null);
	let approval = $state<QrApproval | null>(null);
	let busy = $state(false);
	let now = $state(Date.now());

	const secondsLeft = $derived(
		approval ? Math.max(0, Math.floor((Date.parse(approval.expiresAt) - now) / 1000)) : 0
	);
	const timeLeft = $derived(
		`${Math.floor(secondsLeft / 60)}:${String(secondsLeft % 60).padStart(2, '0')}`
	);
	const device = $derived(describeUserAgent(approval?.requester.userAgent));
	const requestedAt = $derived(
		approval
			? new Date(approval.requestedAt).toLocaleTimeString([], {
					hour: '2-digit',
					minute: '2-digit'
				})
			: ''
	);

	// `#<requestId>.<token>`: the fragment never reaches a server.
	function readCode(): { requestId: string; token: string } | null {
		const fragment = window.location.hash.slice(1);
		const dot = fragment.indexOf('.');
		if (dot <= 0) return null;
		const requestId = fragment.slice(0, dot);
		const token = fragment.slice(dot + 1);
		return requestId && token ? { requestId, token } : null;
	}

	// Drop the secret from the address bar and the history entry. Deferred a tick: the
	// router refuses shallow navigation during the very first hydration pass.
	function forgetCode() {
		setTimeout(() => {
			try {
				replaceState(resolve('/auth/[clientId]/qr-approve', { clientId }), {});
			} catch (err) {
				console.error(err);
			}
		}, 0);
	}

	onMount(() => {
		const tick = setInterval(() => (now = Date.now()), 1000);
		const result = page.url.searchParams.get('result');
		const code = readCode();
		if (code) {
			forgetCode();
			void scan(code);
		} else if (result === 'approved') {
			phase = 'approved';
		} else if (result === 'failed') {
			phase = 'failed';
		} else {
			// A reload after the scan: the claim is remembered in this tab.
			approval = loadQrApproval(clientId);
			phase = approval ? 'review' : 'invalid';
		}
		return () => clearInterval(tick);
	});

	async function scan(code: { requestId: string; token: string }) {
		const [data, err] = await scanQrLoginApi({ clientId, ...code });
		if (err || !data) {
			console.error(err);
			if (err?.status === 401) {
				// Expired, already scanned (by this device, after a reload, or another
				// one), or not a real code: the remedy is the same.
				const saved = loadQrApproval(clientId);
				if (saved?.requestId === code.requestId) {
					approval = saved;
					phase = 'review';
					return;
				}
				phase = 'failed';
			} else {
				phase = 'error';
				error = err?.message || m.auth_generic_error();
			}
			return;
		}
		approval = { ...data, requestId: code.requestId };
		saveQrApproval(clientId, approval);
		phase = 'review';
	}

	async function onContinue() {
		if (!approval?.signIn) {
			phase = 'failed';
			return;
		}
		busy = true;
		phase = 'continuing';
		const params = await approvalSignInParams(approval.requestId, approval.signIn);
		await goto(resolve(`/auth/[clientId]/sign-in?${params.toString()}`, { clientId }));
	}

	async function onDeny() {
		if (!approval) return;
		busy = true;
		const [, err] = await denyQrLoginApi({
			clientId,
			requestId: approval.requestId,
			claimCode: approval.claimCode
		});
		busy = false;
		if (err && err.status !== 401) {
			error = err.message || m.auth_generic_error();
			phase = 'error';
			return;
		}
		clearQrApproval(clientId);
		phase = err ? 'failed' : 'denied';
	}

	$effect(() => {
		if (phase === 'review' && approval && secondsLeft === 0) {
			clearQrApproval(clientId);
			phase = 'failed';
		}
	});
</script>

<div class="grid gap-4">
	{#if phase === 'loading' || phase === 'continuing'}
		<div class="flex items-center justify-center gap-2 py-6 text-sm text-muted-foreground">
			<LoadingSpinner />
			{m.qr_approve_loading()}
		</div>
	{:else if phase === 'review' && approval}
		<div class="flex flex-col space-y-2 text-center">
			<h1 class="text-2xl font-semibold tracking-tight">{m.qr_approve_heading()}</h1>
			<p class="text-sm text-muted-foreground">
				{m.qr_approve_description({ app: approval.clientName })}
			</p>
		</div>

		<dl class="divide-y rounded-lg border text-sm">
			<div class="flex items-center gap-3 px-4 py-3">
				<MonitorIcon class="size-4 shrink-0 text-muted-foreground" />
				<dt class="sr-only">{m.qr_approve_device()}</dt>
				<dd class="font-medium">{device ?? m.qr_approve_unknown_device()}</dd>
			</div>
			<div class="flex items-center gap-3 px-4 py-3">
				<MapPinIcon class="size-4 shrink-0 text-muted-foreground" />
				<dt class="sr-only">{m.qr_approve_location()}</dt>
				<dd>{approval.requester.location ?? m.qr_approve_unknown_location()}</dd>
			</div>
			<div class="flex items-center gap-3 px-4 py-3">
				<ClockIcon class="size-4 shrink-0 text-muted-foreground" />
				<dt class="sr-only">{m.qr_approve_requested()}</dt>
				<dd>{m.qr_approve_requested_at({ time: requestedAt })}</dd>
			</div>
		</dl>

		<Alert.Root>
			<ShieldAlertIcon class="size-5" />
			<Alert.Title class="font-bold">{m.qr_approve_warning_title()}</Alert.Title>
			<Alert.Description>{m.qr_approve_warning()}</Alert.Description>
		</Alert.Root>

		<div class="grid gap-2">
			<Button onclick={onContinue} disabled={busy}>{m.qr_approve_continue()}</Button>
			<Button variant="outline" onclick={onDeny} disabled={busy}>{m.qr_approve_deny()}</Button>
		</div>
		<p class="text-center text-xs text-muted-foreground tabular-nums">
			{m.qr_approve_expires_in({ time: timeLeft })}
		</p>
	{:else if phase === 'approved'}
		<Alert.Root class="bg-green-100 dark:bg-green-700">
			<CircleCheckIcon class="size-5 text-green-600 dark:text-green-300" />
			<Alert.Title class="font-bold">{m.qr_approve_approved_title()}</Alert.Title>
			<Alert.Description>{m.qr_approve_approved_description()}</Alert.Description>
		</Alert.Root>
	{:else if phase === 'denied'}
		<Alert.Root>
			<CircleXIcon class="size-5" />
			<Alert.Title class="font-bold">{m.qr_approve_denied_title()}</Alert.Title>
			<Alert.Description>{m.qr_approve_denied_description()}</Alert.Description>
		</Alert.Root>
	{:else if phase === 'failed'}
		<Alert.Root>
			<ClockIcon class="size-5" />
			<Alert.Title class="font-bold">{m.qr_approve_failed_title()}</Alert.Title>
			<Alert.Description>{m.qr_approve_failed_description()}</Alert.Description>
		</Alert.Root>
	{:else if phase === 'invalid'}
		<Alert.Root>
			<ScanQrCodeIcon class="size-5" />
			<Alert.Title class="font-bold">{m.qr_approve_invalid_title()}</Alert.Title>
			<Alert.Description>{m.qr_approve_invalid_description()}</Alert.Description>
		</Alert.Root>
	{:else if phase === 'error'}
		<ErrorAlert message={error ?? m.auth_generic_error()} title={m.auth_error_title()} />
	{/if}
</div>

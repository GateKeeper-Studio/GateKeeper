<script lang="ts">
	import { onMount } from 'svelte';
	import { replaceState } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import CheckCircleIcon from '@lucide/svelte/icons/circle-check';
	import MonitorSmartphoneIcon from '@lucide/svelte/icons/monitor-smartphone';

	import * as Alert from '$lib/components/ui/alert/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import type { LoginResponse } from '$lib/services/auth/login';
	import { redeemMagicLinkApi } from '$lib/services/auth/magic-link';
	import {
		clearMagicLinkBinding,
		loadMagicLinkBinding,
		openMagicLinkChannel,
		type MagicLinkBinding,
		type MagicLinkMessage
	} from '$lib/state/magic-link-binding';
	import { continueAfterLogin, mfaHintsFrom } from '$lib/utils/continue-login';
	import { m } from '$lib/paraglide/messages.js';

	// How long the tab that asked for the link gets to answer before this tab finishes
	// the sign-in itself (the asking tab may have been closed).
	const ACK_TIMEOUT_MS = 2000;

	type Phase =
		| 'redeeming'
		| 'continuing'
		| 'other-browser'
		| 'invalid'
		| 'continued-elsewhere'
		| 'error';

	const clientId = page.params.clientId as string;

	let phase = $state<Phase>('redeeming');
	let error = $state<string | null>(null);
	// The binding's OIDC query string, to start over from the sign-in page.
	let retrySearch = $state<string | null>(null);

	// The link carries `#id=…&token=…` in its fragment, which never reaches a server.
	function readLink(): { id: string; token: string } | null {
		const fragment = new URLSearchParams(window.location.hash.slice(1));
		const id = fragment.get('id');
		const token = fragment.get('token');
		return id && token ? { id, token } : null;
	}

	// Drop the secret from the address bar and the history entry. Deferred a tick: the
	// router refuses shallow navigation during the very first hydration pass.
	function forgetLink() {
		setTimeout(() => {
			try {
				replaceState(page.url.pathname, {});
			} catch (err) {
				console.error(err);
			}
		}, 0);
	}

	onMount(() => {
		const link = readLink();
		forgetLink();
		void redeem(link);
	});

	async function redeem(link: { id: string; token: string } | null) {
		if (!link) {
			phase = 'invalid';
			return;
		}

		// No binding: this is not the browser that asked for the link (forwarded, opened
		// in a mail app's own browser, or by a scanner). Nothing to send — the IdP would
		// refuse it anyway, and the link stays usable where it belongs.
		const binding = loadMagicLinkBinding(clientId);
		if (!binding) {
			phase = 'other-browser';
			return;
		}
		retrySearch = binding.oidcSearch;
		const oidcParams = new URLSearchParams(binding.oidcSearch);

		const [data, err] = await redeemMagicLinkApi({
			clientId,
			id: link.id,
			token: link.token,
			sessionCode: binding.sessionCode,
			...mfaHintsFrom(oidcParams)
		});

		if (err || !data) {
			console.error(err);
			if (err?.status === 401) {
				phase = 'invalid';
			} else {
				phase = 'error';
				error = err?.message || m.auth_generic_error();
			}
			return;
		}

		clearMagicLinkBinding(clientId);

		if (await handOff(binding, data)) {
			phase = 'continued-elsewhere';
			return;
		}

		phase = 'continuing';
		const outcome = await continueAfterLogin(data, {
			clientId,
			email: binding.email,
			oidcParams
		});
		if (outcome.kind === 'navigated') return;

		phase = 'error';
		if (outcome.kind === 'enrollment-required') {
			error = m.signin_mfa_enrollment_required();
		} else if (outcome.kind === 'session-expired') {
			error = m.magic_link_invalid();
		} else {
			error = outcome.message || m.auth_generic_error();
		}
	}

	// Give the result to the tab that asked for the link. Resolves true once it answers.
	function handOff(binding: MagicLinkBinding, response: LoginResponse): Promise<boolean> {
		const channel = openMagicLinkChannel(clientId);
		if (!channel) return Promise.resolve(false);

		return new Promise((settle) => {
			const timer = setTimeout(() => {
				channel.close();
				settle(false);
			}, ACK_TIMEOUT_MS);

			channel.onmessage = (event: MessageEvent<MagicLinkMessage>) => {
				if (event.data.type === 'ack' && event.data.requestCode === binding.sessionCode) {
					clearTimeout(timer);
					channel.close();
					settle(true);
				}
			};

			channel.postMessage({
				type: 'redeemed',
				requestCode: binding.sessionCode,
				email: binding.email,
				oidcSearch: binding.oidcSearch,
				response
			} satisfies MagicLinkMessage);
		});
	}
</script>

<div class="grid gap-4">
	{#if phase === 'redeeming' || phase === 'continuing'}
		<div class="flex items-center justify-center gap-2 text-sm text-muted-foreground">
			<LoadingSpinner />
			{m.magic_link_signing_in()}
		</div>
	{:else if phase === 'continued-elsewhere'}
		<Alert.Root variant="default" class="bg-green-100 dark:bg-green-700">
			<CheckCircleIcon class="h-5 w-5 text-green-500 dark:text-green-400" />
			<Alert.Title class="font-bold">{m.magic_link_signed_in_title()}</Alert.Title>
			<Alert.Description>{m.magic_link_continue_in_other_tab()}</Alert.Description>
		</Alert.Root>
	{:else if phase === 'other-browser'}
		<Alert.Root variant="default">
			<MonitorSmartphoneIcon class="h-5 w-5" />
			<Alert.Title class="font-bold">{m.magic_link_open_in_same_browser_title()}</Alert.Title>
			<Alert.Description>{m.magic_link_open_in_same_browser()}</Alert.Description>
		</Alert.Root>
	{:else if phase === 'invalid'}
		<ErrorAlert message={m.magic_link_invalid()} title={m.auth_error_title()} />
	{:else if phase === 'error'}
		<ErrorAlert message={error ?? m.auth_generic_error()} title={m.auth_error_title()} />
	{/if}

	{#if (phase === 'invalid' || phase === 'error') && retrySearch !== null}
		<a
			href={resolve(`/auth/[clientId]/sign-in?${retrySearch}`, { clientId })}
			class="text-center text-sm font-semibold hover:underline"
		>
			{m.magic_link_request_new()}
		</a>
	{/if}
</div>

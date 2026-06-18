<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { buildOidcParams } from '$lib/utils/form';
	import { EMfaType } from '$lib/services/auth/login';
	import { sendEmailMfaApi } from '$lib/services/auth/send-email-mfa';
	import { mfaMethods } from './mfa-method-meta';

	let { currentMethod }: { currentMethod: EMfaType } = $props();

	const clientId = page.params.clientId as string;
	const searchParams = page.url.searchParams;

	// The user's usable second factors (confirmed enrollments + implicit email,
	// already filtered by the client's MFA policy). Forwarded from the login
	// response's availableMfaMethods through the auth pages' query string.
	const availableMethods = (searchParams.get('available_mfa_methods') || '')
		.split(',')
		.map((method) => method.trim())
		.filter((method): method is EMfaType =>
			[EMfaType.MfaWebauthn, EMfaType.MfaApp, EMfaType.MfaEmail].includes(method as EMfaType)
		);

	const canSwitch = availableMethods.some((method) => method !== currentMethod);

	let open = $state(false);
	let switching = $state<EMfaType | null>(null);

	function targetPath(type: EMfaType, params: URLSearchParams) {
		switch (type) {
			case EMfaType.MfaEmail:
				return resolve(`/auth/[clientId]/mfa-mail?${params.toString()}`, { clientId });
			case EMfaType.MfaApp:
				return resolve(`/auth/[clientId]/mfa-app?${params.toString()}`, { clientId });
			case EMfaType.MfaWebauthn:
				return resolve(`/auth/[clientId]/mfa-webauthn?${params.toString()}`, { clientId });
		}
	}

	async function switchTo(type: EMfaType) {
		if (switching) return;
		switching = type;

		const sessionCode = sessionStorage.getItem(`auth_session_${clientId}`) || '';
		if (!sessionCode) {
			toast.error('Your sign-in session expired. Please sign in again.');
			switching = null;
			return;
		}

		// E-mail needs a fresh code issued before its page is useful; the
		// authenticator and passkey pages create their own challenge.
		if (type === EMfaType.MfaEmail) {
			const [, sendErr] = await sendEmailMfaApi({ clientId, sessionCode });
			if (sendErr) {
				toast.error(sendErr.data?.message || 'Could not send a verification code.');
				switching = null;
				return;
			}
			toast.success('A verification code was sent to your e-mail');
		}

		// Carry the sign-in flow params over, but drop the previous method's
		// mfa_id — each page resolves its own challenge.
		const urlParams = buildOidcParams(searchParams, {
			mfa_auth_app_required: searchParams.get('mfa_auth_app_required') || '',
			email: searchParams.get('email') || '',
			change_password_code: searchParams.get('change_password_code') || '',
			user_id: searchParams.get('user_id') || ''
		});

		open = false;
		switching = null;
		await goto(targetPath(type, urlParams));
	}
</script>

{#if canSwitch}
	<Dialog.Root bind:open>
		<div
			class="flex flex-col gap-2 rounded-md bg-amber-500/10 p-2 text-center text-sm text-muted-foreground"
		>
			<span>Cannot use this authentication method now?</span>

			<Dialog.Trigger
				class="rounded-sm bg-amber-600/30 p-1 px-3 text-foreground transition-all hover:brightness-50"
			>
				Try another one
			</Dialog.Trigger>
		</div>

		<Dialog.Content class="top-[40%] sm:max-w-[38.75rem]">
			<Dialog.Header>
				<Dialog.Title>Two Factor Methods</Dialog.Title>

				<Dialog.Description>
					Two-factor authentication adds an additional layer of security to your account by
					requiring more than just a password to sign in.
				</Dialog.Description>
			</Dialog.Header>

			{#each mfaMethods as method (method.type)}
				{@const isCurrent = method.type === currentMethod}
				{@const isAvailable = availableMethods.includes(method.type)}
				<button
					type="button"
					class="w-full text-left disabled:opacity-70"
					disabled={isCurrent || !isAvailable || switching !== null}
					onclick={() => switchTo(method.type)}
				>
					<Card.Root
						class={`flex w-full flex-row overflow-hidden p-0 transition-all hover:shadow-lg ${
							isCurrent || !isAvailable
								? 'hover:cursor-not-allowed'
								: 'hover:scale-[1.01] hover:cursor-pointer'
						}`}
					>
						<div class="flex items-center justify-center bg-muted p-4">
							<method.icon class="h-6 w-6 text-muted-foreground" />
						</div>

						<Card.Header class="w-full p-4">
							<Card.Title class="flex justify-between gap-2">
								{method.label}
								{#if switching === method.type}
									<LoadingSpinner />
								{:else if isCurrent}
									<Badge variant="outline">Current Method</Badge>
								{:else if isAvailable}
									<Badge>Configured</Badge>
								{:else}
									<Badge variant="secondary">Not Configured</Badge>
								{/if}
							</Card.Title>

							<Card.Description class="line-clamp-4">
								{method.description}
							</Card.Description>
						</Card.Header>
					</Card.Root>
				</button>
			{/each}
		</Dialog.Content>
	</Dialog.Root>
{/if}

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
	import { readSessionCode } from '$lib/state/auth-session-expiry.svelte';
	import { mfaMethods } from './mfa-method-meta';
	import { m } from '$lib/paraglide/messages.js';

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

		// Read late, via the shared helper: a session refresh rotates the code, so a value
		// captured any earlier than this could already be dead.
		const sessionCode = readSessionCode(clientId);
		if (!sessionCode) {
			toast.error(m.mfa_modal_session_expired());
			switching = null;
			return;
		}

		// E-mail needs a fresh code issued before its page is useful; the
		// authenticator and passkey pages create their own challenge.
		if (type === EMfaType.MfaEmail) {
			const [, sendErr] = await sendEmailMfaApi({ clientId, sessionCode });
			if (sendErr) {
				toast.error(sendErr.message || m.mfa_modal_send_code_error());
				switching = null;
				return;
			}
			toast.success(m.mfa_modal_code_sent());
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
			<span>{m.mfa_modal_cannot_use_method()}</span>

			<Dialog.Trigger
				class="rounded-sm bg-amber-600/30 p-1 px-3 text-foreground transition-all hover:brightness-50"
			>
				{m.mfa_modal_try_another()}
			</Dialog.Trigger>
		</div>

		<Dialog.Content class="top-[40%] sm:max-w-[38.75rem]">
			<Dialog.Header>
				<Dialog.Title>{m.mfa_modal_title()}</Dialog.Title>

				<Dialog.Description>
					{m.mfa_modal_description()}
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
									<Badge variant="outline">{m.mfa_modal_current_method()}</Badge>
								{:else if isAvailable}
									<Badge>{m.mfa_modal_configured()}</Badge>
								{:else}
									<Badge variant="secondary">{m.mfa_modal_not_configured()}</Badge>
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

<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import {
		acceptInvitation,
		lookupInvitation,
		type InvitationDetails
	} from '$lib/services/auth/admin-invitation';

	// Keep in sync with the server's admin password policy.
	const MIN_LENGTH = 12;
	const MAX_LENGTH = 128;

	type State = 'loading' | 'invalid' | 'form' | 'done';
	let view = $state<State>('loading');
	let token = '';
	let invitation = $state<InvitationDetails | null>(null);
	let name = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let error = $state('');
	let busy = $state(false);

	onMount(async () => {
		// The token travels in the fragment, which no server ever receives. Take it,
		// then drop it from the address bar and the history entry.
		token = decodeURIComponent(window.location.hash.slice(1));
		history.replaceState(history.state, '', window.location.pathname);
		if (!token) {
			view = 'invalid';
			return;
		}
		const [data, err] = await lookupInvitation(token);
		if (err || !data) {
			view = 'invalid';
			return;
		}
		invitation = data;
		view = 'form';
	});

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();
		const length = [...password].length;
		if (name.trim().length === 0) {
			error = m.invite_name_required();
			return;
		}
		if (length < MIN_LENGTH) {
			error = m.settings_password_min_length({ minLength: MIN_LENGTH });
			return;
		}
		if (length > MAX_LENGTH) {
			error = m.settings_password_max_length({ maxLength: MAX_LENGTH });
			return;
		}
		if (password !== confirmPassword) {
			error = m.settings_password_mismatch();
			return;
		}
		error = '';
		busy = true;
		const [, err] = await acceptInvitation(token, name.trim(), password);
		busy = false;
		if (err) {
			if (err.status === 404) {
				view = 'invalid';
				return;
			}
			error = err.data?.message || err.message;
			return;
		}
		password = '';
		confirmPassword = '';
		view = 'done';
	}
</script>

<svelte:head>
	<title>{m.invite_title()} - GateKeeper</title>
</svelte:head>

<main class="mx-auto flex min-h-screen max-w-sm flex-col justify-center px-4 py-10">
	<span
		class="mb-8 flex size-9 items-center justify-center rounded-lg bg-gradient-to-br from-brand to-brand-2 text-brand-foreground"
	>
		<ShieldCheckIcon class="size-5" />
	</span>

	{#if view === 'loading'}
		<div class="flex items-center gap-2 text-muted-foreground"><LoadingSpinner /></div>
	{:else if view === 'invalid'}
		<h1 class="text-2xl font-semibold tracking-tight">{m.invite_invalid_title()}</h1>
		<p class="mt-1.5 text-sm text-muted-foreground">{m.invite_invalid_description()}</p>
	{:else if view === 'done'}
		<h1 class="text-2xl font-semibold tracking-tight">{m.invite_done_title()}</h1>
		<p class="mt-1.5 text-sm text-muted-foreground">
			{m.invite_done_description({ email: invitation?.email ?? '' })}
		</p>
		<Button href={resolve('/login')} class="mt-6 w-fit">{m.invite_go_to_sign_in()}</Button>
	{:else if invitation}
		<h1 class="text-2xl font-semibold tracking-tight">{m.invite_title()}</h1>
		<p class="mt-1.5 text-sm text-muted-foreground">
			{m.invite_description({ organization: invitation.organizationName })}
		</p>

		{#if error}
			<div class="mt-6"><ErrorAlert message={error} title={m.auth_error_title()} /></div>
		{/if}

		<form onsubmit={onSubmit} class="mt-6 grid gap-4">
			<div class="space-y-2">
				<Label for="invite-email">{m.auth_email_label()}</Label>
				<Input id="invite-email" type="email" value={invitation.email} readonly disabled />
			</div>
			<div class="space-y-2">
				<Label for="invite-name">{m.settings_account_name_label()}</Label>
				<Input id="invite-name" type="text" autocomplete="name" bind:value={name} />
			</div>
			<div class="space-y-2">
				<Label for="invite-password">{m.settings_password_new_label()}</Label>
				<Input
					id="invite-password"
					type="password"
					autocomplete="new-password"
					bind:value={password}
				/>
			</div>
			<div class="space-y-2">
				<Label for="invite-confirm">{m.settings_password_confirm_label()}</Label>
				<Input
					id="invite-confirm"
					type="password"
					autocomplete="new-password"
					bind:value={confirmPassword}
				/>
			</div>
			<Button type="submit" disabled={busy} class="relative">
				{#if busy}<LoadingSpinner class="absolute left-3" />{/if}
				{m.invite_submit()}
			</Button>
		</form>
	{/if}
</main>

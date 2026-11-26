<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';

	import { Button } from '$lib/components/ui/button/index.js';
	import MfaCard from '$lib/components/settings-dialog/sections/mfa-card.svelte';
	import { adminLogout } from '$lib/services/auth/admin-login';
	import { m } from '$lib/paraglide/messages.js';

	let { data } = $props();

	async function signOut() {
		await adminLogout();
		await goto(resolve('/login'));
	}
</script>

<svelte:head>
	<title>{m.two_step_required_title()} - GateKeeper</title>
</svelte:head>

<main class="mx-auto flex min-h-screen max-w-[640px] flex-col justify-center px-4 py-10">
	<div class="flex items-start gap-3">
		<ShieldAlertIcon class="mt-1 size-6 shrink-0 text-brand" />
		<div>
			<h1 class="text-2xl font-semibold tracking-tight">{m.two_step_required_title()}</h1>
			<p class="mt-1.5 text-sm text-muted-foreground">
				{m.two_step_required_description({ email: data.admin.email })}
			</p>
		</div>
	</div>

	<MfaCard onenabled={() => goto(resolve('/console'))} />

	<div class="mt-6">
		<Button variant="ghost" onclick={signOut}>{m.two_step_required_sign_out()}</Button>
	</div>
</main>

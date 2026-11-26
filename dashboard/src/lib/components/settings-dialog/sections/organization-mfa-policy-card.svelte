<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import { setAdminMfaPolicy } from '$lib/services/admin/organization-admins';

	let { required }: { required: boolean } = $props();

	const queryClient = useQueryClient();
	let open = $state(false);
	let password = $state('');
	let busy = $state(false);
	let error = $state('');

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (password.length === 0) {
			error = m.settings_mfa_password_required();
			return;
		}
		error = '';
		busy = true;
		const [, err] = await setAdminMfaPolicy(!required, password);
		busy = false;
		password = '';
		if (err) {
			error = err.data?.message || err.message || m.settings_mfa_error();
			return;
		}
		open = false;
		toast.success(m.org_admins_done());
		queryClient.invalidateQueries({ queryKey: ['organization'] });
	}
</script>

<Card.Root class="mt-6 max-w-[640px]">
	<Card.Header>
		<Card.Title class="flex items-center gap-2">
			{m.org_mfa_policy_title()}
			{#if required}
				<Badge variant="default">{m.org_mfa_policy_required()}</Badge>
			{:else}
				<Badge variant="outline">{m.org_mfa_policy_optional()}</Badge>
			{/if}
		</Card.Title>
		<Card.Description>{m.org_mfa_policy_description()}</Card.Description>
	</Card.Header>
	<Card.Content>
		<Button
			variant={required ? 'outline' : 'default'}
			onclick={() => {
				error = '';
				password = '';
				open = true;
			}}
		>
			{required ? m.org_mfa_policy_turn_off() : m.org_mfa_policy_turn_on()}
		</Button>
	</Card.Content>
</Card.Root>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>
				{required ? m.org_mfa_policy_turn_off() : m.org_mfa_policy_turn_on()}
			</Dialog.Title>
			<Dialog.Description>
				{required ? m.org_mfa_policy_off_description() : m.org_mfa_policy_on_description()}
			</Dialog.Description>
		</Dialog.Header>
		<form onsubmit={submit} class="grid gap-4">
			<div class="space-y-2">
				<Label for="mfa-policy-password">{m.settings_mfa_password_label()}</Label>
				<Input
					id="mfa-policy-password"
					type="password"
					autocomplete="current-password"
					bind:value={password}
				/>
			</div>
			{#if error}<p class="text-sm font-medium text-destructive">{error}</p>{/if}
			<Dialog.Footer>
				<Button type="button" variant="outline" disabled={busy} onclick={() => (open = false)}>
					{m.settings_mfa_cancel()}
				</Button>
				<Button type="submit" disabled={busy} class="relative">
					{#if busy}<LoadingSpinner class="absolute left-3" />{/if}
					{m.settings_mfa_continue()}
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>

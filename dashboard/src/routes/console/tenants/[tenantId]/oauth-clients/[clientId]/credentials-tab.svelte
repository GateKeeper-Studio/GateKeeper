<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import CopyIcon from '@lucide/svelte/icons/copy';
	import CheckIcon from '@lucide/svelte/icons/check';
	import RefreshCcwIcon from '@lucide/svelte/icons/refresh-ccw';
	import InfoIcon from '@lucide/svelte/icons/info';
	import IdCardIcon from '@lucide/svelte/icons/id-card';
	import Building2Icon from '@lucide/svelte/icons/building-2';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import LockIcon from '@lucide/svelte/icons/lock';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { rotateOAuthClientSecret, type OAuthClientView } from '$lib/services/admin/oauth-clients';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		client: OAuthClientView;
		tenantId: string;
		clientId: string;
	};

	let { client, tenantId, clientId }: Props = $props();

	const queryClient = useQueryClient();

	// ---- copy with flash feedback ------------------------------------------------
	let tenantIdCopied = $state(false);
	let clientIdCopied = $state(false);

	async function copyValue(value: string, label: string, flashSignal?: () => void) {
		try {
			await navigator.clipboard.writeText(value);
			flashSignal?.();
			toast.success(m.oauth_clients_toast_copied_with_label({ label }));
		} catch {
			toast.error(m.oauth_clients_toast_copy_error());
		}
	}

	// ---- secret rotation ---------------------------------------------------------
	let rotateConfirmOpen = $state(false);
	let rotating = $state(false);
	let rotatedSecret = $state<string | null>(null);
	let secretRevealOpen = $state(false);
	let secretCopied = $state(false);

	async function confirmRotate() {
		rotating = true;
		const [data, err] = await rotateOAuthClientSecret(tenantId, clientId);
		rotating = false;

		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}

		rotateConfirmOpen = false;
		rotatedSecret = data.client_secret ?? null;
		secretRevealOpen = true;
		toast.success(m.oauth_clients_credentials_tab_rotate_success_toast());
		queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId, clientId] });
		queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId] });
	}
</script>

<div class="grid max-w-3xl gap-4">
	<!-- Tenant ID -->
	<Card.Root>
		<Card.Header>
			<div class="flex items-start justify-between gap-3">
				<div class="space-y-1.5">
					<Card.Title class="flex items-center gap-2">
						<Building2Icon class="text-muted-foreground size-4" />
						{m.oauth_clients_credentials_tab_tenant_id_title()}
					</Card.Title>
					<Card.Description>
						{m.oauth_clients_credentials_tab_tenant_id_description()}
					</Card.Description>
				</div>
				<Badge
					variant="outline"
					class="shrink-0 gap-1 border-violet-500/20 bg-violet-500/5 text-violet-700 dark:text-violet-400"
				>
					<Building2Icon class="size-3" />
					{m.oauth_clients_credentials_tab_tenant_badge()}
				</Badge>
			</div>
		</Card.Header>
		<Card.Content>
			<div class="bg-muted/40 flex items-center gap-2 rounded-lg border p-1.5 pl-3">
				<code class="flex-1 truncate font-mono text-sm">{tenantId}</code>
				<Button
					variant="ghost"
					size="icon"
					class="text-muted-foreground hover:text-foreground size-8 shrink-0"
					onclick={() =>
						copyValue(tenantId, m.oauth_clients_credentials_tab_tenant_id_title(), () => {
							tenantIdCopied = true;
							setTimeout(() => (tenantIdCopied = false), 1500);
						})}
				>
					{#if tenantIdCopied}<CheckIcon class="text-emerald-600 dark:text-emerald-400" />{:else}
						<CopyIcon />
					{/if}
				</Button>
			</div>
		</Card.Content>
	</Card.Root>

	<!-- Client ID -->
	<Card.Root>
		<Card.Header>
			<div class="flex items-start justify-between gap-3">
				<div class="space-y-1.5">
					<Card.Title class="flex items-center gap-2">
						<IdCardIcon class="text-muted-foreground size-4" />
						{m.oauth_clients_credentials_tab_client_id_title()}
					</Card.Title>
					<Card.Description>{m.oauth_clients_credentials_tab_client_id_description()}</Card.Description>
				</div>
				<Badge
					variant="outline"
					class="shrink-0 gap-1 border-sky-500/20 bg-sky-500/5 text-sky-700 dark:text-sky-400"
				>
					<GlobeIcon class="size-3" />
					{m.oauth_clients_credentials_tab_public_badge()}
				</Badge>
			</div>
		</Card.Header>
		<Card.Content>
			<div class="bg-muted/40 flex items-center gap-2 rounded-lg border p-1.5 pl-3">
				<code class="flex-1 truncate font-mono text-sm">{client.client_id}</code>
				<Button
					variant="ghost"
					size="icon"
					class="text-muted-foreground hover:text-foreground size-8 shrink-0"
					onclick={() =>
						copyValue(client.client_id, m.oauth_clients_credentials_tab_client_id_title(), () => {
							clientIdCopied = true;
							setTimeout(() => (clientIdCopied = false), 1500);
						})}
				>
					{#if clientIdCopied}<CheckIcon class="text-emerald-600 dark:text-emerald-400" />{:else}
						<CopyIcon />
					{/if}
				</Button>
			</div>
		</Card.Content>
	</Card.Root>

	<!-- Client secret -->
	<Card.Root>
		<Card.Header>
			<div class="flex items-start justify-between gap-3">
				<div class="space-y-1.5">
					<Card.Title class="flex items-center gap-2">
						<KeyRoundIcon class="text-muted-foreground size-4" />
						{m.oauth_clients_credentials_tab_secret_title()}
					</Card.Title>
					<Card.Description>{m.oauth_clients_credentials_tab_secret_description()}</Card.Description>
				</div>
				{#if client.client_type === 'confidential'}
					<Badge
						variant="outline"
						class="shrink-0 gap-1 border-emerald-500/20 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400"
					>
						<ShieldCheckIcon class="size-3" />
						{m.oauth_clients_credentials_tab_secret_configured_badge()}
					</Badge>
				{:else}
					<Badge
						variant="outline"
						class="shrink-0 gap-1 border-sky-500/20 bg-sky-500/5 text-sky-700 dark:text-sky-400"
					>
						<GlobeIcon class="size-3" />
						{m.oauth_clients_credentials_tab_public_client_badge()}
					</Badge>
				{/if}
			</div>
		</Card.Header>
		<Card.Content>
			{#if client.client_type === 'confidential'}
				<div class="space-y-3">
					<div class="bg-muted/40 flex items-center gap-2.5 rounded-lg border p-1.5 pl-3">
						<LockIcon class="text-muted-foreground size-4 shrink-0" />
						<code class="text-muted-foreground flex-1 font-mono text-sm tracking-[0.2em]">
							••••••••••••••••••••••••
						</code>
						<Badge variant="secondary" class="mr-1 shrink-0 text-xs"
							>{m.oauth_clients_credentials_tab_hashed_badge()}</Badge
						>
					</div>

					<div
						class="flex items-start gap-2.5 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3"
					>
						<TriangleAlertIcon
							class="mt-0.5 size-4 shrink-0 text-amber-600 dark:text-amber-400"
						/>
						<p class="text-muted-foreground text-sm">
							{m.oauth_clients_credentials_tab_secret_hint()}
						</p>
					</div>

					<div class="flex justify-end">
						<Button variant="outline" onclick={() => (rotateConfirmOpen = true)}>
							<RefreshCcwIcon /> {m.oauth_clients_credentials_tab_rotate_button()}
						</Button>
					</div>
				</div>
			{:else}
				<div class="flex items-start gap-3 rounded-lg border border-sky-500/20 bg-sky-500/5 p-4">
					<span
						class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-sky-500/10 text-sky-600 dark:text-sky-400"
					>
						<ShieldCheckIcon class="size-5" />
					</span>
					<div class="space-y-1">
						<p class="text-sm font-medium">{m.oauth_clients_credentials_tab_no_secret_title()}</p>
						<p class="text-muted-foreground text-sm">
							{m.oauth_clients_credentials_tab_no_secret_description()}
						</p>
					</div>
				</div>
			{/if}
		</Card.Content>
	</Card.Root>
</div>

<!-- Rotate: confirmation -->
<Dialog.Root bind:open={rotateConfirmOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>{m.oauth_clients_credentials_tab_rotate_dialog_title()}</Dialog.Title>
			<Dialog.Description>
				{m.oauth_clients_credentials_tab_rotate_dialog_description()}
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" disabled={rotating} onclick={() => (rotateConfirmOpen = false)}>
				{m.oauth_clients_action_cancel()}
			</Button>
			<Button disabled={rotating} onclick={confirmRotate}>
				{#if rotating}<LoadingSpinner />{/if}
				{m.oauth_clients_credentials_tab_rotate_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- Rotate: one-time secret reveal -->
<Dialog.Root bind:open={secretRevealOpen}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>{m.oauth_clients_credentials_tab_secret_reveal_title()}</Dialog.Title>
			<Dialog.Description>
				{m.oauth_clients_credentials_tab_secret_reveal_description()}
			</Dialog.Description>
		</Dialog.Header>

		{#if rotatedSecret}
			<div class="space-y-1.5">
				<p class="text-sm font-medium">{m.oauth_clients_credentials_tab_secret_title()}</p>
				<div
					class="flex items-center gap-2.5 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-1.5 pl-3"
				>
					<KeyRoundIcon class="size-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
					<code class="flex-1 truncate font-mono text-sm">{rotatedSecret}</code>
					<Button
						variant="ghost"
						size="icon"
						class="text-muted-foreground hover:text-foreground size-8 shrink-0"
						onclick={() =>
							copyValue(rotatedSecret!, m.oauth_clients_credentials_tab_secret_title(), () => {
								secretCopied = true;
								setTimeout(() => (secretCopied = false), 1500);
							})}
					>
						{#if secretCopied}<CheckIcon class="text-emerald-600 dark:text-emerald-400" />{:else}
							<CopyIcon />
						{/if}
					</Button>
				</div>
			</div>

			<div class="text-muted-foreground bg-muted/50 flex items-start gap-2 rounded-md p-3 text-xs">
				<InfoIcon class="mt-0.5 size-4 shrink-0" />
				<span>{m.oauth_clients_credentials_tab_secret_reveal_update_hint()}</span>
			</div>
		{/if}

		<Dialog.Footer>
			<Button onclick={() => (secretRevealOpen = false)}
				>{m.oauth_clients_credentials_tab_secret_reveal_done_button()}</Button
			>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

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
			toast.success(`${label} copied to clipboard`);
		} catch {
			toast.error('Could not copy to clipboard');
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
		toast.success('Client secret rotated');
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
						Tenant ID
					</Card.Title>
					<Card.Description>
						Identifies the tenant (issuer) this client belongs to.
					</Card.Description>
				</div>
				<Badge
					variant="outline"
					class="shrink-0 gap-1 border-violet-500/20 bg-violet-500/5 text-violet-700 dark:text-violet-400"
				>
					<Building2Icon class="size-3" />
					Tenant
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
						copyValue(tenantId, 'Tenant ID', () => {
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
						Client ID
					</Card.Title>
					<Card.Description>Public identifier sent in the authorization request.</Card.Description>
				</div>
				<Badge
					variant="outline"
					class="shrink-0 gap-1 border-sky-500/20 bg-sky-500/5 text-sky-700 dark:text-sky-400"
				>
					<GlobeIcon class="size-3" />
					Public
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
						copyValue(client.client_id, 'Client ID', () => {
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
						Client secret
					</Card.Title>
					<Card.Description>Authenticates the client at the token endpoint.</Card.Description>
				</div>
				{#if client.client_type === 'confidential'}
					<Badge
						variant="outline"
						class="shrink-0 gap-1 border-emerald-500/20 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400"
					>
						<ShieldCheckIcon class="size-3" />
						Configured
					</Badge>
				{:else}
					<Badge
						variant="outline"
						class="shrink-0 gap-1 border-sky-500/20 bg-sky-500/5 text-sky-700 dark:text-sky-400"
					>
						<GlobeIcon class="size-3" />
						Public client
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
						<Badge variant="secondary" class="mr-1 shrink-0 text-xs">Hashed</Badge>
					</div>

					<div
						class="flex items-start gap-2.5 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3"
					>
						<TriangleAlertIcon
							class="mt-0.5 size-4 shrink-0 text-amber-600 dark:text-amber-400"
						/>
						<p class="text-muted-foreground text-sm">
							The secret is shown only once, at creation, and stored hashed — it can't be displayed
							again. Rotating issues a new secret and immediately invalidates the old one.
						</p>
					</div>

					<div class="flex justify-end">
						<Button variant="outline" onclick={() => (rotateConfirmOpen = true)}>
							<RefreshCcwIcon /> Rotate secret
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
						<p class="text-sm font-medium">No secret required</p>
						<p class="text-muted-foreground text-sm">
							Public clients authenticate with PKCE on the authorization code flow instead of a
							stored secret.
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
			<Dialog.Title>Rotate the client secret?</Dialog.Title>
			<Dialog.Description>
				A new secret is generated and the current one stops working immediately. Any service still
				using the old secret will fail to authenticate until you update it. This can't be undone.
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" disabled={rotating} onclick={() => (rotateConfirmOpen = false)}>
				Cancel
			</Button>
			<Button disabled={rotating} onclick={confirmRotate}>
				{#if rotating}<LoadingSpinner />{/if}
				Rotate secret
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- Rotate: one-time secret reveal -->
<Dialog.Root bind:open={secretRevealOpen}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>New client secret — save it now</Dialog.Title>
			<Dialog.Description>
				This is the only time the new secret is shown. Store it somewhere safe; it can't be
				retrieved again (only rotated).
			</Dialog.Description>
		</Dialog.Header>

		{#if rotatedSecret}
			<div class="space-y-1.5">
				<p class="text-sm font-medium">Client secret</p>
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
							copyValue(rotatedSecret!, 'Client secret', () => {
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
				<span>Update every service that authenticates as this client with the new secret.</span>
			</div>
		{/if}

		<Dialog.Footer>
			<Button onclick={() => (secretRevealOpen = false)}>I've saved it — done</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

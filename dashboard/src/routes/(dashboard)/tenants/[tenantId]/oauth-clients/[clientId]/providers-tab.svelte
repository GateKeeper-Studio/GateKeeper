<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import { env } from '$env/dynamic/public';

	import CopyIcon from '@lucide/svelte/icons/copy';
	import InfoIcon from '@lucide/svelte/icons/info';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SaveIcon from '@lucide/svelte/icons/save';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import GithubLogo from '$lib/components/icons/github-logo.svelte';
	import GoogleLogo from '$lib/components/icons/google-logo.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import {
		createIdentityProvider,
		deleteIdentityProvider,
		listIdentityProviders,
		updateIdentityProvider,
		type IdentityProvider
	} from '$lib/services/admin/identity-providers';

	type Props = { tenantId: string; clientId: string; externalLoginOnly: boolean };
	let { tenantId, clientId, externalLoginOnly }: Props = $props();

	const apiBase = env.PUBLIC_API_BASE_URL ?? 'http://localhost:8080';
	const PROVIDERS = [
		{ value: 'google', label: 'Google' },
		{ value: 'github', label: 'GitHub' }
	] as const;

	const queryClient = useQueryClient();
	const queryKey = $derived(['identity-providers', tenantId, clientId]);

	const providersQuery = createQuery(() => ({
		queryKey: ['identity-providers', tenantId, clientId],
		enabled: !!tenantId && !!clientId,
		queryFn: async () => {
			const [data, err] = await listIdentityProviders(tenantId, clientId);
			if (err) throw err;
			return data;
		}
	}));

	// The IdP refuses to reach this state through the client form or by removing the
	// last active provider — but a client configured before the flag existed, or one
	// whose providers were disabled out-of-band, can still land here with no way in.
	const strandedWithoutProvider = $derived(
		externalLoginOnly && (providersQuery.data ?? []).every((p) => !p.isActive)
	);

	// ── Add / edit dialog ──────────────────────────────────────────────────────
	let dialogOpen = $state(false);
	let editing = $state<IdentityProvider | null>(null);
	let saving = $state(false);

	let provider = $state('google');
	let providerClientId = $state('');
	let clientSecret = $state('');
	let redirectUri = $state('');
	let isActive = $state(true);

	const suggestedRedirect = $derived(`${apiBase}/v1/auth/oauth-provider/${provider}/callback`);

	function providerLabel(value: string) {
		return PROVIDERS.find((p) => p.value === value)?.label ?? value;
	}

	function openCreate() {
		editing = null;
		provider = 'google';
		providerClientId = '';
		clientSecret = '';
		redirectUri = `${apiBase}/v1/auth/oauth-provider/google/callback`;
		isActive = true;
		dialogOpen = true;
	}

	function openEdit(p: IdentityProvider) {
		editing = p;
		provider = p.provider;
		providerClientId = p.providerClientId;
		clientSecret = '';
		redirectUri = p.redirectUri;
		isActive = p.isActive;
		dialogOpen = true;
	}

	// On create, keep the redirect URI in step with the chosen provider unless the
	// admin has typed a custom one.
	function selectProvider(next: string) {
		const wasSuggested = redirectUri === '' || redirectUri === suggestedRedirect;
		provider = next;
		if (!editing && wasSuggested) {
			redirectUri = `${apiBase}/v1/auth/oauth-provider/${next}/callback`;
		}
	}

	async function copyCallback() {
		try {
			await navigator.clipboard.writeText(suggestedRedirect);
			toast.success(m.oauth_clients_providers_callback_copied());
		} catch {
			toast.error(m.oauth_clients_toast_copy_error());
		}
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();

		if (!providerClientId.trim() || !redirectUri.trim()) {
			toast.error(m.oauth_clients_providers_client_id_redirect_required());
			return;
		}
		if (!editing && !clientSecret.trim()) {
			toast.error(m.oauth_clients_providers_secret_required());
			return;
		}

		saving = true;
		const input = {
			provider,
			clientId: providerClientId.trim(),
			clientSecret: clientSecret.trim() ? clientSecret.trim() : undefined,
			redirectUri: redirectUri.trim(),
			isActive
		};

		const [err] = editing
			? await updateIdentityProvider(tenantId, clientId, editing.id, input)
			: await createIdentityProvider(tenantId, clientId, input);
		saving = false;

		if (err) {
			toast.error(err.data?.message || err.data?.error || m.oauth_clients_providers_save_error());
			return;
		}

		toast.success(
			editing ? m.oauth_clients_providers_updated() : m.oauth_clients_providers_added()
		);
		dialogOpen = false;
		queryClient.invalidateQueries({ queryKey });
	}

	// ── Delete confirmation ────────────────────────────────────────────────────
	let deleteTarget = $state<IdentityProvider | null>(null);
	let deleting = $state(false);

	async function confirmDelete() {
		if (!deleteTarget) return;
		deleting = true;
		const [err] = await deleteIdentityProvider(tenantId, clientId, deleteTarget.id);
		deleting = false;

		if (err) {
			toast.error(err.data?.message || m.oauth_clients_providers_delete_error());
			return;
		}

		toast.success(m.oauth_clients_providers_removed());
		deleteTarget = null;
		queryClient.invalidateQueries({ queryKey });
	}
</script>

<Card.Root>
	<Card.Header class="flex flex-row items-start justify-between gap-3">
		<div>
			<Card.Title>{m.oauth_clients_providers_title()}</Card.Title>
			<Card.Description>
				{m.oauth_clients_providers_description()}
			</Card.Description>
		</div>
		<Button size="sm" onclick={openCreate}>
			<PlusIcon class="size-4" />
			{m.oauth_clients_providers_add_button()}
		</Button>
	</Card.Header>
	<Card.Content>
		{#if externalLoginOnly}
			<p
				class="mb-3 rounded-md border px-3 py-2 text-sm"
				class:border-destructive={strandedWithoutProvider}
				class:text-destructive={strandedWithoutProvider}
				class:text-muted-foreground={!strandedWithoutProvider}
			>
				{strandedWithoutProvider
					? m.oauth_clients_providers_external_only_stranded()
					: m.oauth_clients_providers_external_only_notice()}
			</p>
		{/if}

		{#if providersQuery.isPending}
			<div class="flex items-center gap-2 text-muted-foreground">
				<LoadingSpinner />
				{m.oauth_clients_providers_loading()}
			</div>
		{:else if providersQuery.isError}
			<p class="text-sm text-destructive">{providersQuery.error.message}</p>
		{:else if (providersQuery.data ?? []).length === 0}
			<p class="text-sm text-muted-foreground">
				{m.oauth_clients_providers_empty()}
			</p>
		{:else}
			<ul class="divide-y rounded-lg border">
				{#each providersQuery.data ?? [] as p (p.id)}
					<li class="flex items-center gap-3 px-3 py-2.5">
						<span
							class="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground"
						>
							{#if p.provider === 'github'}
								<GithubLogo size={20} />
							{:else if p.provider === 'google'}
								<GoogleLogo size={20} />
							{/if}
						</span>
						<div class="min-w-0 flex-1">
							<p class="text-sm font-medium">{providerLabel(p.provider)}</p>
							<p
								class="truncate font-mono text-xs text-muted-foreground"
								title={p.providerClientId}
							>
								{p.providerClientId}
							</p>
						</div>
						<Badge variant={p.isActive ? 'outline' : 'secondary'} class="shrink-0">
							{p.isActive ? m.oauth_clients_status_active() : m.oauth_clients_status_inactive()}
						</Badge>
						<Button
							variant="ghost"
							size="icon"
							onclick={() => openEdit(p)}
							aria-label={m.oauth_clients_providers_edit_aria()}
						>
							<PencilIcon class="size-4" />
						</Button>
						<Button
							variant="ghost"
							size="icon"
							onclick={() => (deleteTarget = p)}
							aria-label={m.oauth_clients_providers_delete_aria()}
						>
							<Trash2Icon class="size-4 text-destructive" />
						</Button>
					</li>
				{/each}
			</ul>
		{/if}
	</Card.Content>
</Card.Root>

<!-- Add / edit provider dialog -->
<Dialog.Root bind:open={dialogOpen}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title
				>{editing
					? m.oauth_clients_providers_edit_title()
					: m.oauth_clients_providers_add_title()}</Dialog.Title
			>
			<Dialog.Description>
				{m.oauth_clients_providers_dialog_description()}
			</Dialog.Description>
		</Dialog.Header>

		<form onsubmit={submit} class="space-y-4">
			<div class="space-y-2">
				<Label>{m.oauth_clients_providers_provider_label()}</Label>
				<Select.Root
					type="single"
					value={provider}
					onValueChange={selectProvider}
					disabled={!!editing}
				>
					<Select.Trigger class="w-full">{providerLabel(provider)}</Select.Trigger>
					<Select.Content>
						{#each PROVIDERS as p (p.value)}
							<Select.Item value={p.value} label={p.label}>{p.label}</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			<div class="space-y-2">
				<Label for="provider-client-id">{m.oauth_clients_credentials_tab_client_id_title()}</Label>
				<Input
					id="provider-client-id"
					bind:value={providerClientId}
					placeholder={m.oauth_clients_providers_client_id_placeholder()}
				/>
			</div>

			<div class="space-y-2">
				<Label for="provider-client-secret">{m.oauth_clients_credentials_tab_secret_title()}</Label>
				<Input
					id="provider-client-secret"
					type="password"
					bind:value={clientSecret}
					placeholder={editing
						? m.oauth_clients_providers_keep_secret_placeholder()
						: m.oauth_clients_providers_secret_placeholder()}
				/>
			</div>

			<div class="space-y-2">
				<Label for="provider-redirect-uri">{m.oauth_clients_providers_redirect_uri_label()}</Label>
				<Input
					id="provider-redirect-uri"
					bind:value={redirectUri}
					placeholder={suggestedRedirect}
				/>
				<div
					class="flex items-start gap-2 rounded-md bg-muted/50 p-3 text-xs text-muted-foreground"
				>
					<InfoIcon class="mt-0.5 size-4 shrink-0" />
					<span class="min-w-0">
						{m.oauth_clients_providers_callback_hint()}
						<button
							type="button"
							class="ml-1 inline-flex items-center gap-1 font-mono break-all underline-offset-2 hover:underline"
							onclick={copyCallback}
						>
							{suggestedRedirect}
							<CopyIcon class="size-3 shrink-0" />
						</button>
					</span>
				</div>
			</div>

			<label class="flex items-center gap-2 text-sm font-medium">
				<Checkbox bind:checked={isActive} />
				{m.oauth_clients_providers_active_label()}
			</label>

			<Dialog.Footer>
				<Button type="button" variant="outline" onclick={() => (dialogOpen = false)}
					>{m.oauth_clients_action_cancel()}</Button
				>
				<Button type="submit" disabled={saving}>
					{#if saving}
						<LoadingSpinner class="mr-1" />
					{:else if editing}
						<SaveIcon />
					{:else}
						<PlusIcon />
					{/if}
					{editing ? m.oauth_clients_action_save_changes() : m.oauth_clients_providers_add_button()}
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>

<!-- Delete confirmation dialog -->
<Dialog.Root open={!!deleteTarget} onOpenChange={(open) => !open && (deleteTarget = null)}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>{m.oauth_clients_providers_remove_title()}</Dialog.Title>
			<Dialog.Description>
				{#if deleteTarget}
					{m.oauth_clients_providers_remove_description({
						provider: providerLabel(deleteTarget.provider)
					})}
				{/if}
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" onclick={() => (deleteTarget = null)}
				>{m.oauth_clients_action_cancel()}</Button
			>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{#if deleting}<LoadingSpinner class="mr-1" />{/if}
				{m.oauth_clients_providers_remove_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

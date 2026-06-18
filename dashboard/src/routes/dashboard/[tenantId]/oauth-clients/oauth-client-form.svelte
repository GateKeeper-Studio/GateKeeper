<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { toast } from 'svelte-sonner';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';

	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import InfoIcon from '@lucide/svelte/icons/info';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import LockIcon from '@lucide/svelte/icons/lock';

	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import { listEmailProviders } from '$lib/services/admin/email-providers';
	import {
		createOAuthClient,
		updateOAuthClient,
		type OAuthClientView,
		type CreatedOAuthClient
	} from '$lib/services/admin/oauth-clients';
	import {
		KNOWN_SCOPES,
		KNOWN_GRANT_TYPES,
		scopeLabel,
		grantLabel,
		clientTypeLabel
	} from './display.js';
	import { oauthClientFormSchema } from './schema.js';
	import StringListInput from './string-list-input.svelte';

	type Props = {
		mode: 'create' | 'edit';
		tenantId: string;
		client?: OAuthClientView;
	};

	let { mode, tenantId, client }: Props = $props();

	const queryClient = useQueryClient();
	const knownScopes = KNOWN_SCOPES as readonly string[];
	const knownGrants = KNOWN_GRANT_TYPES as readonly string[];

	// ---- form state (seeded from `client` in edit mode) ----------------------
	let name = $state(client?.name ?? '');
	let clientType = $state<'public' | 'confidential'>(
		(client?.client_type as 'public' | 'confidential') ?? 'public'
	);
	let redirectUris = $state<string[]>(client?.redirect_uris ?? []);
	let postLogoutRedirectUris = $state<string[]>(client?.post_logout_redirect_uris ?? []);

	const initialScopes = client?.allowed_scopes ?? ['openid'];
	let scopeSelections = $state<Record<string, boolean>>(
		Object.fromEntries(knownScopes.map((s) => [s, initialScopes.includes(s)]))
	);
	let customScopes = $state<string[]>(initialScopes.filter((s) => !knownScopes.includes(s)));

	const initialGrants = client?.allowed_grant_types ?? ['authorization_code'];
	let grantSelections = $state<Record<string, boolean>>(
		Object.fromEntries(knownGrants.map((g) => [g, initialGrants.includes(g)]))
	);

	let requirePkce = $state(client?.require_pkce ?? true);
	let requireConsent = $state(client?.require_consent ?? true);
	let mfaRequired = $state(client?.mfa_required ?? false);
	let mfaEmailEnabled = $state(client?.mfa_email_enabled ?? true);
	let mfaTotpEnabled = $state(client?.mfa_totp_enabled ?? true);
	let mfaWebauthnEnabled = $state(client?.mfa_webauthn_enabled ?? true);
	let isActive = $state(client?.is_active ?? true);
	let emailProviderId = $state(client?.email_provider_id ?? '');
	let tosUri = $state(client?.tos_uri ?? '');
	let policyUri = $state(client?.policy_uri ?? '');

	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);

	// Email providers populate the optional "send emails through" select.
	const providersQuery = createQuery(() => ({
		queryKey: ['email-providers', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listEmailProviders(tenantId);
			if (err) throw err;
			return data.items;
		}
	}));

	// Label shown in the email-provider select trigger, derived from the selection.
	const emailProviderLabel = $derived(
		emailProviderId
			? (providersQuery.data?.find((p) => p.id === emailProviderId)?.name ?? 'Tenant default')
			: 'Tenant default'
	);

	// ---- created-secret dialog (create mode, confidential clients) -----------
	let createdClient = $state<CreatedOAuthClient | null>(null);
	let secretDialogOpen = $state(false);
	let secretCopied = $state(false);

	async function copy(value: string, label: string) {
		try {
			await navigator.clipboard.writeText(value);
			if (label === 'Client secret') {
				secretCopied = true;
				setTimeout(() => (secretCopied = false), 1500);
			}
			toast.success(`${label} copied to clipboard`);
		} catch {
			toast.error('Could not copy to clipboard');
		}
	}

	let navigatingAway = false;
	function goToCreatedClient() {
		if (navigatingAway) return; // closing the dialog also calls this — run once.
		navigatingAway = true;
		secretDialogOpen = false;
		if (createdClient) {
			goto(
				resolve('/dashboard/[tenantId]/oauth-clients/[clientId]', {
					tenantId,
					clientId: createdClient.id
				})
			);
		}
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const allowedScopes = [...knownScopes.filter((s) => scopeSelections[s]), ...customScopes];
		const allowedGrantTypes = knownGrants.filter((g) => grantSelections[g]);

		const result = validate(oauthClientFormSchema, {
			name: name.trim(),
			clientType,
			redirectUris,
			postLogoutRedirectUris,
			allowedScopes,
			allowedGrantTypes,
			requirePkce,
			requireConsent,
			mfaRequired,
			mfaEmailEnabled,
			mfaTotpEnabled,
			mfaWebauthnEnabled,
			isActive,
			emailProviderId: emailProviderId || null,
			tosUri,
			policyUri
		});

		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		if (mode === 'create') {
			const [data, err] = await createOAuthClient(tenantId, result.data);
			isLoading = false;

			if (err) {
				toast.error(err.data?.message || 'An error occurred while creating the client.');
				return;
			}

			queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId] });

			// Confidential clients return a one-time secret — reveal it before leaving.
			if (data.client_secret) {
				createdClient = data;
				secretDialogOpen = true;
				return;
			}

			toast.success('OAuth client created');
			await goto(
				resolve('/dashboard/[tenantId]/oauth-clients/[clientId]', {
					tenantId,
					clientId: data.id
				})
			);
			return;
		}

		// edit mode
		const [err] = await updateOAuthClient(tenantId, client!.id, result.data);
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || 'An error occurred while updating the client.');
			return;
		}

		toast.success('OAuth client updated');
		queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId] });
		queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId, client!.id] });
		await goto(
			resolve('/dashboard/[tenantId]/oauth-clients/[clientId]', {
				tenantId,
				clientId: client!.id
			})
		);
	}
</script>

<form onsubmit={onSubmit} class="mt-4 grid max-w-[760px] gap-6">
	<!-- Basics -->
	<div class="grid gap-4">
		<div class="space-y-2">
			<Label for="name" class="flex gap-1">
				Name
				<span class="text-red-500">*</span>
			</Label>
			<Input id="name" type="text" placeholder="My web application" bind:value={name} />
			{#if errors.name}<p class="text-sm font-medium text-destructive">{errors.name}</p>{/if}
		</div>

		<div class="space-y-2">
			<Label class="flex gap-1">
				Client type
				{#if mode === 'create'}<span class="text-red-500">*</span>{/if}
			</Label>

			{#if mode === 'create'}
				<div class="inline-flex w-fit items-center rounded-lg bg-muted p-1 text-muted-foreground">
					{#each ['public', 'confidential'] as const as type (type)}
						<button
							type="button"
							onclick={() => (clientType = type)}
							class={cn(
								'inline-flex h-7 items-center rounded-md px-4 text-sm font-medium transition-all',
								clientType === type && 'bg-background text-foreground shadow-sm'
							)}
						>
						{#if type === 'public'}
							<GlobeIcon class="size-4 mr-2" />
						{:else}
							<LockIcon class="size-4 mr-2" />
						{/if}
							{clientTypeLabel(type)}
						</button>
					{/each}
				</div>
				<p class="text-sm text-muted-foreground">
					{#if clientType === 'confidential'}
						Server-side app that can keep a secret. A client secret is generated and shown once at
						creation.
					{:else}
						Browser or native app that can't keep a secret. Authenticates with PKCE — no client
						secret.
					{/if}
				</p>
			{:else}
				<p class="text-sm">
					{clientTypeLabel(clientType)}
					<span class="text-muted-foreground"
						>— the client type can't be changed after creation.</span
					>
				</p>
			{/if}
		</div>
	</div>

	<Separator />

	<!-- Redirect URIs -->
	<div class="grid gap-4">
		<div class="space-y-2">
			<Label class="flex gap-1">
				Redirect URIs
				<span class="text-red-500">*</span>
			</Label>
			<p class="text-sm text-muted-foreground">
				Exact-match URLs the IdP may redirect back to after authorization. Must be absolute; plain
				http is only allowed for localhost.
			</p>
			<StringListInput
				bind:values={redirectUris}
				placeholder="https://app.example.com/callback"
				addLabel="Add redirect URI"
				inputmode="url"
			/>
			{#if errors.redirectUris}
				<p class="text-sm font-medium text-destructive">{errors.redirectUris}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label>Post-logout redirect URIs</Label>
			<p class="text-sm text-muted-foreground">
				Optional — where users may be sent after signing out.
			</p>
			<StringListInput
				bind:values={postLogoutRedirectUris}
				placeholder="https://app.example.com/"
				addLabel="Add post-logout URI"
				inputmode="url"
			/>
		</div>
	</div>

	<Separator />

	<!-- Legal links -->
	<div class="grid gap-4">
		<div class="space-y-2">
			<Label for="tosUri">Terms of Service URL</Label>
			<Input
				id="tosUri"
				type="url"
				inputmode="url"
				placeholder="https://app.example.com/terms"
				bind:value={tosUri}
			/>
			<p class="text-sm text-muted-foreground">
				Optional — shown on the hosted auth pages for this client. Leave blank to use the platform
				terms.
			</p>
			{#if errors.tosUri}
				<p class="text-sm font-medium text-destructive">{errors.tosUri}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label for="policyUri">Privacy Policy URL</Label>
			<Input
				id="policyUri"
				type="url"
				inputmode="url"
				placeholder="https://app.example.com/privacy"
				bind:value={policyUri}
			/>
			<p class="text-sm text-muted-foreground">
				Optional — shown on the hosted auth pages for this client. Leave blank to use the platform
				policy.
			</p>
			{#if errors.policyUri}
				<p class="text-sm font-medium text-destructive">{errors.policyUri}</p>
			{/if}
		</div>
	</div>

	<Separator />

	<!-- Scopes & grants -->
	<div class="grid gap-4">
		<div class="space-y-2">
			<Label class="flex gap-1">
				Allowed scopes
				<span class="text-red-500">*</span>
			</Label>
			<div class="grid gap-2 sm:grid-cols-2">
				{#each knownScopes as scope (scope)}
					<label class="flex items-center gap-2 text-sm">
						<Checkbox bind:checked={scopeSelections[scope]} />
						{scopeLabel(scope)}
						<span class="font-mono text-xs text-muted-foreground">{scope}</span>
					</label>
				{/each}
			</div>
			<div class="space-y-1.5">
				<p class="text-sm text-muted-foreground">Custom scopes</p>
				<StringListInput
					bind:values={customScopes}
					placeholder="custom:scope"
					addLabel="Add custom scope"
				/>
			</div>
			{#if errors.allowedScopes}
				<p class="text-sm font-medium text-destructive">{errors.allowedScopes}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label class="flex gap-1">
				Grant types
				<span class="text-red-500">*</span>
			</Label>
			<div class="grid gap-2 sm:grid-cols-2">
				{#each knownGrants as grant (grant)}
					<label class="flex items-center gap-2 text-sm">
						<Checkbox bind:checked={grantSelections[grant]} />
						{grantLabel(grant)}
						<span class="font-mono text-xs text-muted-foreground">{grant}</span>
					</label>
				{/each}
			</div>
			{#if errors.allowedGrantTypes}
				<p class="text-sm font-medium text-destructive">{errors.allowedGrantTypes}</p>
			{/if}
		</div>
	</div>

	<Separator />

	<!-- Security & delivery -->
	<div class="grid gap-3">
		<span class="text-sm font-medium">Security</span>

		<label class="flex items-center gap-2 text-sm">
			<Checkbox bind:checked={requirePkce} />
			Require PKCE
			<span class="text-muted-foreground">— proof-key for code exchange (recommended).</span>
		</label>

		<label class="flex items-center gap-2 text-sm">
			<Checkbox bind:checked={requireConsent} />
			Require consent
			<span class="text-muted-foreground">— show the consent screen on first authorization.</span>
		</label>

		<label class="flex items-center gap-2 text-sm">
			<Checkbox bind:checked={mfaRequired} />
			Require MFA
			<span class="text-muted-foreground">— require a second factor during sign-in.</span>
		</label>

		<div class="grid gap-2 pl-6 sm:grid-cols-3">
			<label class="flex items-center gap-2 text-sm">
				<Checkbox bind:checked={mfaEmailEnabled} />
				Email code
			</label>
			<label class="flex items-center gap-2 text-sm">
				<Checkbox bind:checked={mfaTotpEnabled} />
				Authenticator app
			</label>
			<label class="flex items-center gap-2 text-sm">
				<Checkbox bind:checked={mfaWebauthnEnabled} />
				Passkey
			</label>
		</div>

		{#if mode === 'edit'}
			<label class="flex items-center gap-2 text-sm">
				<Checkbox bind:checked={isActive} />
				Active
				<span class="text-muted-foreground">— inactive clients can't authorize new sessions.</span>
			</label>
		{/if}

		<div class="mt-2 space-y-2">
			<Label for="emailProvider">Email provider</Label>
			<Select.Root type="single" bind:value={emailProviderId}>
				<Select.Trigger id="emailProvider" class="w-full max-w-sm">
					{emailProviderLabel}
				</Select.Trigger>
				<Select.Content>
					<Select.Item value="" label="Tenant default">Tenant default</Select.Item>
					{#each providersQuery.data ?? [] as provider (provider.id)}
						<Select.Item value={provider.id} label={provider.name}>{provider.name}</Select.Item>
					{/each}
				</Select.Content>
			</Select.Root>
			<p class="text-sm text-muted-foreground">
				Which tenant SMTP provider sends this client's emails. Defaults to the tenant's provider.
			</p>
		</div>
	</div>

	<div class="flex justify-end">
		<Button type="submit" disabled={isLoading} class="relative w-fit">
			{#if isLoading}<LoadingSpinner class="absolute left-3" />{/if}
			{#if mode === 'create'}
				{isLoading ? 'Creating client…' : 'Create client'}
			{:else}
				{isLoading ? 'Saving changes…' : 'Save changes'}
			{/if}
		</Button>
	</div>
</form>

<!-- One-time client secret reveal (confidential clients only) -->
<Dialog.Root
	bind:open={secretDialogOpen}
	onOpenChange={(open) => {
		if (!open) goToCreatedClient();
	}}
>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>Client created — save your secret</Dialog.Title>
			<Dialog.Description>
				This is the only time the client secret is shown. Store it somewhere safe; it can't be
				retrieved again (only rotated).
			</Dialog.Description>
		</Dialog.Header>

		{#if createdClient}
			<div class="space-y-3">
				<div class="space-y-1.5">
					<p class="text-sm font-medium">Client ID</p>
					<div class="flex items-center gap-2">
						<code class="flex-1 truncate rounded-md bg-muted px-3 py-2 font-mono text-sm">
							{createdClient.client_id}
						</code>
						<Button
							variant="outline"
							size="icon"
							onclick={() => copy(createdClient!.client_id, 'Client ID')}
						>
							<CopyIcon />
						</Button>
					</div>
				</div>

				<div class="space-y-1.5">
					<p class="text-sm font-medium">Client secret</p>
					<div class="flex items-center gap-2">
						<code class="flex-1 truncate rounded-md bg-muted px-3 py-2 font-mono text-sm">
							{createdClient.client_secret}
						</code>
						<Button
							variant="outline"
							size="icon"
							onclick={() => copy(createdClient!.client_secret!, 'Client secret')}
						>
							{#if secretCopied}<CheckIcon />{:else}<CopyIcon />{/if}
						</Button>
					</div>
				</div>

				<div
					class="flex items-start gap-2 rounded-md bg-muted/50 p-3 text-xs text-muted-foreground"
				>
					<InfoIcon class="mt-0.5 size-4 shrink-0" />
					<span
						>Anyone with this secret can authenticate as this client. Treat it like a password.</span
					>
				</div>
			</div>
		{/if}

		<Dialog.Footer>
			<Button onclick={goToCreatedClient}>I've saved it — continue</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

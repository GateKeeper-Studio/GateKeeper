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
	import SaveIcon from '@lucide/svelte/icons/save';

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
	import { m } from '$lib/paraglide/messages.js';

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
	let externalLoginOnly = $state(client?.external_login_only ?? false);
	let magicLinkEnabled = $state(client?.magic_link_enabled ?? false);
	let mfaRequired = $state(client?.mfa_required ?? false);
	let mfaEmailEnabled = $state(client?.mfa_email_enabled ?? true);
	let mfaTotpEnabled = $state(client?.mfa_totp_enabled ?? true);
	let mfaWebauthnEnabled = $state(client?.mfa_webauthn_enabled ?? true);
	let isActive = $state(client?.is_active ?? true);
	let emailProviderId = $state(client?.email_provider_id ?? '');
	let tosUri = $state(client?.tos_uri ?? '');
	let policyUri = $state(client?.policy_uri ?? '');
	let initiateLoginUri = $state(client?.initiate_login_uri ?? '');

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
			? (providersQuery.data?.find((p) => p.id === emailProviderId)?.name ??
					m.oauth_clients_tenant_default())
			: m.oauth_clients_tenant_default()
	);

	// ---- created-secret dialog (create mode, confidential clients) -----------
	let createdClient = $state<CreatedOAuthClient | null>(null);
	let secretDialogOpen = $state(false);
	let secretCopied = $state(false);

	async function copy(value: string, label: string) {
		try {
			await navigator.clipboard.writeText(value);
			if (label === m.oauth_clients_credentials_tab_secret_title()) {
				secretCopied = true;
				setTimeout(() => (secretCopied = false), 1500);
			}
			toast.success(m.oauth_clients_toast_copied_with_label({ label }));
		} catch {
			toast.error(m.oauth_clients_toast_copy_error());
		}
	}

	let navigatingAway = false;
	function goToCreatedClient() {
		if (navigatingAway) return; // closing the dialog also calls this — run once.
		navigatingAway = true;
		secretDialogOpen = false;
		if (createdClient) {
			goto(
				resolve('/(dashboard)/tenants/[tenantId]/oauth-clients/[clientId]', {
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
			externalLoginOnly,
			magicLinkEnabled,
			mfaRequired,
			mfaEmailEnabled,
			mfaTotpEnabled,
			mfaWebauthnEnabled,
			isActive,
			emailProviderId: emailProviderId || null,
			tosUri,
			policyUri,
			initiateLoginUri
		});

		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		if (mode === 'create') {
			// Appearance and languages are configured later in the client's
			// "Login page" tab — default to English-only until then.
			const [data, err] = await createOAuthClient(tenantId, {
				...result.data,
				loginAppearance: null,
				allowedLanguages: ['en'],
				defaultLanguage: 'en'
			});
			isLoading = false;

			if (err) {
				toast.error(err.data?.message || m.oauth_clients_form_create_error());
				return;
			}

			queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId] });

			// Confidential clients return a one-time secret — reveal it before leaving.
			if (data.client_secret) {
				createdClient = data;
				secretDialogOpen = true;
				return;
			}

			toast.success(m.oauth_clients_form_create_success());
			await goto(
				resolve('/(dashboard)/tenants/[tenantId]/oauth-clients/[clientId]', {
					tenantId,
					clientId: data.id
				})
			);
			return;
		}

		// edit mode — preserve the login-page appearance/languages (managed in
		// their own tab) so this full-replacement PUT doesn't wipe them.
		const [err] = await updateOAuthClient(tenantId, client!.id, {
			...result.data,
			loginAppearance: client!.login_appearance ?? null,
			allowedLanguages: client!.allowed_languages,
			defaultLanguage: client!.default_language
		});
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || m.oauth_clients_form_update_error());
			return;
		}

		toast.success(m.oauth_clients_form_update_success());
		queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId] });
		queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId, client!.id] });
		await goto(
			resolve('/(dashboard)/tenants/[tenantId]/oauth-clients/[clientId]', {
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
				{m.oauth_clients_form_name_label()}
				<span class="text-red-500">*</span>
			</Label>
			<Input
				id="name"
				type="text"
				placeholder={m.oauth_clients_form_name_placeholder()}
				bind:value={name}
			/>
			{#if errors.name}<p class="text-sm font-medium text-destructive">{errors.name}</p>{/if}
		</div>

		<div class="space-y-2">
			<Label class="flex gap-1">
				{m.oauth_clients_form_client_type_label()}
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
								<GlobeIcon class="mr-2 size-4" />
							{:else}
								<LockIcon class="mr-2 size-4" />
							{/if}
							{clientTypeLabel(type)}
						</button>
					{/each}
				</div>
				<p class="text-sm text-muted-foreground">
					{#if clientType === 'confidential'}
						{m.oauth_clients_form_confidential_description()}
					{:else}
						{m.oauth_clients_form_public_description()}
					{/if}
				</p>
			{:else}
				<p class="text-sm">
					{clientTypeLabel(clientType)}
					<span class="text-muted-foreground">{m.oauth_clients_form_client_type_locked()}</span>
				</p>
			{/if}
		</div>
	</div>

	<Separator />

	<!-- Redirect URIs -->
	<div class="grid gap-4">
		<div class="space-y-2">
			<Label class="flex gap-1">
				{m.oauth_clients_form_redirect_uris_label()}
				<span class="text-red-500">*</span>
			</Label>
			<p class="text-sm text-muted-foreground">
				{m.oauth_clients_form_redirect_uris_description()}
			</p>
			<StringListInput
				bind:values={redirectUris}
				placeholder="https://app.example.com/callback"
				addLabel={m.oauth_clients_form_add_redirect_uri()}
				inputmode="url"
			/>
			{#if errors.redirectUris}
				<p class="text-sm font-medium text-destructive">{errors.redirectUris}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label>{m.oauth_clients_form_post_logout_redirect_uris_label()}</Label>
			<p class="text-sm text-muted-foreground">
				{m.oauth_clients_form_post_logout_redirect_uris_description()}
			</p>
			<StringListInput
				bind:values={postLogoutRedirectUris}
				placeholder="https://app.example.com/"
				addLabel={m.oauth_clients_form_add_post_logout_uri()}
				inputmode="url"
			/>
		</div>
	</div>

	<Separator />

	<!-- Sign-in recovery -->
	<div class="grid gap-4">
		<div class="space-y-2">
			<Label for="initiateLoginUri">{m.oauth_clients_form_initiate_login_uri_label()}</Label>
			<Input
				id="initiateLoginUri"
				type="url"
				inputmode="url"
				placeholder="https://app.example.com/auth/sign-in"
				bind:value={initiateLoginUri}
			/>
			<p class="text-sm text-muted-foreground">
				{m.oauth_clients_form_initiate_login_uri_description()}
			</p>
			{#if errors.initiateLoginUri}
				<p class="text-sm font-medium text-destructive">{errors.initiateLoginUri}</p>
			{/if}
		</div>
	</div>

	<Separator />

	<!-- Legal links -->
	<div class="grid gap-4">
		<div class="space-y-2">
			<Label for="tosUri">{m.oauth_clients_form_terms_url_label()}</Label>
			<Input
				id="tosUri"
				type="url"
				inputmode="url"
				placeholder="https://app.example.com/terms"
				bind:value={tosUri}
			/>
			<p class="text-sm text-muted-foreground">
				{m.oauth_clients_form_terms_url_description()}
			</p>
			{#if errors.tosUri}
				<p class="text-sm font-medium text-destructive">{errors.tosUri}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label for="policyUri">{m.oauth_clients_form_privacy_url_label()}</Label>
			<Input
				id="policyUri"
				type="url"
				inputmode="url"
				placeholder="https://app.example.com/privacy"
				bind:value={policyUri}
			/>
			<p class="text-sm text-muted-foreground">
				{m.oauth_clients_form_privacy_url_description()}
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
				{m.oauth_clients_form_allowed_scopes_label()}
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
				<p class="text-sm text-muted-foreground">{m.oauth_clients_form_custom_scopes_label()}</p>
				<StringListInput
					bind:values={customScopes}
					placeholder="custom:scope"
					addLabel={m.oauth_clients_form_add_custom_scope()}
				/>
			</div>
			{#if errors.allowedScopes}
				<p class="text-sm font-medium text-destructive">{errors.allowedScopes}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label class="flex gap-1">
				{m.oauth_clients_form_grant_types_label()}
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
		<span class="text-sm font-medium">{m.oauth_clients_form_security_section()}</span>

		<label class="flex items-center gap-2 text-sm">
			<Checkbox bind:checked={requirePkce} />
			{m.oauth_clients_form_require_pkce_label()}
			<span class="text-muted-foreground">{m.oauth_clients_form_require_pkce_description()}</span>
		</label>

		<label class="flex items-center gap-2 text-sm">
			<Checkbox bind:checked={requireConsent} />
			{m.oauth_clients_form_require_consent_label()}
			<span class="text-muted-foreground">{m.oauth_clients_form_require_consent_description()}</span
			>
		</label>

		<label class="flex items-start gap-2 text-sm">
			<Checkbox bind:checked={externalLoginOnly} class="mt-0.5" />
			<span>
				{m.oauth_clients_form_external_login_only_label()}
				<span class="text-muted-foreground">
					{m.oauth_clients_form_external_login_only_description()}
				</span>
			</span>
		</label>

		{#if externalLoginOnly}
			<p class="pl-6 text-sm text-muted-foreground">
				{m.oauth_clients_form_external_login_only_hint()}
			</p>
		{/if}

		<!-- A sign-in link is not an external provider: an external-only client never
		     offers one, so the box is disabled while that is on. -->
		<label class="flex items-start gap-2 text-sm">
			<Checkbox bind:checked={magicLinkEnabled} disabled={externalLoginOnly} class="mt-0.5" />
			<span>
				{m.oauth_clients_form_magic_link_label()}
				<span class="text-muted-foreground">
					{m.oauth_clients_form_magic_link_description()}
				</span>
			</span>
		</label>

		<label class="flex items-center gap-2 text-sm">
			<Checkbox bind:checked={mfaRequired} />
			{m.oauth_clients_form_require_mfa_label()}
			<span class="text-muted-foreground">{m.oauth_clients_form_require_mfa_description()}</span>
		</label>

		<div class="grid gap-2 pl-6 sm:grid-cols-3">
			<label class="flex items-center gap-2 text-sm">
				<Checkbox bind:checked={mfaEmailEnabled} />
				{m.oauth_clients_form_mfa_email_label()}
			</label>
			<label class="flex items-center gap-2 text-sm">
				<Checkbox bind:checked={mfaTotpEnabled} />
				{m.oauth_clients_form_mfa_app_label()}
			</label>
			<label class="flex items-center gap-2 text-sm">
				<Checkbox bind:checked={mfaWebauthnEnabled} />
				{m.oauth_clients_form_mfa_passkey_label()}
			</label>
		</div>

		{#if mode === 'edit'}
			<label class="flex items-center gap-2 text-sm">
				<Checkbox bind:checked={isActive} />
				{m.oauth_clients_status_active()}
				<span class="text-muted-foreground">{m.oauth_clients_form_active_description()}</span>
			</label>
		{/if}

		<div class="mt-2 space-y-2">
			<Label for="emailProvider">{m.oauth_clients_form_email_provider_label()}</Label>
			<Select.Root type="single" bind:value={emailProviderId}>
				<Select.Trigger id="emailProvider" class="w-full max-w-sm">
					{emailProviderLabel}
				</Select.Trigger>
				<Select.Content>
					<Select.Item value="" label={m.oauth_clients_tenant_default()}
						>{m.oauth_clients_tenant_default()}</Select.Item
					>
					{#each providersQuery.data ?? [] as provider (provider.id)}
						<Select.Item value={provider.id} label={provider.name}>{provider.name}</Select.Item>
					{/each}
				</Select.Content>
			</Select.Root>
			<p class="text-sm text-muted-foreground">
				{m.oauth_clients_form_email_provider_description()}
			</p>
		</div>
	</div>

	<div class="flex justify-end">
		<Button type="submit" disabled={isLoading} class="w-fit">
			{#if isLoading}
				<LoadingSpinner class="size-4" />
			{:else if mode === 'edit'}
				<SaveIcon />
			{/if}
			{#if mode === 'create'}
				{isLoading ? m.oauth_clients_form_creating_client() : m.oauth_clients_form_create_client()}
			{:else}
				{isLoading ? m.roles_form_saving_loading() : m.oauth_clients_action_save_changes()}
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
			<Dialog.Title>{m.oauth_clients_form_secret_dialog_title()}</Dialog.Title>
			<Dialog.Description>
				{m.oauth_clients_form_secret_dialog_description()}
			</Dialog.Description>
		</Dialog.Header>

		{#if createdClient}
			<div class="space-y-3">
				<div class="space-y-1.5">
					<p class="text-sm font-medium">{m.oauth_clients_credentials_tab_client_id_title()}</p>
					<div class="flex items-center gap-2">
						<code class="flex-1 truncate rounded-md bg-muted px-3 py-2 font-mono text-sm">
							{createdClient.client_id}
						</code>
						<Button
							variant="outline"
							size="icon"
							onclick={() =>
								copy(createdClient!.client_id, m.oauth_clients_credentials_tab_client_id_title())}
						>
							<CopyIcon />
						</Button>
					</div>
				</div>

				<div class="space-y-1.5">
					<p class="text-sm font-medium">{m.oauth_clients_credentials_tab_secret_title()}</p>
					<div class="flex items-center gap-2">
						<code class="flex-1 truncate rounded-md bg-muted px-3 py-2 font-mono text-sm">
							{createdClient.client_secret}
						</code>
						<Button
							variant="outline"
							size="icon"
							onclick={() =>
								copy(createdClient!.client_secret!, m.oauth_clients_credentials_tab_secret_title())}
						>
							{#if secretCopied}<CheckIcon />{:else}<CopyIcon />{/if}
						</Button>
					</div>
				</div>

				<div
					class="flex items-start gap-2 rounded-md bg-muted/50 p-3 text-xs text-muted-foreground"
				>
					<InfoIcon class="mt-0.5 size-4 shrink-0" />
					<span>{m.oauth_clients_form_secret_warning()}</span>
				</div>
			</div>
		{/if}

		<Dialog.Footer>
			<Button onclick={goToCreatedClient}>{m.oauth_clients_form_secret_continue()}</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

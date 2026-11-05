<script lang="ts">
	import { resolve } from '$app/paths';
	import { toast } from 'svelte-sonner';

	import ActivityIcon from '@lucide/svelte/icons/activity';
	import CalendarPlusIcon from '@lucide/svelte/icons/calendar-plus';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import CircleUserIcon from '@lucide/svelte/icons/circle-user';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import KeyIcon from '@lucide/svelte/icons/key';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import LinkIcon from '@lucide/svelte/icons/link';
	import LockIcon from '@lucide/svelte/icons/lock';
	import MailIcon from '@lucide/svelte/icons/mail';
	import ScrollTextIcon from '@lucide/svelte/icons/scroll-text';
	import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import ShieldOffIcon from '@lucide/svelte/icons/shield-off';
	import SmartphoneIcon from '@lucide/svelte/icons/smartphone';
	import UserCheckIcon from '@lucide/svelte/icons/user-check';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { cn } from '$lib/utils.js';

	import { type OAuthClientView } from '$lib/services/admin/oauth-clients';
	import { clientTypeLabel, formatClientDate, grantLabel, scopeLabel } from '../display.js';
	import EmailProviderSelect from './email-provider-select.svelte';
	import { m } from '$lib/paraglide/messages.js';

	type Props = { client: OAuthClientView; tenantId: string };
	type IconComponent = typeof ActivityIcon;
	let { client, tenantId }: Props = $props();

	async function copyUri(uri: string) {
		try {
			await navigator.clipboard.writeText(uri);
			toast.success(m.oauth_clients_overview_uri_copied());
		} catch {
			toast.error(m.oauth_clients_toast_copy_error());
		}
	}

	function enabledMfaMethodCount(client: OAuthClientView): number {
		return [client.mfa_email_enabled, client.mfa_totp_enabled, client.mfa_webauthn_enabled].filter(
			Boolean
		).length;
	}

	function mfaSummary(client: OAuthClientView): string {
		if (!client.mfa_required) return m.oauth_clients_overview_not_required();
		const count = enabledMfaMethodCount(client);
		return count === 1
			? m.oauth_clients_overview_method_count_one({ count })
			: m.oauth_clients_overview_method_count_other({ count });
	}

	const editHref = $derived(
		resolve('/(dashboard)/tenants/[tenantId]/oauth-clients/[clientId]/edit', {
			tenantId,
			clientId: client.id
		})
	);
</script>

{#snippet sectionTitle(title: string, description: string, Icon: IconComponent)}
	<Card.Header>
		<Card.Title class="flex items-center gap-2">
			<Icon class="size-4 text-muted-foreground" />
			{title}
		</Card.Title>
		<Card.Description>{description}</Card.Description>
	</Card.Header>
{/snippet}

{#snippet postureItem(label: string, value: string, Icon: IconComponent, chip: string)}
	<div class="flex min-w-0 items-center gap-3 rounded-lg border bg-background/40 px-3 py-2.5">
		<span class={cn('flex size-8 shrink-0 items-center justify-center rounded-md', chip)}>
			<Icon class="size-4" />
		</span>
		<div class="min-w-0">
			<p class="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">{label}</p>
			<p class="truncate text-sm font-semibold">{value}</p>
		</div>
	</div>
{/snippet}

{#snippet detailRow(label: string, value: string, Icon: IconComponent, chip: string)}
	<div class="flex items-center gap-3 px-3 py-2.5">
		<span class={cn('flex size-8 shrink-0 items-center justify-center rounded-md', chip)}>
			<Icon class="size-4" />
		</span>
		<div class="min-w-0">
			<p class="text-xs text-muted-foreground">{label}</p>
			<p class="truncate text-sm font-medium">{value}</p>
		</div>
	</div>
{/snippet}

{#snippet mfaMethodRow(label: string, enabled: boolean, Icon: IconComponent, chip: string)}
	<div class="flex items-center justify-between gap-3 px-3 py-2.5">
		<div class="flex min-w-0 items-center gap-3">
			<span
				class={cn(
					'flex size-8 shrink-0 items-center justify-center rounded-md',
					enabled ? chip : 'bg-muted text-muted-foreground'
				)}
			>
				<Icon class="size-4" />
			</span>
			<span class={cn('truncate text-sm font-medium', !enabled && 'text-muted-foreground')}>
				{label}
			</span>
		</div>
		<Badge
			variant={enabled ? 'outline' : 'secondary'}
			class={cn('shrink-0', !enabled && 'text-muted-foreground')}
		>
			{enabled ? m.oauth_clients_overview_enabled() : m.oauth_clients_overview_disabled()}
		</Badge>
	</div>
{/snippet}

{#snippet uriRow(uri: string)}
	<li
		class="group/uri flex items-center gap-2.5 rounded-md border bg-background/40 px-3 py-2 transition-colors hover:bg-muted/40"
	>
		<LinkIcon class="size-4 shrink-0 text-muted-foreground" />
		<span class="flex-1 truncate font-mono text-xs" title={uri}>{uri}</span>
		<button
			type="button"
			onclick={() => copyUri(uri)}
			class="shrink-0 rounded-md p-1 text-muted-foreground opacity-0 transition group-hover/uri:opacity-100 hover:bg-muted hover:text-foreground focus-visible:opacity-100"
			aria-label={m.oauth_clients_overview_copy_uri()}
		>
			<CopyIcon class="size-3.5" />
		</button>
	</li>
{/snippet}

{#snippet legalUriRow(label: string, uri: string | null, fallback: string)}
	<div class="flex items-center justify-between gap-3 px-3 py-2.5">
		<div class="min-w-0">
			<p class="text-xs text-muted-foreground">{label}</p>
			{#if uri}
				<p class="truncate font-mono text-xs font-medium" title={uri}>{uri}</p>
			{:else}
				<p class="text-sm font-medium">{fallback}</p>
			{/if}
		</div>
		{#if uri}
			<button
				type="button"
				onclick={() => copyUri(uri)}
				class="shrink-0 rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
				aria-label={`Copy ${label}`}
			>
				<CopyIcon class="size-3.5" />
			</button>
		{/if}
	</div>
{/snippet}

<div class="space-y-5">
	<Card.Root>
		{@render sectionTitle(
			m.oauth_clients_overview_security_posture_title(),
			m.oauth_clients_overview_security_posture_description(),
			ShieldCheckIcon
		)}
		<Card.Content class="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
			{@render postureItem(
				m.oauth_clients_overview_status_label(),
				client.is_active ? m.oauth_clients_status_active() : m.oauth_clients_status_inactive(),
				client.is_active ? ShieldCheckIcon : ShieldOffIcon,
				client.is_active
					? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
					: 'bg-muted text-muted-foreground'
			)}
			{@render postureItem(
				m.oauth_clients_form_client_type_label(),
				clientTypeLabel(client.client_type),
				client.client_type === 'confidential' ? LockIcon : GlobeIcon,
				client.client_type === 'confidential'
					? 'bg-violet-500/10 text-violet-600 dark:text-violet-400'
					: 'bg-sky-500/10 text-sky-600 dark:text-sky-400'
			)}
			{@render postureItem(
				m.oauth_clients_overview_pkce_label(),
				client.require_pkce
					? m.oauth_clients_overview_required()
					: m.oauth_clients_overview_optional(),
				KeyRoundIcon,
				client.require_pkce
					? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
					: 'bg-amber-500/10 text-amber-600 dark:text-amber-400'
			)}
			{@render postureItem(
				m.oauth_clients_overview_consent_label(),
				client.require_consent
					? m.oauth_clients_overview_required()
					: m.oauth_clients_overview_skipped(),
				UserCheckIcon,
				client.require_consent
					? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
					: 'bg-amber-500/10 text-amber-600 dark:text-amber-400'
			)}
			{@render postureItem(
				m.oauth_clients_overview_mfa_label(),
				mfaSummary(client),
				ShieldAlertIcon,
				client.mfa_required
					? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
					: 'bg-muted text-muted-foreground'
			)}
			{@render postureItem(
				m.oauth_clients_overview_sign_in_methods_label(),
				client.external_login_only
					? m.oauth_clients_overview_sign_in_external_only()
					: m.oauth_clients_overview_sign_in_password_and_external(),
				CircleUserIcon,
				client.external_login_only
					? 'bg-violet-500/10 text-violet-600 dark:text-violet-400'
					: 'bg-muted text-muted-foreground'
			)}
		</Card.Content>
	</Card.Root>

	<div class="grid items-start gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(340px,420px)]">
		<div class="space-y-5">
			<Card.Root>
				{@render sectionTitle(
					m.oauth_clients_overview_scopes_grants_title(),
					m.oauth_clients_overview_scopes_grants_description(),
					ScrollTextIcon
				)}
				<Card.Content class="space-y-5">
					<div class="space-y-2.5">
						<div class="flex items-center gap-2">
							<p class="text-sm font-medium">{m.oauth_clients_form_allowed_scopes_label()}</p>
							<Badge variant="secondary" class="rounded-full px-2">
								{client.allowed_scopes.length}
							</Badge>
						</div>
						{#if client.allowed_scopes.length}
							<div class="flex flex-wrap gap-1.5">
								{#each client.allowed_scopes as scope (scope)}
									<span
										class="inline-flex items-center gap-1.5 rounded-md border border-primary/20 bg-primary/5 px-2 py-1 text-xs font-medium text-primary"
										title={scope}
									>
										<span class="size-1.5 rounded-full bg-primary"></span>
										{scopeLabel(scope)}
									</span>
								{/each}
							</div>
						{:else}
							<p class="text-sm text-muted-foreground">{m.oauth_clients_overview_no_scopes()}</p>
						{/if}
					</div>

					<div class="h-px bg-border"></div>

					<div class="space-y-2.5">
						<div class="flex items-center gap-2">
							<p class="text-sm font-medium">{m.oauth_clients_form_grant_types_label()}</p>
							<Badge variant="secondary" class="rounded-full px-2">
								{client.allowed_grant_types.length}
							</Badge>
						</div>
						{#if client.allowed_grant_types.length}
							<div class="flex flex-wrap gap-1.5">
								{#each client.allowed_grant_types as grant (grant)}
									<span
										class="inline-flex items-center gap-1.5 rounded-md border border-sky-500/20 bg-sky-500/5 px-2 py-1 text-xs font-medium text-sky-700 dark:text-sky-300"
										title={grant}
									>
										<KeyIcon class="size-3" />
										{grantLabel(grant)}
									</span>
								{/each}
							</div>
						{:else}
							<p class="text-sm text-muted-foreground">
								{m.oauth_clients_overview_no_grant_types()}
							</p>
						{/if}
					</div>
				</Card.Content>
			</Card.Root>

			<Card.Root>
				{@render sectionTitle(
					m.oauth_clients_form_redirect_uris_label(),
					m.oauth_clients_overview_redirect_uris_description(),
					LinkIcon
				)}
				<Card.Content class="grid gap-5 lg:grid-cols-2">
					<div class="min-w-0">
						<p class="mb-2 text-sm font-medium">{m.oauth_clients_overview_sign_in_redirects()}</p>
						{#if client.redirect_uris.length}
							<ul class="space-y-1.5">
								{#each client.redirect_uris as uri (uri)}
									{@render uriRow(uri)}
								{/each}
							</ul>
						{:else}
							<p class="text-sm text-muted-foreground">
								{m.oauth_clients_overview_no_redirect_uris()}
							</p>
						{/if}
					</div>

					<div class="min-w-0">
						<p class="mb-2 text-sm font-medium">
							{m.oauth_clients_form_post_logout_redirect_uris_label()}
						</p>
						{#if client.post_logout_redirect_uris.length}
							<ul class="space-y-1.5">
								{#each client.post_logout_redirect_uris as uri (uri)}
									{@render uriRow(uri)}
								{/each}
							</ul>
						{:else}
							<p class="text-sm text-muted-foreground">
								{m.oauth_clients_overview_none_configured()}
							</p>
						{/if}
					</div>
				</Card.Content>
			</Card.Root>
		</div>

		<div class="space-y-5">
			<Card.Root>
				{@render sectionTitle(
					m.oauth_clients_overview_mfa_policy_title(),
					m.oauth_clients_overview_mfa_policy_description(),
					ShieldAlertIcon
				)}
				<Card.Content class="space-y-3">
					<div class="rounded-lg border bg-muted/25 p-3">
						<div class="flex items-center justify-between gap-3">
							<p class="text-sm font-medium">{m.oauth_clients_overview_requirement_label()}</p>
							<Badge variant={client.mfa_required ? 'default' : 'secondary'} class="shrink-0">
								{client.mfa_required
									? m.oauth_clients_overview_required()
									: m.oauth_clients_overview_not_required()}
							</Badge>
						</div>
						<p class="mt-1 text-xs text-muted-foreground">
							{enabledMfaMethodCount(client) === 1
								? m.oauth_clients_overview_methods_enabled_one({
										count: enabledMfaMethodCount(client)
									})
								: m.oauth_clients_overview_methods_enabled_other({
										count: enabledMfaMethodCount(client)
									})}
						</p>
					</div>

					<div class="divide-y rounded-lg border">
						{@render mfaMethodRow(
							m.oauth_clients_form_mfa_email_label(),
							client.mfa_email_enabled,
							MailIcon,
							'bg-sky-500/10 text-sky-600 dark:text-sky-400'
						)}
						{@render mfaMethodRow(
							m.oauth_clients_form_mfa_app_label(),
							client.mfa_totp_enabled,
							SmartphoneIcon,
							'bg-violet-500/10 text-violet-600 dark:text-violet-400'
						)}
						{@render mfaMethodRow(
							m.oauth_clients_form_mfa_passkey_label(),
							client.mfa_webauthn_enabled,
							KeyRoundIcon,
							'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
						)}
					</div>
				</Card.Content>
			</Card.Root>

			<Card.Root>
				{@render sectionTitle(
					m.oauth_clients_overview_activity_delivery_title(),
					m.oauth_clients_overview_activity_delivery_description(),
					ActivityIcon
				)}
				<Card.Content class="space-y-3">
					<div class="divide-y rounded-lg border">
						{@render detailRow(
							m.roles_overview_created_label(),
							formatClientDate(client.created_at),
							CalendarPlusIcon,
							'bg-sky-500/10 text-sky-600 dark:text-sky-400'
						)}
						{@render detailRow(
							m.roles_overview_last_updated_label(),
							formatClientDate(client.updated_at),
							ClockIcon,
							'bg-violet-500/10 text-violet-600 dark:text-violet-400'
						)}
						<div class="px-3 py-2.5">
							<EmailProviderSelect {tenantId} {client} />
						</div>
					</div>
				</Card.Content>
			</Card.Root>

			<Card.Root>
				<Card.Header class="flex flex-row items-start justify-between gap-3">
					<div>
						<Card.Title class="flex items-center gap-2">
							<ScrollTextIcon class="size-4 text-muted-foreground" />
							{m.oauth_clients_overview_legal_links_title()}
						</Card.Title>
						<Card.Description>{m.oauth_clients_overview_legal_links_description()}</Card.Description
						>
					</div>
					<Button variant="outline" size="sm" href={editHref}
						>{m.oauth_clients_action_edit()}</Button
					>
				</Card.Header>
				<Card.Content>
					<div class="divide-y rounded-lg border">
						{@render legalUriRow(
							m.tenant_terms_of_service(),
							client.tos_uri,
							m.oauth_clients_overview_platform_terms()
						)}
						{@render legalUriRow(
							m.tenant_privacy_policy(),
							client.policy_uri,
							m.oauth_clients_overview_platform_policy()
						)}
					</div>
				</Card.Content>
			</Card.Root>
		</div>
	</div>
</div>

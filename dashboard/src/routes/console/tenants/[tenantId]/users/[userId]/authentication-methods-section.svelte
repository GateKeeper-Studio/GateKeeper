<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import FingerprintIcon from '@lucide/svelte/icons/fingerprint';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import LinkIcon from '@lucide/svelte/icons/link';
	import MailIcon from '@lucide/svelte/icons/mail';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import SmartphoneIcon from '@lucide/svelte/icons/smartphone';
	import UsersIcon from '@lucide/svelte/icons/users';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import GithubLogo from '$lib/components/icons/github-logo.svelte';
	import GoogleLogo from '$lib/components/icons/google-logo.svelte';
	import { getUserAuthenticationMethods } from '$lib/services/admin/users';
	import ResetPasswordDialog from './reset-password-dialog.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let { tenantId, userId, email }: { tenantId: string; userId: string; email: string } = $props();

	let resetOpen = $state(false);

	// How this user can sign in: read-only, account-level. Whether a given app offers a
	// method (email links, passkeys, a social provider) is that app's own setting.
	const methodsQuery = createQuery(() => ({
		queryKey: ['tenant-users', tenantId, userId, 'authentication-methods'],
		enabled: !!tenantId && !!userId,
		queryFn: async () => {
			const [data, err] = await getUserAuthenticationMethods(tenantId, userId);
			if (err) throw err;
			return data;
		}
	}));

	const date = (iso: string | null) => (iso ? new Date(iso).toLocaleDateString() : '');

	function providerLabel(provider: string): string {
		if (provider === 'github') return 'GitHub';
		if (provider === 'google') return 'Google';
		return provider;
	}

	function methodLabel(method: string | null): string {
		if (method === 'totp') return m.users_auth_methods_authenticator_app();
		if (method === 'webauthn') return m.users_auth_methods_passkeys();
		if (method === 'email') return m.users_auth_methods_email_code();
		return m.users_auth_methods_preferred_auto();
	}
</script>

{#snippet row(
	Icon: typeof KeyRoundIcon,
	label: string,
	on: boolean,
	status: string,
	details: string | null = null
)}
	<li class="flex items-start gap-3 py-3">
		<Icon class="mt-0.5 size-4 shrink-0 text-muted-foreground" />
		<div class="min-w-0 flex-1">
			<div class="flex flex-wrap items-center justify-between gap-2">
				<span class="font-medium">{label}</span>
				<Badge variant={on ? 'default' : 'outline'} class="font-normal">{status}</Badge>
			</div>
			{#if details}<p class="mt-0.5 text-xs text-muted-foreground">{details}</p>{/if}
		</div>
	</li>
{/snippet}

<div class="mt-4 space-y-1">
	<h2 class="text-lg font-semibold">{m.users_auth_methods_title()}</h2>
	<p class="max-w-2xl text-sm text-muted-foreground">{m.users_auth_methods_description()}</p>
</div>

<div class="mt-4">
	{#if methodsQuery.isPending}
		<Skeleton class="h-64 w-full rounded-xl" />
	{:else if methodsQuery.isError}
		<ErrorAlert title={m.users_auth_methods_load_error()} message={methodsQuery.error.message} />
	{:else if methodsQuery.data}
		{@const a = methodsQuery.data}
		{@const factors = a.secondFactors}
		{@const unverified = m.users_auth_methods_needs_verified_email()}
		<div class="grid gap-4 lg:grid-cols-2">
			<Card.Root>
				<Card.Header>
					<Card.Title>{m.users_auth_methods_sign_in_title()}</Card.Title>
				</Card.Header>
				<Card.Content>
					<ul class="divide-y text-sm">
						{@render row(
							KeyRoundIcon,
							m.users_auth_methods_password(),
							a.password.set,
							a.password.set ? m.users_auth_methods_set() : m.users_auth_methods_not_set(),
							a.password.set
								? [
										a.password.changedAt
											? m.users_auth_methods_changed({ date: date(a.password.changedAt) })
											: null,
										a.password.mustChange ? m.users_auth_methods_must_change() : null
									]
										.filter(Boolean)
										.join(' · ')
								: m.users_auth_methods_no_password()
						)}
						{@render row(
							FingerprintIcon,
							m.users_auth_methods_passkey_sign_in(),
							a.passkeySignIn,
							a.passkeySignIn
								? m.users_auth_methods_available()
								: m.users_auth_methods_unavailable(),
							a.passkeySignIn
								? m.users_auth_methods_passkey_sign_in_on()
								: m.users_auth_methods_passkey_sign_in_off()
						)}
						{@render row(
							MailIcon,
							m.users_auth_methods_magic_link(),
							a.magicLink,
							a.magicLink ? m.users_auth_methods_available() : m.users_auth_methods_unavailable(),
							a.magicLink ? m.users_auth_methods_magic_link_on() : unverified
						)}
						<li class="flex items-start gap-3 py-3">
							<UsersIcon class="mt-0.5 size-4 shrink-0 text-muted-foreground" />
							<div class="min-w-0 flex-1">
								<div class="flex flex-wrap items-center justify-between gap-2">
									<span class="font-medium">{m.users_auth_methods_social()}</span>
									<Badge
										variant={a.socialAccounts.length > 0 ? 'default' : 'outline'}
										class="font-normal"
									>
										{a.socialAccounts.length > 0
											? m.users_auth_methods_linked({ count: a.socialAccounts.length })
											: m.users_auth_methods_none()}
									</Badge>
								</div>
								{#each a.socialAccounts as account (account.provider + account.linkedAt)}
									<p class="mt-1 flex items-center gap-1.5 text-xs text-muted-foreground">
										{#if account.provider === 'github'}
											<GithubLogo size={12} />
										{:else if account.provider === 'google'}
											<GoogleLogo size={12} />
										{:else}
											<LinkIcon class="size-3" />
										{/if}
										<span class="font-medium text-foreground"
											>{providerLabel(account.provider)}</span
										>
										{#if account.email}<span>{account.email}</span>{/if}
										<span>· {m.users_auth_methods_linked_on({ date: date(account.linkedAt) })}</span
										>
									</p>
								{/each}
							</div>
						</li>
					</ul>
				</Card.Content>
				{#if a.password.set}
					<Card.Footer>
						<Button variant="outline" size="sm" onclick={() => (resetOpen = true)}>
							<KeyRoundIcon class="size-4" />
							{m.users_reset_password_button()}
						</Button>
					</Card.Footer>
				{/if}
			</Card.Root>

			<Card.Root>
				<Card.Header>
					<Card.Title>{m.users_auth_methods_two_step_title()}</Card.Title>
					<Card.Description>
						{m.users_auth_methods_preferred({ method: methodLabel(factors.preferredMethod) })}
					</Card.Description>
				</Card.Header>
				<Card.Content>
					<ul class="divide-y text-sm">
						{@render row(
							MailIcon,
							m.users_auth_methods_email_code(),
							factors.emailCode,
							factors.emailCode
								? m.users_auth_methods_available()
								: m.users_auth_methods_unavailable(),
							factors.emailCode ? m.users_auth_methods_email_code_on() : unverified
						)}
						{@render row(
							SmartphoneIcon,
							m.users_auth_methods_authenticator_app(),
							!!factors.authenticatorApp?.confirmed,
							factors.authenticatorApp
								? factors.authenticatorApp.confirmed
									? m.users_auth_methods_on()
									: m.users_auth_methods_pending()
								: m.users_auth_methods_off(),
							factors.authenticatorApp
								? factors.authenticatorApp.confirmed
									? [
											m.users_auth_methods_since({
												date: date(factors.authenticatorApp.enrolledAt)
											}),
											factors.authenticatorApp.lastUsedAt
												? m.users_auth_methods_last_used({
														date: date(factors.authenticatorApp.lastUsedAt)
													})
												: null
										]
											.filter(Boolean)
											.join(' · ')
									: m.users_auth_methods_pending_details()
								: null
						)}
						<li class="flex items-start gap-3 py-3">
							<ShieldCheckIcon class="mt-0.5 size-4 shrink-0 text-muted-foreground" />
							<div class="min-w-0 flex-1">
								<div class="flex flex-wrap items-center justify-between gap-2">
									<span class="font-medium">{m.users_auth_methods_passkeys()}</span>
									<Badge
										variant={factors.passkeys.length > 0 ? 'default' : 'outline'}
										class="font-normal"
									>
										{factors.passkeys.length > 0
											? m.users_auth_methods_passkey_count({ count: factors.passkeys.length })
											: m.users_auth_methods_none()}
									</Badge>
								</div>
								{#each factors.passkeys as passkey (passkey.id)}
									<p class="mt-1 text-xs text-muted-foreground">
										<span class="font-medium text-foreground"
											>{passkey.name || m.users_auth_methods_unnamed_passkey()}</span
										>
										· {m.users_auth_methods_added({ date: date(passkey.createdAt) })}
										{#if passkey.lastUsedAt}
											· {m.users_auth_methods_last_used({ date: date(passkey.lastUsedAt) })}
										{/if}
									</p>
								{/each}
							</div>
						</li>
					</ul>
				</Card.Content>
			</Card.Root>
		</div>
		<p class="mt-2 text-xs text-muted-foreground">{m.users_auth_methods_footnote()}</p>
	{/if}
</div>

<ResetPasswordDialog
	{tenantId}
	{userId}
	{email}
	open={resetOpen}
	onOpenChange={(o) => (resetOpen = o)}
/>

<script lang="ts">
	import MailIcon from '@lucide/svelte/icons/mail';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import GithubLogo from '$lib/components/icons/github-logo.svelte';
	import GoogleLogo from '$lib/components/icons/google-logo.svelte';
	import type { TenantUserView } from '$lib/services/admin/users';
	import UserAvatar from './user-avatar.svelte';
	import { userFullName } from './display.js';
	import { m } from '$lib/paraglide/messages.js';

	let { user, tenantId }: { user: TenantUserView; tenantId: string } = $props();

	const providers = $derived(user.providers ?? []);
	// Show login-method badges only when there is something beyond a plain password
	// account, so unremarkable rows stay uncluttered.
	const showMethods = $derived(providers.length > 0);

	function providerLabel(provider: string): string {
		if (provider === 'github') return 'GitHub';
		if (provider === 'google') return 'Google';
		return provider;
	}
</script>

<div class="flex items-center gap-3">
	<UserAvatar {user} />
	<div class="min-w-0">
		<a
			href={`/tenants/${tenantId}/users/${user.user_id}`}
			class="hover:text-primary block truncate font-medium hover:underline"
		>
			{userFullName(user)}
		</a>
		<p class="text-muted-foreground flex items-center gap-1 truncate text-xs">
			<MailIcon class="size-3 shrink-0" />
			<span class="truncate">{user.email}</span>
		</p>
		{#if showMethods}
			<div class="mt-1 flex flex-wrap items-center gap-1">
				{#if user.has_password}
					<Badge variant="outline" class="gap-1 font-normal">
						<KeyRoundIcon class="size-3" />
						{m.users_badge_password()}
					</Badge>
				{/if}
				{#each providers as provider (provider)}
					<Badge variant="outline" class="gap-1 font-normal">
						{#if provider === 'github'}
							<GithubLogo size={12} />
						{:else if provider === 'google'}
							<GoogleLogo size={12} />
						{/if}
						{providerLabel(provider)}
					</Badge>
				{/each}
			</div>
		{/if}
	</div>
</div>

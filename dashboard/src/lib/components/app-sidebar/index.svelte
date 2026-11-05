<script lang="ts" module>
</script>

<script lang="ts">
	import type { ComponentProps } from 'svelte';
	import SquareTerminalIcon from '@lucide/svelte/icons/square-terminal';
	import UserIcon from '@lucide/svelte/icons/user';
	import GroupIcon from '@lucide/svelte/icons/group';
	import UserKeyIcon from '@lucide/svelte/icons/user-key';
	import MailsIcon from '@lucide/svelte/icons/mails';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import ListChecksIcon from '@lucide/svelte/icons/list-checks';
	import ShieldIcon from '@lucide/svelte/icons/shield';
	import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';
	import ActivityIcon from '@lucide/svelte/icons/activity';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import BanIcon from '@lucide/svelte/icons/ban';
	import SlidersHorizontalIcon from '@lucide/svelte/icons/sliders-horizontal';
	import ScrollTextIcon from '@lucide/svelte/icons/scroll-text';
	import Building2Icon from '@lucide/svelte/icons/building-2';
	import CreditCardIcon from '@lucide/svelte/icons/credit-card';
	import WebhookIcon from '@lucide/svelte/icons/webhook';

	import TenantSwitcher from './tenant-switcher.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import NavMain from './nav-main.svelte';
	import NavUser from './nav-user.svelte';
	import NavOAuthClients from './nav-oauth-clients.svelte';
	import { page } from '$app/state';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let {
		ref = $bindable(null),
		collapsible = 'icon',
		...restProps
	}: ComponentProps<typeof Sidebar.Root> = $props();

	// The signed-in admin, resolved server-side in `(dashboard)/+layout.server.ts`
	// from the session cookie. It used to be decoded from an access token in
	// localStorage, which meant it was null during SSR and only appeared after
	// hydration; coming through page data it is correct on the first render.
	const admin = $derived(page.data.admin);
	const user = $derived({
		name: admin?.name ?? 'Admin',
		email: admin?.email ?? '',
		initials: admin?.initials || 'AD'
	});

	const navMain = $derived([
		{
			title: m.nav_general(),
			url: '#',
			icon: SquareTerminalIcon,
			isActive: true,
			tenantScoped: true,
			items: [
				{
					title: m.nav_users(),
					url: `/tenants/${selectedTenant.tenantId}/users`,
					icon: UserIcon
				},
				{
					title: m.nav_groups(),
					url: `/tenants/${selectedTenant.tenantId}/groups`,
					icon: GroupIcon
				},
				{
					title: m.nav_roles(),
					url: `/tenants/${selectedTenant.tenantId}/roles`,
					icon: UserKeyIcon
				},
				{
					title: m.nav_oauth_clients(),
					url: `/tenants/${selectedTenant.tenantId}/oauth-clients`,
					icon: KeyRoundIcon
				},
				{
					title: m.nav_custom_fields(),
					url: `/tenants/${selectedTenant.tenantId}/custom-fields`,
					icon: ListChecksIcon
				},
				{
					title: m.nav_email(),
					url: `/tenants/${selectedTenant.tenantId}/email`,
					icon: MailsIcon
				},
				{
					title: m.nav_webhooks(),
					url: `/tenants/${selectedTenant.tenantId}/webhooks`,
					icon: WebhookIcon
				}
			]
		},
		{
			title: m.nav_security(),
			url: '#',
			icon: ShieldAlertIcon,
			tenantScoped: true,
			items: [
				{
					title: m.nav_overview(),
					url: `/tenants/${selectedTenant.tenantId}/security`,
					icon: ShieldAlertIcon
				},
				{
					title: m.nav_login_events(),
					url: `/tenants/${selectedTenant.tenantId}/security/login-events`,
					icon: ActivityIcon
				},
				{
					title: m.nav_suspicious_ips(),
					url: `/tenants/${selectedTenant.tenantId}/security/suspicious-ips`,
					icon: GlobeIcon
				},
				{
					title: m.nav_attacked_users(),
					url: `/tenants/${selectedTenant.tenantId}/security/attacked-users`,
					icon: BanIcon
				},
				{
					title: m.nav_risk_policy(),
					url: `/tenants/${selectedTenant.tenantId}/security/risk-policy`,
					icon: SlidersHorizontalIcon
				}
			]
		},
		{
			title: m.nav_compliance(),
			url: '#',
			icon: ShieldIcon,
			tenantScoped: true,
			items: [
				{
					title: m.nav_audit_log(),
					url: `/tenants/${selectedTenant.tenantId}/audit`,
					icon: ShieldIcon
				},
				{
					title: m.nav_compliance(),
					url: `/tenants/${selectedTenant.tenantId}/compliance`,
					icon: ShieldIcon
				},
				{
					title: m.nav_legal_pages(),
					url: `/tenants/${selectedTenant.tenantId}/legal`,
					icon: ScrollTextIcon
				}
			]
		}
	]);
</script>

<Sidebar.Root bind:ref {collapsible} {...restProps}>
	<Sidebar.Header>
		<TenantSwitcher />
	</Sidebar.Header>

	<Sidebar.Content>
		<NavMain items={navMain} />
	</Sidebar.Content>

	<Sidebar.Footer>
		<NavUser {user} />
	</Sidebar.Footer>
	<Sidebar.Rail />
</Sidebar.Root>

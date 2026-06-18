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
	import ShieldIcon from '@lucide/svelte/icons/shield';
	import ScrollTextIcon from '@lucide/svelte/icons/scroll-text';
	import Building2Icon from '@lucide/svelte/icons/building-2';
	import CreditCardIcon from '@lucide/svelte/icons/credit-card';

	import TenantSwitcher from './tenant-switcher.svelte';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import NavMain from './nav-main.svelte';
	import NavUser from './nav-user.svelte';
	import NavOAuthClients from './nav-oauth-clients.svelte';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';

	let {
		ref = $bindable(null),
		collapsible = 'icon',
		...restProps
	}: ComponentProps<typeof Sidebar.Root> = $props();

	const user = { name: 'shadcn', email: 'm@example.com', avatar: '/avatars/shadcn.jpg' };

	const navMain = $derived([
		{
			// Organization-level (not tenant-scoped): an admin manages exactly the one
			// organization it belongs to and that organization's single subscription.
			title: 'Organization',
			url: '#',
			icon: Building2Icon,
			isActive: true,
			items: [
				{
					title: 'Organization',
					url: '/dashboard/organization',
					icon: Building2Icon
				},
				{
					title: 'Billing',
					url: '/dashboard/billing',
					icon: CreditCardIcon
				}
			]
		},
		{
			title: 'General',
			url: '#',
			icon: SquareTerminalIcon,
			isActive: true,
			items: [
				{
					title: 'Users',
					url: `/dashboard/${selectedTenant.tenantId}/users`,
					icon: UserIcon
				},
				{
					title: 'Groups',
					url: `/dashboard/${selectedTenant.tenantId}/groups`,
					icon: GroupIcon
				},
				{
					title: 'Roles',
					url: `/dashboard/${selectedTenant.tenantId}/roles`,
					icon: UserKeyIcon
				},
				{
					title: 'OAuth Clients',
					url: `/dashboard/${selectedTenant.tenantId}/oauth-clients`,
					icon: KeyRoundIcon
				},
				{
					title: 'E-mail Providers',
					url: `/dashboard/${selectedTenant.tenantId}/email-providers`,
					icon: MailsIcon
				}
			]
		},
		{
			title: 'Compliance',
			url: '#',
			icon: ShieldIcon,
			items: [
				{
					title: 'Audit Log',
					url: `/dashboard/${selectedTenant.tenantId}/audit`,
					icon: ShieldIcon
				},
				{
					title: 'Compliance',
					url: `/dashboard/${selectedTenant.tenantId}/compliance`,
					icon: ShieldIcon
				},
				{
					title: 'Legal Pages',
					url: `/dashboard/${selectedTenant.tenantId}/legal`,
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
		<NavOAuthClients />
	</Sidebar.Content>

	<Sidebar.Footer>
		<NavUser {user} />
	</Sidebar.Footer>
	<Sidebar.Rail />
</Sidebar.Root>

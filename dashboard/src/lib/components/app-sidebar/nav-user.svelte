<script lang="ts">
	import * as Avatar from '$lib/components/ui/avatar/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { useSidebar } from '$lib/components/ui/sidebar/index.js';
	import BadgeCheckIcon from '@lucide/svelte/icons/badge-check';
	import Building2Icon from '@lucide/svelte/icons/building-2';
	import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { adminLogout } from '$lib/services/auth/admin-login';
	import { settingsDialog } from '$lib/state/settings-dialog.svelte';

	let { user }: { user: { name: string; email: string; initials: string } } = $props();

	const sidebar = useSidebar();

	// Server-side sign-out: revokes the refresh-token family at the IdP (so a leaked
	// copy is dead, not merely forgotten) and clears the session cookie. There are no
	// client-side tokens left to clear.
	async function logout() {
		await adminLogout();
		await goto(resolve('/login'));
	}
</script>

<Sidebar.Menu>
	<Sidebar.MenuItem>
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Sidebar.MenuButton
						size="lg"
						class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
						{...props}
					>
						<Avatar.Root class="size-8 rounded-lg">
							<Avatar.Fallback class="rounded-lg">{user.initials}</Avatar.Fallback>
						</Avatar.Root>
						<div class="grid flex-1 text-start text-sm leading-tight">
							<span class="truncate font-medium">{user.name}</span>
							<span class="truncate text-xs">{user.email}</span>
						</div>
						<ChevronsUpDownIcon class="ms-auto size-4" />
					</Sidebar.MenuButton>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content
				class="w-(--bits-dropdown-menu-anchor-width) min-w-56 rounded-lg"
				side={sidebar.isMobile ? 'bottom' : 'right'}
				align="end"
				sideOffset={4}
			>
				<DropdownMenu.Label class="p-0 font-normal">
					<div class="flex items-center gap-2 px-1 py-1.5 text-start text-sm">
						<Avatar.Root class="size-8 rounded-lg">
							<Avatar.Fallback class="rounded-lg">{user.initials}</Avatar.Fallback>
						</Avatar.Root>
						<div class="grid flex-1 text-start text-sm leading-tight">
							<span class="truncate font-medium">{user.name}</span>
							<span class="truncate text-xs">{user.email}</span>
						</div>
					</div>
				</DropdownMenu.Label>

				<DropdownMenu.Separator />

				<DropdownMenu.Group>
					<DropdownMenu.Item onSelect={() => settingsDialog.openCurrent()}>
						<SettingsIcon />
						Settings
					</DropdownMenu.Item>

					<DropdownMenu.Item onSelect={() => settingsDialog.openTo('account')}>
						<BadgeCheckIcon />
						Account
					</DropdownMenu.Item>
				</DropdownMenu.Group>

				<DropdownMenu.Separator />

				<DropdownMenu.Group>
					<DropdownMenu.Item onSelect={() => settingsDialog.openTo('organization')}>
						<Building2Icon />
						Organization
					</DropdownMenu.Item>
				</DropdownMenu.Group>

				<DropdownMenu.Separator />

				<DropdownMenu.Item onSelect={logout} class="text-red-500">
					<LogOutIcon />
					Log out
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</Sidebar.MenuItem>
</Sidebar.Menu>

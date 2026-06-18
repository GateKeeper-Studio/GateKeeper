<script lang="ts">
	import { toast } from 'svelte-sonner';

	import CalendarPlusIcon from '@lucide/svelte/icons/calendar-plus';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import ShieldIcon from '@lucide/svelte/icons/shield';
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import IdCardIcon from '@lucide/svelte/icons/id-card';
	import AlignLeftIcon from '@lucide/svelte/icons/align-left';
	import CopyIcon from '@lucide/svelte/icons/copy';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';

	import type { TenantRoleView } from '$lib/services/admin/roles';
	import { formatRoleDate } from '../display.js';

	type Props = { role: TenantRoleView };
	let { role }: Props = $props();

	async function copyValue(value: string, label: string) {
		try {
			await navigator.clipboard.writeText(value);
			toast.success(`${label} copied to clipboard`);
		} catch {
			toast.error('Could not copy to clipboard');
		}
	}
</script>

{#snippet statTile(label: string, value: string, Icon: any, chip: string)}
	<div class="bg-card flex items-center gap-3 rounded-xl border p-4">
		<span class={`flex size-10 shrink-0 items-center justify-center rounded-lg ${chip}`}>
			<Icon class="size-5" />
		</span>
		<div class="min-w-0">
			<p class="text-muted-foreground text-xs font-medium">{label}</p>
			<p class="truncate font-semibold">{value}</p>
		</div>
	</div>
{/snippet}

{#snippet infoRow(label: string, value: string, Icon: any, copyable: boolean)}
	<div class="group flex items-center gap-3">
		<span
			class="bg-muted text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-lg"
		>
			<Icon class="size-4" />
		</span>
		<div class="min-w-0 flex-1">
			<p class="text-muted-foreground text-xs">{label}</p>
			<p class="truncate text-sm font-medium">{value}</p>
		</div>
		{#if copyable}
			<Button
				variant="ghost"
				size="icon"
				class="text-muted-foreground hover:text-foreground size-8 shrink-0 opacity-0 transition group-hover:opacity-100"
				onclick={() => copyValue(value, label)}
			>
				<CopyIcon class="size-3.5" />
			</Button>
		{/if}
	</div>
{/snippet}

<div class="space-y-4">
	<div class="grid grid-cols-2 gap-3 lg:grid-cols-3">
		{@render statTile(
			'Created',
			formatRoleDate(role.created_at),
			CalendarPlusIcon,
			'bg-sky-500/10 text-sky-600 dark:text-sky-400'
		)}
		{@render statTile(
			'Last updated',
			formatRoleDate(role.updated_at),
			ClockIcon,
			'bg-violet-500/10 text-violet-600 dark:text-violet-400'
		)}
		{@render statTile(
			'Users',
			String(role.user_count),
			UsersRoundIcon,
			'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
		)}
	</div>

	<Card.Root>
		<Card.Header>
			<Card.Title class="flex items-center gap-2">
				<IdCardIcon class="text-muted-foreground size-4" />
				Details
			</Card.Title>
			<Card.Description>Identity and configuration for this role.</Card.Description>
		</Card.Header>
		<Card.Content class="grid gap-3 sm:grid-cols-2">
			{@render infoRow('Name', role.name, ShieldIcon, false)}
			{@render infoRow('Description', role.description || '—', AlignLeftIcon, false)}
			{@render infoRow('Role ID', role.role_id, IdCardIcon, true)}
		</Card.Content>
	</Card.Root>
</div>

<script lang="ts">
	import { toast } from 'svelte-sonner';

	import CalendarPlusIcon from '@lucide/svelte/icons/calendar-plus';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import IdCardIcon from '@lucide/svelte/icons/id-card';
	import AlignLeftIcon from '@lucide/svelte/icons/align-left';
	import CopyIcon from '@lucide/svelte/icons/copy';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';

	import type { TenantGroupView } from '$lib/services/admin/groups';
	import { m } from '$lib/paraglide/messages.js';
	import { formatGroupDate } from '../display.js';

	type Props = { group: TenantGroupView };
	let { group }: Props = $props();

	async function copyValue(value: string, label: string) {
		try {
			await navigator.clipboard.writeText(value);
			toast.success(m.groups_copy_value_success_toast({ label }));
		} catch {
			toast.error(m.groups_copy_id_error_toast());
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
			m.groups_overview_stat_created(),
			formatGroupDate(group.created_at),
			CalendarPlusIcon,
			'bg-sky-500/10 text-sky-600 dark:text-sky-400'
		)}
		{@render statTile(
			m.groups_overview_stat_last_updated(),
			formatGroupDate(group.updated_at),
			ClockIcon,
			'bg-violet-500/10 text-violet-600 dark:text-violet-400'
		)}
		{@render statTile(
			m.groups_members_label(),
			String(group.member_count),
			UsersRoundIcon,
			'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
		)}
	</div>

	<Card.Root>
		<Card.Header>
			<Card.Title class="flex items-center gap-2">
				<IdCardIcon class="text-muted-foreground size-4" />
				{m.groups_overview_details_title()}
			</Card.Title>
			<Card.Description>{m.groups_overview_details_description()}</Card.Description>
		</Card.Header>
		<Card.Content class="grid gap-3 sm:grid-cols-2">
			{@render infoRow(m.groups_form_name_label(), group.name, UsersRoundIcon, false)}
			{@render infoRow(
				m.groups_form_description_label(),
				group.description || '—',
				AlignLeftIcon,
				false
			)}
			{@render infoRow(m.groups_overview_field_group_id(), group.group_id, IdCardIcon, true)}
		</Card.Content>
	</Card.Root>
</div>

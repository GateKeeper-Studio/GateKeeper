<script lang="ts">
	import { createQuery, keepPreviousData } from '@tanstack/svelte-query';
	import {
		getCoreRowModel,
		type PaginationState,
		type RowSelectionState
	} from '@tanstack/table-core';

	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';

	import * as Table from '$lib/components/ui/table/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import { createSvelteTable, FlexRender } from '$lib/components/ui/data-table/index.js';

	import { listTenantGroups, type TenantGroupView } from '$lib/services/admin/groups';
	import { m } from '$lib/paraglide/messages.js';
	import { getGroupColumns } from './columns.js';
	import DeleteGroupDialog from './delete-group-dialog.svelte';
	import BulkDeleteGroupsDialog from './bulk-delete-groups-dialog.svelte';

	type Props = { tenantId: string };
	let { tenantId }: Props = $props();

	const PAGE_SIZES = [10, 20, 50];

	let pagination = $state<PaginationState>({ pageIndex: 0, pageSize: 10 });
	let rowSelection = $state<RowSelectionState>({});

	let lastTenant = tenantId;
	$effect(() => {
		if (tenantId !== lastTenant) {
			lastTenant = tenantId;
			pagination = { pageIndex: 0, pageSize: pagination.pageSize };
			rowSelection = {};
		}
	});

	const groupsQuery = createQuery(() => ({
		queryKey: ['tenant-groups', tenantId, pagination.pageIndex, pagination.pageSize],
		enabled: !!tenantId,
		placeholderData: keepPreviousData,
		queryFn: async () => {
			const [data, err] = await listTenantGroups(tenantId, {
				page: pagination.pageIndex + 1,
				pageSize: pagination.pageSize
			});
			if (err) throw err;
			return data;
		}
	}));

	const rows = $derived(groupsQuery.data?.items ?? []);
	const total = $derived(groupsQuery.data?.total ?? 0);

	const selectedIds = $derived(
		Object.entries(rowSelection)
			.filter(([, selected]) => selected)
			.map(([id]) => id)
	);
	const selectedSample = $derived(rows.filter((g) => rowSelection[g.group_id]));

	let groupToDelete = $state<TenantGroupView | null>(null);
	let bulkDeleteOpen = $state(false);

	const columns = getGroupColumns(tenantId, (group) => (groupToDelete = group));

	const table = createSvelteTable({
		get data() {
			return rows;
		},
		columns,
		state: {
			get pagination() {
				return pagination;
			},
			get rowSelection() {
				return rowSelection;
			}
		},
		getRowId: (group) => group.group_id,
		enableRowSelection: true,
		manualPagination: true,
		get rowCount() {
			return total;
		},
		onPaginationChange: (updater) => {
			pagination = typeof updater === 'function' ? updater(pagination) : updater;
		},
		onRowSelectionChange: (updater) => {
			rowSelection = typeof updater === 'function' ? updater(rowSelection) : updater;
		},
		getCoreRowModel: getCoreRowModel()
	});

	const pageCount = $derived(Math.max(1, Math.ceil(total / pagination.pageSize)));
	const rangeStart = $derived(total === 0 ? 0 : pagination.pageIndex * pagination.pageSize + 1);
	const rangeEnd = $derived(Math.min(total, (pagination.pageIndex + 1) * pagination.pageSize));
</script>

<div class="space-y-4">
	{#if selectedIds.length}
		<div class="flex h-9 items-center justify-between gap-2">
			<p class="text-muted-foreground text-sm">
				{#if selectedIds.length}
					{m.groups_selected_count({ count: selectedIds.length })}
				{:else if total}
					{m.groups_count_label({
						count: total,
						noun: total === 1 ? m.groups_noun_singular() : m.groups_noun_plural()
					})}
				{/if}
			</p>

			{#if selectedIds.length}
				<Button variant="destructive" size="sm" onclick={() => (bulkDeleteOpen = true)}>
					<Trash2Icon />
					{m.groups_delete_selected_button({ count: selectedIds.length })}
				</Button>
			{/if}
		</div>
	{/if}

	{#if groupsQuery.isError}
		<ErrorAlert title={m.groups_load_groups_error_title()} message={groupsQuery.error.message} />
	{:else}
		<div
			class={[
				'overflow-hidden rounded-xl border',
				groupsQuery.isPlaceholderData && 'opacity-60 transition-opacity'
			]}
		>
			<Table.Root>
				<Table.Header>
					{#each table.getHeaderGroups() as headerGroup (headerGroup.id)}
						<Table.Row class="bg-muted/40 hover:bg-muted/40">
							{#each headerGroup.headers as header (header.id)}
								<Table.Head
									class={header.column.id === 'select'
										? 'w-10'
										: header.column.id === 'actions'
											? 'w-12'
											: header.column.id === 'members' || header.column.id === 'created'
												? 'hidden sm:table-cell'
												: ''}
								>
									{#if !header.isPlaceholder}
										<FlexRender
											content={header.column.columnDef.header}
											context={header.getContext()}
										/>
									{/if}
								</Table.Head>
							{/each}
						</Table.Row>
					{/each}
				</Table.Header>

				<Table.Body>
					{#if groupsQuery.isPending}
						{#each Array(pagination.pageSize > 10 ? 10 : pagination.pageSize) as _, i (i)}
							<Table.Row class="hover:bg-transparent">
								<Table.Cell><Skeleton class="size-4 rounded" /></Table.Cell>
								<Table.Cell>
									<div class="flex items-center gap-3">
										<Skeleton class="size-9 rounded-lg" />
										<div class="space-y-1.5">
											<Skeleton class="h-3.5 w-32" />
											<Skeleton class="h-3 w-40" />
										</div>
									</div>
								</Table.Cell>
								<Table.Cell class="hidden sm:table-cell"><Skeleton class="h-3.5 w-8" /></Table.Cell>
								<Table.Cell class="hidden sm:table-cell"><Skeleton class="h-3.5 w-20" /></Table.Cell>
								<Table.Cell><Skeleton class="ml-auto size-8 rounded" /></Table.Cell>
							</Table.Row>
						{/each}
					{:else if table.getRowModel().rows.length}
						{#each table.getRowModel().rows as row (row.id)}
							<Table.Row data-state={row.getIsSelected() ? 'selected' : undefined}>
								{#each row.getVisibleCells() as cell (cell.id)}
									<Table.Cell
										class={cell.column.id === 'members' || cell.column.id === 'created'
											? 'hidden sm:table-cell'
											: ''}
									>
										<FlexRender content={cell.column.columnDef.cell} context={cell.getContext()} />
									</Table.Cell>
								{/each}
							</Table.Row>
						{/each}
					{:else}
						<Table.Row class="hover:bg-transparent">
							<Table.Cell colspan={columns.length} class="h-32 text-center">
								<div class="text-muted-foreground flex flex-col items-center gap-2">
									<UsersRoundIcon class="size-7 opacity-60" />
									<p class="text-sm">{m.groups_table_empty_state()}</p>
								</div>
							</Table.Cell>
						</Table.Row>
					{/if}
				</Table.Body>
			</Table.Root>
		</div>

		<div class="flex flex-wrap items-center justify-between gap-4">
			<p class="text-muted-foreground text-sm">
				{#if total}
					{m.groups_pagination_showing_prefix()}
					<span class="text-foreground font-medium">{rangeStart}–{rangeEnd}</span>
					{m.groups_pagination_of()}
					<span class="text-foreground font-medium">{total}</span>
				{/if}
			</p>

			<div class="flex items-center gap-4">
				<div class="text-muted-foreground flex items-center gap-2 text-sm">
					<span>{m.groups_rows_per_page()}</span>
					<Select.Root
						type="single"
						value={String(pagination.pageSize)}
						onValueChange={(value) => table.setPageSize(Number(value))}
					>
						<Select.Trigger size="sm" class="w-14">
							{pagination.pageSize}
						</Select.Trigger>

						<Select.Content class="p-1">
							{#each PAGE_SIZES as size (size)}
								<Select.Item value={String(size)} label={String(size)}>{size}</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
				</div>

				<div class="flex items-center gap-2">
					<span class="text-muted-foreground text-sm">
						{m.groups_pagination_page_of({ page: pagination.pageIndex + 1, pageCount })}
					</span>

					<Button
						variant="outline"
						size="icon"
						class="size-8"
						disabled={!table.getCanPreviousPage()}
						onclick={() => table.previousPage()}
					>
						<span class="sr-only">{m.groups_previous_page_sr()}</span>
						<ChevronLeftIcon />
					</Button>
					<Button
						variant="outline"
						size="icon"
						class="size-8"
						disabled={!table.getCanNextPage()}
						onclick={() => table.nextPage()}
					>
						<span class="sr-only">{m.groups_next_page_sr()}</span>
						<ChevronRightIcon />
					</Button>
				</div>
			</div>
		</div>
	{/if}
</div>

<DeleteGroupDialog {tenantId} group={groupToDelete} onOpenChange={() => (groupToDelete = null)} />

<BulkDeleteGroupsDialog
	{tenantId}
	groupIds={selectedIds}
	sample={selectedSample}
	open={bulkDeleteOpen}
	onOpenChange={(value) => (bulkDeleteOpen = value)}
	onDeleted={() => (rowSelection = {})}
/>

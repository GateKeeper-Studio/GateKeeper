import type { ColumnDef } from '@tanstack/table-core';

import { Checkbox } from '$lib/components/ui/checkbox/index.js';
import { renderComponent } from '$lib/components/ui/data-table/index.js';

import type { TenantGroupView } from '$lib/services/admin/groups';
import GroupCell from './group-cell.svelte';
import GroupActions from './group-actions.svelte';
import { formatGroupDate } from './display.js';

export function getGroupColumns(
	tenantId: string,
	onDelete: (group: TenantGroupView) => void
): ColumnDef<TenantGroupView>[] {
	return [
		{
			id: 'select',
			header: ({ table }) =>
				renderComponent(Checkbox, {
					checked: table.getIsAllPageRowsSelected(),
					indeterminate: table.getIsSomePageRowsSelected() && !table.getIsAllPageRowsSelected(),
					onCheckedChange: (value: boolean) => table.toggleAllPageRowsSelected(!!value),
					'aria-label': 'Select all'
				}),
			cell: ({ row }) =>
				renderComponent(Checkbox, {
					checked: row.getIsSelected(),
					onCheckedChange: (value: boolean) => row.toggleSelected(!!value),
					'aria-label': 'Select row'
				}),
			enableSorting: false,
			enableHiding: false
		},
		{
			id: 'group',
			header: 'Group',
			cell: ({ row }) => renderComponent(GroupCell, { group: row.original, tenantId })
		},
		{
			id: 'members',
			header: 'Members',
			cell: ({ row }) => String(row.original.member_count)
		},
		{
			id: 'created',
			header: 'Created',
			cell: ({ row }) => formatGroupDate(row.original.created_at)
		},
		{
			id: 'actions',
			cell: ({ row }) => renderComponent(GroupActions, { group: row.original, tenantId, onDelete })
		}
	];
}

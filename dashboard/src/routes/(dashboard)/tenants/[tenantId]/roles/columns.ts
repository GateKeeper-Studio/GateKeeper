import type { ColumnDef } from '@tanstack/table-core';

import { Checkbox } from '$lib/components/ui/checkbox/index.js';
import { renderComponent } from '$lib/components/ui/data-table/index.js';

import type { TenantRoleView } from '$lib/services/admin/roles';
import RoleCell from './role-cell.svelte';
import RoleActions from './role-actions.svelte';
import { formatRoleDate } from './display.js';

export function getRoleColumns(
	tenantId: string,
	onDelete: (role: TenantRoleView) => void
): ColumnDef<TenantRoleView>[] {
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
			id: 'role',
			header: 'Role',
			cell: ({ row }) => renderComponent(RoleCell, { role: row.original, tenantId })
		},
		{
			id: 'users',
			header: 'Users',
			cell: ({ row }) => String(row.original.user_count)
		},
		{
			id: 'created',
			header: 'Created',
			cell: ({ row }) => formatRoleDate(row.original.created_at)
		},
		{
			id: 'actions',
			cell: ({ row }) => renderComponent(RoleActions, { role: row.original, tenantId, onDelete })
		}
	];
}

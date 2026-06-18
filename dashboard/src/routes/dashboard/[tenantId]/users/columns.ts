import type { ColumnDef } from '@tanstack/table-core';

import { Checkbox } from '$lib/components/ui/checkbox/index.js';
import { renderComponent } from '$lib/components/ui/data-table/index.js';

import type { TenantUserView } from '$lib/services/admin/users';
import UserCell from './user-cell.svelte';
import UserActions from './user-actions.svelte';
import { formatUserDate } from './display.js';

// Column definitions for the users data table. Built by a factory so the
// `tenantId` and the single-row delete handler can be captured in the cell
// renderers.
export function getUserColumns(
	tenantId: string,
	onDelete: (user: TenantUserView) => void
): ColumnDef<TenantUserView>[] {
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
			id: 'user',
			header: 'User',
			cell: ({ row }) => renderComponent(UserCell, { user: row.original, tenantId })
		},
		{
			id: 'joined',
			header: 'Joined',
			cell: ({ row }) => formatUserDate(row.original.created_at)
		},
		{
			id: 'actions',
			cell: ({ row }) => renderComponent(UserActions, { user: row.original, tenantId, onDelete })
		}
	];
}

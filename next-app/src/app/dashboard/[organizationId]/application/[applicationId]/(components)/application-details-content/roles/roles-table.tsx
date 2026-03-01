"use client";

import { useState } from "react";
import { useParams } from "next/navigation";
import { ChevronDown, MoreHorizontal, Search, Trash } from "lucide-react";
import {
  ColumnDef,
  ColumnFiltersState,
  OnChangeFn,
  PaginationState,
  SortingState,
  VisibilityState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Checkbox } from "@/components/ui/checkbox";

import { copy } from "@/lib/utils";

import { NewRoleDialog } from "./new-role-dialog";
import { DeleteRoleDialog } from "./delete-role-dialog";

import { ApplicationRoleItem } from "@/services/dashboard/use-application-roles-swr";
import { deleteApplicationRoleApi } from "@/services/dashboard/delete-application-role";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
import { DataTableFilter } from "@/components/data-table-filter";
import { DataTablePagination } from "@/components/data-table-pagination";
import { Item } from "@/components/ui/item";
import { Spinner } from "@/components/ui/spinner";

export type ApplicationRole = ApplicationRoleItem;
export type RoleTableItem = ApplicationRoleItem;

type Props = {
  items: RoleTableItem[];
  totalCount: number;
  pagination: PaginationState;
  onPaginationChange: OnChangeFn<PaginationState>;
  setItems: (items: RoleTableItem[]) => void;
  addRole: (role: RoleTableItem) => void;
  isLoading: boolean;
};

export function RolesTable({
  items,
  totalCount,
  pagination,
  onPaginationChange,
  setItems,
  addRole,
  isLoading,
}: Props) {
  const { organizationId, applicationId } = useParams() as {
    organizationId: string;
    applicationId: string;
  };

  const [selectedRole, setSelectedRole] = useState<ApplicationRole | null>(
    null,
  );
  const [isDeleteModalOpened, setIsDeleteModalOpened] = useState(false);

  const [sorting, setSorting] = useState<SortingState>([]);
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({});
  const [rowSelection, setRowSelection] = useState({});

  const columns: ColumnDef<ApplicationRole>[] = [
    {
      id: "select",
      header: ({ table }) => (
        <Checkbox
          checked={
            table.getIsAllPageRowsSelected() ||
            (table.getIsSomePageRowsSelected() && "indeterminate")
          }
          onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
          aria-label="Select all"
        />
      ),
      cell: ({ row }) => (
        <Checkbox
          checked={row.getIsSelected()}
          onCheckedChange={(value) => row.toggleSelected(!!value)}
          aria-label="Select row"
        />
      ),
      enableSorting: false,
      enableHiding: false,
    },

    {
      accessorKey: "name",
      header: "Name",
      cell: ({ row }) => (
        <div className="capitalize">{row.getValue("name")}</div>
      ),
    },

    {
      accessorKey: "description",
      header: "Description",
      cell: ({ row }) => (
        <div className="capitalize">{row.getValue("description")}</div>
      ),
    },

    {
      id: "actions",
      enableHiding: false,
      cell: ({ row }) => {
        const role = row.original;

        return (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" className="h-8 w-8 p-0">
                <span className="sr-only">Open menu</span>
                <MoreHorizontal />
              </Button>
            </DropdownMenuTrigger>

            <DropdownMenuContent align="end">
              <DropdownMenuLabel>Actions</DropdownMenuLabel>
              <DropdownMenuItem onClick={() => copy(role.id)}>
                Copy role ID
              </DropdownMenuItem>

              <DropdownMenuSeparator />

              <DropdownMenuItem
                className="text-red-500 font-bold"
                onClick={() => {
                  setSelectedRole(role);
                  setIsDeleteModalOpened(true);
                }}
              >
                Remove Role
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        );
      },
    },
  ];

  const pageCount = Math.ceil(totalCount / pagination.pageSize);

  const table = useReactTable({
    data: items,
    columns,
    pageCount,
    manualPagination: true,
    onPaginationChange,
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    onColumnVisibilityChange: setColumnVisibility,
    onRowSelectionChange: setRowSelection,
    state: {
      pagination,
      sorting,
      columnFilters,
      columnVisibility,
      rowSelection,
    },
  });

  async function deleteSelection() {
    const currentRoles = table
      .getFilteredSelectedRowModel()
      .rows.map((item) => item.original);

    await Promise.all(
      currentRoles.map(async (row) => {
        await deleteApplicationRoleApi(
          { applicationId, organizationId, roleId: row.id },
          { accessToken: "" },
        );
      }),
    );

    table.setRowSelection({});

    setItems(
      items.filter((role) => !currentRoles.some((cr) => cr.id === role.id)),
    );
  }

  if (isLoading) {
    return (
      <Item className="px-0">
        <Spinner /> Loading roles...
      </Item>
    );
  }

  return (
    <>
      <div className="mx-auto w-full">
        <div className="flex items-center pb-4 gap-2">
          <Field className="w-fit">
            <FieldLabel htmlFor="input-group-roles" className="sr-only">
              Search
            </FieldLabel>

            <InputGroup className="max-w-sm">
              <InputGroupInput
                id="input-group-roles"
                placeholder="Search by name"
                value={
                  (table.getColumn("name")?.getFilterValue() as string) ?? ""
                }
                onChange={(e) => {
                  table
                    .getColumn("name")
                    ?.setFilterValue(e.currentTarget.value);
                }}
              />
              <InputGroupAddon>
                <Search />
              </InputGroupAddon>
            </InputGroup>
          </Field>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline">
                Columns <ChevronDown />
              </Button>
            </DropdownMenuTrigger>

            <DropdownMenuContent align="end">
              {table
                .getAllColumns()
                .filter((column) => column.getCanHide())
                .map((column) => {
                  return (
                    <DropdownMenuCheckboxItem
                      key={column.id}
                      className="capitalize"
                      checked={column.getIsVisible()}
                      onCheckedChange={(value) =>
                        column.toggleVisibility(!!value)
                      }
                    >
                      {column.id}
                    </DropdownMenuCheckboxItem>
                  );
                })}
            </DropdownMenuContent>
          </DropdownMenu>

          <DataTableFilter table={table} />

          <div className="flex gap-2 ml-auto">
            {table.getFilteredSelectedRowModel().rows.length > 0 && (
              <Button variant="destructive" onClick={() => deleteSelection()}>
                <Trash />
                Delete selected (
                {table.getFilteredSelectedRowModel().rows.length})
              </Button>
            )}

            <NewRoleDialog addRole={addRole} />
          </div>
        </div>

        <div className="rounded-md border mb-2">
          <Table>
            <TableHeader>
              {table.getHeaderGroups().map((headerGroup) => (
                <TableRow key={headerGroup.id}>
                  {headerGroup.headers.map((header) => {
                    return (
                      <TableHead key={header.id}>
                        {header.isPlaceholder
                          ? null
                          : flexRender(
                              header.column.columnDef.header,
                              header.getContext(),
                            )}
                      </TableHead>
                    );
                  })}
                </TableRow>
              ))}
            </TableHeader>

            <TableBody>
              {table.getRowModel().rows?.length ? (
                table.getRowModel().rows.map((row) => (
                  <TableRow
                    key={row.id}
                    data-state={row.getIsSelected() && "selected"}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <TableCell key={cell.id}>
                        {flexRender(
                          cell.column.columnDef.cell,
                          cell.getContext(),
                        )}
                      </TableCell>
                    ))}
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell
                    colSpan={columns.length}
                    className="h-24 text-center"
                  >
                    No results.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>

        <DataTablePagination table={table} />
      </div>

      <DeleteRoleDialog
        isOpened={isDeleteModalOpened}
        onOpenChange={setIsDeleteModalOpened}
        role={selectedRole}
        removeRole={(role) =>
          setItems(items.filter((item) => item.id !== role.id))
        }
      />
    </>
  );
}

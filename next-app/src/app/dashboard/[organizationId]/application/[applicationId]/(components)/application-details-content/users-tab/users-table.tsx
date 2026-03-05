"use client";

import Link from "next/link";
import { useState } from "react";
import {
  ArrowUpDown,
  ChevronDown,
  MoreHorizontal,
  Plus,
  Search,
  Trash,
} from "lucide-react";
import { useParams } from "next/navigation";
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

import { Button, buttonVariants } from "@/components/ui/button";
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
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";

import { cn, copy } from "@/lib/utils";

import { DeleteUserDialog } from "./delete-user-dialog";
import { IApplication } from "@/services/dashboard/get-application-by-id";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
import { DataTableFilter } from "@/components/data-table-filter";
import { DataTablePagination } from "@/components/data-table-pagination";
import { useOrganizationsContext } from "@/app/dashboard/(contexts)/organizations-context-provider";
import { Item } from "@/components/ui/item";
import { Spinner } from "@/components/ui/spinner";

export type TenantUser = IApplication["users"]["data"][number];

export type UserTableItem = IApplication["users"]["data"][number];

type Props = {
  items: UserTableItem[];
  totalCount: number;
  pagination: PaginationState;
  onPaginationChange: OnChangeFn<PaginationState>;
  setItems: (items: UserTableItem[]) => void;
  isLoading: boolean;
};

export function UsersTable({
  items,
  totalCount,
  pagination,
  onPaginationChange,
  setItems,
  isLoading,
}: Props) {
  const { selectedOrganization } = useOrganizationsContext();

  const [selectedUser, setSelectedUser] = useState<TenantUser | null>(
    null,
  );
  const [isDeleteModalOpened, setIsDeleteModalOpened] = useState(false);

  const [sorting, setSorting] = useState<SortingState>([]);
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({});
  const [rowSelection, setRowSelection] = useState({});

  const { applicationId, organizationId } = useParams() as {
    applicationId: string;
    organizationId: string;
  };

  const columns: ColumnDef<TenantUser>[] = [
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
      accessorKey: "displayName",
      header: "Display Name",
      cell: ({ row }) => (
        <div className="capitalize">{row.getValue("displayName")}</div>
      ),
    },

    {
      accessorKey: "email",
      header: ({ column }) => {
        return (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
          >
            Email
            <ArrowUpDown />
          </Button>
        );
      },
      cell: ({ row }) => (
        <div className="lowercase">{row.getValue("email")}</div>
      ),
    },

    {
      accessorKey: "roles",
      header: "Roles",
      cell: ({ row }) => {
        const roles = row.getValue("roles") as TenantUser["roles"];

        return (
          <div className="flex gap-1">
            {roles?.map((role) => (
              <Badge key={role.id} variant="default" color="green">
                {role.name}
              </Badge>
            ))}
          </div>
        );
      },
    },
    {
      id: "actions",
      enableHiding: false,
      cell: ({ row }) => {
        const user = row.original;

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
              <DropdownMenuItem onClick={() => copy(user.id)}>
                Copy user ID
              </DropdownMenuItem>

              <DropdownMenuSeparator />

              <DropdownMenuItem asChild>
                <Link href={`/dashboard/${organizationId}/users/${user.id}`}>
                  View User
                </Link>
              </DropdownMenuItem>

              <DropdownMenuItem asChild>
                <Link
                  href={`/dashboard/${organizationId}/users/${user.id}/edit-user`}
                >
                  Update User
                </Link>
              </DropdownMenuItem>

              <DropdownMenuItem
                className="text-red-500 font-bold"
                onClick={() => {
                  setSelectedUser(user);
                  setIsDeleteModalOpened(true);
                }}
              >
                Remove User
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
    setIsDeleteModalOpened(true);
  }

  // async function deleteSelection() {
  //   const currentUsers = table
  //     .getFilteredSelectedRowModel()
  //     .rows.map((item) => item.original);

  //   await Promise.all(
  //     currentUsers.map(async (row) => {
  //       await deleteTenantUserApi(
  //         { applicationId, organizationId, userId: row.id },
  //         { accessToken: "" },
  //       );
  //     }),
  //   );

  //   table.setRowSelection({}); // Clear selection

  //   setUsers((state) => state.filter((role) => !currentUsers.includes(role)));
  // }

  if (isLoading) {
    return (
      <Item className="px-0">
        <Spinner /> Loading users...
      </Item>
    );
  }

  return (
    <>
      <div className="mx-auto w-full">
        <div className="flex items-center pb-4 gap-2">
          <Field className="w-fit">
            <FieldLabel htmlFor="input-group-url" className="sr-only">
              Search
            </FieldLabel>

            <InputGroup className="max-w-sm">
              <InputGroupInput
                id="input-group-url"
                placeholder="Search by name"
                value={
                  (table
                    .getColumn("displayName")
                    ?.getFilterValue() as string) ?? ""
                }
                onChange={(e) => {
                  table
                    .getColumn("displayName")
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

            <Link
              href={`/dashboard/${selectedOrganization?.id}/users/create-user`}
              className={cn(buttonVariants({ variant: "default" }), "ml-auto")}
            >
              <Plus /> Add User
            </Link>
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

      <DeleteUserDialog
        isOpened={isDeleteModalOpened}
        onOpenChange={setIsDeleteModalOpened}
        users={table
          .getFilteredSelectedRowModel()
          .rows.map((row) => row.original)}
        removeTableSelection={() => table.setRowSelection({})}
        removeUsers={(users) => {
          setItems(
            items.filter((item) => !users.some((user) => user.id === item.id)),
          );
        }}
      />
    </>
  );
}

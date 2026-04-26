"use client";

import {
  ColumnDef,
  ColumnFiltersState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  SortingState,
  useReactTable,
  VisibilityState,
} from "@tanstack/react-table";
import Link from "next/link";
import { toast } from "sonner";
import { useEffect, useState } from "react";
import { ChevronDown, Copy, MoreHorizontal } from "lucide-react";

import { Checkbox } from "@/components/ui/checkbox";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";

import {
  Tenant,
  useTenantsSWR,
} from "@/services/dashboard/use-tenants-swr";
import { deleteTenantApi } from "@/services/settings/delete-tenant";

import { DeleteTenantDialog } from "./delete-tenant-dialog";

import { copy } from "@/lib/utils";

export function TenantsTable() {
  const { data } = useTenantsSWR({
    accessToken: "fake-token",
  });

  const [selectedTenant, setSelectedTenant] =
    useState<Tenant | null>(null);
  const [isDeleteModalOpened, setIsDeleteModalOpened] = useState(false);

  const [sorting, setSorting] = useState<SortingState>([]);
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({});
  const [rowSelection, setRowSelection] = useState({});
  const [tenants, setTenants] = useState<Tenant[]>([]);

  useEffect(() => {
    setTenants(data || []);
  }, [data]);

  const columns: ColumnDef<Tenant>[] = [
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
        <div className="capitalize w-full">{row.getValue("name")}</div>
      ),
    },
    {
      accessorKey: "id",
      header: "Identification",
      cell: ({ row }) => (
        <div className="capitalize w-full flex justify-between items-center">
          {row.getValue("id")}
          <Button
            variant="outline"
            className="ml-2"
            onClick={() => copy(row.getValue("id"))}
            title="Copy tenant ID"
          >
            <Copy />
          </Button>
        </div>
      ),
    },
    {
      id: "actions",
      enableHiding: false,
      cell: ({ row }) => {
        const tenant = row.original;

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
              <DropdownMenuItem onClick={() => copy(tenant.id)}>
                Copy tenant ID
              </DropdownMenuItem>

              <DropdownMenuSeparator />

              <DropdownMenuItem asChild>
                <Link href={`/dashboard/tenants/${tenant.id}`}>
                  View Tenant
                </Link>
              </DropdownMenuItem>

              <DropdownMenuItem asChild>
                <Link href={`/dashboard/tenants/${tenant.id}/edit`}>
                  Update Tenant
                </Link>
              </DropdownMenuItem>

              <DropdownMenuItem
                className="text-red-500 font-bold"
                onClick={() => {
                  setSelectedTenant(tenant);
                  setIsDeleteModalOpened(true);
                }}
              >
                Remove Tenant
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        );
      },
    },
  ];

  const table = useReactTable({
    data: tenants,
    columns,
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    getCoreRowModel: getCoreRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    onColumnVisibilityChange: setColumnVisibility,
    onRowSelectionChange: setRowSelection,
    state: {
      sorting,
      columnFilters,
      columnVisibility,
      rowSelection,
    },
  });

  async function deleteSelection() {
    const currentTenants = table
      .getFilteredSelectedRowModel()
      .rows.map((item) => item.original);

    await Promise.all(
      currentTenants.map(async (row) => {
        await deleteTenantApi(
          { tenantId: row.id },
          { accessToken: "fake-token" },
        );
      }),
    );

    table.setRowSelection({});

    toast.success("The selected tenants were successfully deleted!");

    setTenants((state) =>
      state.filter(
        (tenant) => !currentTenants.includes(tenant),
      ),
    );
  }

  return (
    <>
      <div className="flex items-center py-4">
        <Input
          placeholder="Filter tenants..."
          value={(table.getColumn("name")?.getFilterValue() as string) ?? ""}
          onInput={(e) =>
            table.getColumn("name")?.setFilterValue(e.currentTarget.value)
          }
          onChange={(e) => {
            table.getColumn("name")?.setFilterValue(e.currentTarget.value);
          }}
          className="max-w-sm"
        />

        <DropdownMenu>
          <Button className="ml-4" asChild>
            <Link href="/dashboard/tenants/create-tenant">
              Add Tenant
            </Link>
          </Button>

          {table.getFilteredSelectedRowModel().rows.length !== 0 && (
            <Button
              type="button"
              variant="destructive"
              onClick={deleteSelection}
              className="ml-4"
            >
              Delete Selection
            </Button>
          )}

          <DropdownMenuTrigger asChild>
            <Button variant="outline" className="ml-auto">
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
      </div>

      <div className="rounded-md border">
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

      <div className="flex items-center justify-end space-x-2 py-4">
        <div className="flex-1 text-sm text-muted-foreground">
          {table.getFilteredSelectedRowModel().rows.length} of{" "}
          {table.getFilteredRowModel().rows.length} row(s) selected.
        </div>

        <div className="space-x-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => table.previousPage()}
            disabled={!table.getCanPreviousPage()}
          >
            Previous
          </Button>

          <Button
            variant="outline"
            size="sm"
            onClick={() => table.nextPage()}
            disabled={!table.getCanNextPage()}
          >
            Next
          </Button>
        </div>
      </div>

      <DeleteTenantDialog
        isOpened={isDeleteModalOpened}
        onOpenChange={setIsDeleteModalOpened}
        tenant={selectedTenant}
        removeTenant={(tenant) =>
          setTenants((state) =>
            state.filter((item) => item.id !== tenant.id),
          )
        }
      />
    </>
  );
}

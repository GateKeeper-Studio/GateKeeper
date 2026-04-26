"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import {
  ArrowUpDown,
  ChevronDown,
  Hammer,
  MoreHorizontal,
  Plus,
  Search,
  ShieldCheck,
  Trash,
} from "lucide-react";
import { useParams, useRouter } from "next/navigation";
import {
  ColumnDef,
  ColumnFiltersState,
  SortingState,
  VisibilityState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
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
import { Input } from "@/components/ui/input";
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
import { DataTablePagination } from "@/components/data-table-pagination";

import { cn, copy } from "@/lib/utils";

import { DataTableFilter } from "@/components/data-table-filter";
import { Item } from "@/components/ui/item";
import { Spinner } from "@/components/ui/spinner";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group";
import { useTenantsContext } from "@/app/dashboard/(contexts)/tenants-context-provider";

export type ApplicationTableItem = any;

type Props = {
  items: ApplicationTableItem[];
  setItems: (items: ApplicationTableItem[]) => void;
  isLoading: boolean;
};

export function ApplicationsTable({ items, isLoading, setItems }: Props) {
  const router = useRouter();
  const { selectedTenant } = useTenantsContext();
  const [isDeleteModalOpened, setIsDeleteModalOpened] = useState(false);

  const [sorting, setSorting] = useState<SortingState>([]);
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({});
  const [rowSelection, setRowSelection] = useState({});

  const { projectId } = useParams() as { projectId: string };

  const columns = useMemo<ColumnDef<ApplicationTableItem>[]>(
    () => [
      {
        id: "select",
        header: ({ table }) => (
          <Checkbox
            onClick={(e) => e.stopPropagation()}
            checked={
              table.getIsAllPageRowsSelected() ||
              (table.getIsSomePageRowsSelected() && "indeterminate")
            }
            onCheckedChange={(value) =>
              table.toggleAllPageRowsSelected(!!value)
            }
            aria-label="Select all"
          />
        ),
        cell: ({ row }) => (
          <Checkbox
            onClick={(e) => e.stopPropagation()}
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
        header: ({ column }) => {
          return (
            <Button
              variant="ghost"
              onClick={() =>
                column.toggleSorting(column.getIsSorted() === "asc")
              }
            >
              Name
              <ArrowUpDown />
            </Button>
          );
        },
        cell: ({ row }) => (
          <div className="capitalize">{row.getValue("name")}</div>
        ),
      },

      {
        accessorKey: "description",
        header: () => {
          return <span>Description</span>;
        },
        cell: ({ row }) => (
          <div className="capitalize">{row.getValue("description")}</div>
        ),
      },

      {
        id: "actions",
        enableHiding: false,
        cell: ({ row }) => {
          const application = row.original;

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
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    copy(application.id);
                  }}
                >
                  Copy ID
                </DropdownMenuItem>

                <DropdownMenuSeparator />

                <DropdownMenuItem asChild>
                  <Link
                    onClick={(e) => e.stopPropagation()}
                    href={`/dashboard/${selectedTenant?.id}/${application.id}`}
                  >
                    View
                  </Link>
                </DropdownMenuItem>

                <DropdownMenuItem asChild>
                  <Link
                    onClick={(e) => e.stopPropagation()}
                    href={`/dashboard/${selectedTenant?.id}/${application.id}/edit-application`}
                  >
                    Edit
                  </Link>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          );
        },
      },
    ],
    [projectId],
  );

  const table = useReactTable({
    data: items,
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
    setIsDeleteModalOpened(true);
  }

  if (isLoading) {
    return (
      <Item className="px-0">
        <Spinner /> Loading activities...
      </Item>
    );
  }

  return (
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
                (table.getColumn("name")?.getFilterValue() as string) ?? ""
              }
              onChange={(e) => {
                table.getColumn("name")?.setFilterValue(e.currentTarget.value);
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
              Delete selected ({table.getFilteredSelectedRowModel().rows.length}
              )
            </Button>
          )}

          <Link
            href={`/dashboard/${selectedTenant?.id}/application/create-application`}
            className={cn(buttonVariants({ variant: "default" }), "ml-auto")}
          >
            <Plus /> Create Application
          </Link>
        </div>
      </div>

      <div className="rounded-md border mb-4 relative w-full overflow-auto max-h-[56vh]">
        <Table>
          <TableHeader className="sticky top-0 z-10 bg-background shadow-sm">
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
                  onClick={() =>
                    router.push(
                      `/dashboard/${selectedTenant?.id}/application/${row.original.id}`,
                    )
                  }
                  className="cursor-pointer"
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

      <DataTablePagination table={table} hasSelection={false} />
    </div>
  );
}

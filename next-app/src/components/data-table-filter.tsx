"use client";

import * as React from "react";
import { Table } from "@tanstack/react-table";
import { Filter, X, Check } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Badge } from "@/components/ui/badge";
import type { FilterValue } from "@/lib/utils";

interface DataTableFilterProps<TData> {
  table: Table<TData>;
}

export function DataTableFilter<TData>({ table }: DataTableFilterProps<TData>) {
  const [open, setOpen] = React.useState(false);

  // Estado local do formulário de filtro
  const [selectedColumn, setSelectedColumn] = React.useState<string>("");
  const [operator, setOperator] =
    React.useState<FilterValue["operator"]>("contains");
  const [searchValue, setSearchValue] = React.useState("");

  // Pega apenas colunas que podem ser filtradas e têm um accessorKey (dados reais)
  const filterableColumns = React.useMemo(() => {
    return table
      .getAllColumns()
      .filter(
        (column) =>
          column.getCanFilter() && typeof column.accessorFn !== "undefined",
      );
  }, [table]);

  // Aplica o filtro na tabela
  const handleApply = () => {
    if (!selectedColumn) return;

    table.getColumn(selectedColumn)?.setFilterValue({
      value: searchValue,
      operator: operator,
    } as FilterValue);

    setOpen(false);
  };

  // Limpa o filtro de uma coluna específica
  const clearFilter = (columnId: string) => {
    table.getColumn(columnId)?.setFilterValue(undefined);
  };

  // Reseta o form interno ao abrir
  React.useEffect(() => {
    if (open) {
      setSearchValue("");
      // Opcional: manter o operador anterior
    }
  }, [open]);

  // Verifica se há filtros ativos para mostrar visualmente no botão
  const activeFilters = table.getState().columnFilters;

  return (
    <div className="flex items-center gap-2">
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button variant="outline" className="border-dashed">
            <Filter className="mr-2 h-4 w-4" />
            Filter
            {activeFilters.length > 0 && (
              <Badge
                variant="secondary"
                className="ml-2 rounded-sm px-1 font-normal lg:hidden"
              >
                {activeFilters.length}
              </Badge>
            )}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-[300px] p-4 bg-background" align="start">
          <div className="space-y-4">
            <h4 className="font-medium leading-none">Advanced Filter</h4>

            {/* 1. Seleção da Coluna */}
            <div className="space-y-2 w-full">
              <Label>Column</Label>
              <Select value={selectedColumn} onValueChange={setSelectedColumn}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="Select column..." />
                </SelectTrigger>
                <SelectContent>
                  {filterableColumns.map((col) => (
                    <SelectItem key={col.id} value={col.id}>
                      {/* Tenta pegar o Header como string, ou usa o ID formatado */}
                      {typeof col.columnDef.header === "string"
                        ? col.columnDef.header
                        : col.id.replace(/_/g, " ").toUpperCase()}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* 2. Seleção do Operador */}
            <div className="flex gap-2">
              <div className="flex-1 space-y-2">
                <Label>Operator</Label>
                <Select
                  value={operator}
                  onValueChange={(val: any) => setOperator(val)}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="contains">Contains</SelectItem>
                    <SelectItem value="equals">Equals To</SelectItem>
                    <SelectItem value="startsWith">Starts With</SelectItem>
                    <SelectItem value="endsWith">Ends With</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>

            {/* 3. Valor */}
            <div className="space-y-2 w-full">
              <Label>Value</Label>
              <Input
                placeholder="Type to filter..."
                value={searchValue}
                onChange={(e) => setSearchValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleApply();
                }}
              />
            </div>

            <div className="flex justify-end gap-2 pt-2">
              <Button variant="ghost" size="sm" onClick={() => setOpen(false)}>
                Cancel
              </Button>
              <Button size="sm" onClick={handleApply}>
                Apply
              </Button>
            </div>
          </div>
        </PopoverContent>
      </Popover>

      {/* Lista de filtros ativos (Badges com botão de remover) */}
      {activeFilters.map((filter) => {
        const column = table.getColumn(filter.id);
        const filterValue = filter.value as FilterValue;

        // Se for string simples (do search global), ignoramos ou tratamos diferente
        if (typeof filterValue !== "object") return null;

        return (
          <Badge
            key={filter.id}
            variant="secondary"
            className="h-8 px-2 lg:px-3"
          >
            <span className="font-bold mr-1">
              {typeof column?.columnDef.header === "string"
                ? column.columnDef.header
                : filter.id}
              :
            </span>
            <span className="text-muted-foreground mr-1">
              {filterValue.operator === "equals" ? "=" : "~"}
            </span>
            {filterValue.value}
            <Button
              variant="ghost"
              size="icon"
              className="ml-2 h-4 w-4 p-0 hover:bg-transparent"
              onClick={() => clearFilter(filter.id)}
            >
              <X className="h-3 w-3" />
            </Button>
          </Badge>
        );
      })}

      {activeFilters.length > 0 && (
        <Button
          variant="ghost"
          onClick={() => table.resetColumnFilters()}
          className="h-8 px-2 lg:px-3"
        >
          Reset
          <X className="ml-2 h-4 w-4" />
        </Button>
      )}
    </div>
  );
}

import { clsx, type ClassValue } from "clsx";
import { toast } from "sonner";
import { twMerge } from "tailwind-merge";
import { FilterFn } from "@tanstack/react-table";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function copy(value: string) {
  navigator.clipboard.writeText(value);
  toast.info(`"${value}" was copied to clipboard!`);
}

export function formatDate(date: Date) {
  // Format to "MM/DD/YYYY at HH:MM"
  return new Intl.DateTimeFormat("en-US", {
    month: "2-digit",
    day: "2-digit",
    year: "numeric",
    hour: "numeric",
    minute: "numeric",
  }).format(date);
}

/**
 * Return the value of a cookie by its name.
 * @param name Nome do cookie
 * @returns Cookie value if it exists
 */
export function getCookieValue(name: string): string | null {
  if (typeof document === "undefined") return null;

  const cookies = document.cookie.split(";").map((cookie) => cookie.trim());
  if (!cookies.length) return null;
  for (const cookie of cookies) {
    const [key, ...rest] = cookie.split("=");
    if (key === name) {
      return decodeURIComponent(rest.join("="));
    }
  }

  return null;
}

// Tipo para o nosso filtro customizado
export type FilterValue = {
  value: string;
  operator: "contains" | "equals" | "startsWith" | "endsWith";
};

// Essa função deve ser colocada na definição da coluna (columns.tsx)
// para as colunas que você quer que aceitem esse filtro avançado
export const smartFilterFn: FilterFn<any> = (
  row,
  columnId,
  filterValue: FilterValue | string,
) => {
  const cellValue = String(row.getValue(columnId)).toLowerCase();

  // Suporte a filtro simples (pesquisa global) ou filtro complexo
  const needle =
    typeof filterValue === "string" ? filterValue : filterValue.value;
  const operator =
    typeof filterValue === "string" ? "contains" : filterValue.operator;

  const search = needle.toLowerCase();

  if (!search) return true; // Se vazio, mostra tudo

  switch (operator) {
    case "contains":
      return cellValue.includes(search);
    case "equals":
      return cellValue === search;
    case "startsWith":
      return cellValue.startsWith(search);
    case "endsWith":
      return cellValue.endsWith(search);
    default:
      return cellValue.includes(search);
  }
};

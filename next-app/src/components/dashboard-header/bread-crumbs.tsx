import Link from "next/link";
import { Fragment } from "react";

import {
  Breadcrumb,
  BreadcrumbEllipsis,
  BreadcrumbItem,
  BreadcrumbList,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SidebarTrigger } from "@/components/ui/sidebar";

// Utilitário simples para juntar classes condicionalmente (caso não tenha o cn/clsx)
function cn(...classes: (string | undefined | boolean)[]) {
  return classes.filter(Boolean).join(" ");
}

type Props = {
  items: {
    name: string;
    path?: string;
  }[];
  disableSideBar?: boolean;
  maxItems?: number;
};

export function Breadcrumbs({
  items,
  disableSideBar = false,
  maxItems = 4,
}: Props) {
  // 1. Atualizamos a função para receber "isLast"
  const renderItemLink = (
    item: { name: string; path?: string },
    isLast: boolean,
  ) => {
    return item.path ? (
      <Link
        href={item.path}
        className={cn(
          "transition-colors hover:text-foreground",
          isLast
            ? "font-bold text-foreground"
            : "font-normal text-muted-foreground",
        )}
      >
        {item.name}
      </Link>
    ) : (
      <span
        className={cn("text-foreground", isLast ? "font-bold" : "font-normal")}
      >
        {item.name}
      </span>
    );
  };

  return (
    <div className="flex items-center gap-4">
      {!disableSideBar && <SidebarTrigger />}

      <Breadcrumb>
        <BreadcrumbList>
          {/* CENÁRIO 1: Quantidade de itens menor ou igual ao limite */}
          {items.length <= maxItems ? (
            items.map((item, index) => {
              const isLast = index === items.length - 1;

              return (
                <Fragment key={index}>
                  <BreadcrumbItem>
                    {/* Passamos o isLast aqui */}
                    {renderItemLink(item, isLast)}
                  </BreadcrumbItem>
                  {!isLast && <BreadcrumbSeparator />}
                </Fragment>
              );
            })
          ) : (
            <>
              {/* Primeiro Item (Nunca é o último nesse cenário) */}
              <BreadcrumbItem>{renderItemLink(items[0], false)}</BreadcrumbItem>
              <BreadcrumbSeparator />

              {/* Dropdown (Itens do meio) */}
              <BreadcrumbItem>
                <DropdownMenu>
                  <DropdownMenuTrigger className="flex items-center gap-1 hover:text-foreground">
                    <BreadcrumbEllipsis className="h-4 w-4" />
                    <span className="sr-only">Toggle menu</span>
                  </DropdownMenuTrigger>

                  <DropdownMenuContent align="start">
                    {items.slice(1, -2).map((item, index) => (
                      <DropdownMenuItem key={index} asChild>
                        {item.path ? (
                          <Link href={item.path}>
                            {index + 1}. {item.name}
                          </Link>
                        ) : (
                          <span>
                            {index + 1}. {item.name}
                          </span>
                        )}
                      </DropdownMenuItem>
                    ))}
                  </DropdownMenuContent>
                </DropdownMenu>
              </BreadcrumbItem>
              <BreadcrumbSeparator />

              {/* Últimos 2 itens */}
              {items.slice(-2).map((item, index, array) => {
                // Aqui o index é relativo ao slice (0 ou 1)
                // O último item deste array é o último item geral
                const isLast = index === array.length - 1;

                return (
                  <Fragment key={index}>
                    <BreadcrumbItem>
                      {renderItemLink(item, isLast)}
                    </BreadcrumbItem>
                    {!isLast && <BreadcrumbSeparator />}
                  </Fragment>
                );
              })}
            </>
          )}
        </BreadcrumbList>
      </Breadcrumb>
    </div>
  );
}

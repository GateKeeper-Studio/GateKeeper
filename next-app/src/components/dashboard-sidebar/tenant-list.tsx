"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { ChevronsUpDown, GalleryVerticalEnd, Plus } from "lucide-react";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "../ui/dropdown-menu";
import { Skeleton } from "../ui/skeleton";
import { SidebarMenuButton, useSidebar } from "../ui/sidebar";

import {
  Tenant,
  useTenantsSWR,
} from "@/services/dashboard/use-tenants-swr";

export function TenantList() {
  const { isMobile } = useSidebar();
  const router = useRouter();

  const selectedTenantId = useParams()?.tenantId;

  const { data, isLoading } = useTenantsSWR({
    accessToken: "",
  });

  useEffect(() => {
    if (data && data.length > 0) {
      if (selectedTenantId) {
        const org = data.find(
          (tenant) => tenant.id === selectedTenantId,
        );

        if (org) {
          setSelectedTenant(org);
          cookieStore.set("tenant", org.name);
        }

        return;
      }

      setSelectedTenant(data[0]);
      cookieStore.set("tenant", data[0].name);
    }
  }, [data, selectedTenantId]);

  const [selectedTenant, setSelectedTenant] =
    useState<Tenant | null>(null);

  function handleSelect(tenant: Tenant) {
    setSelectedTenant(tenant);

    cookieStore.set("tenant", tenant.name);
    router.push(`/dashboard/${tenant.id}`);
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <SidebarMenuButton
          size="lg"
          className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
        >
          <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
            <GalleryVerticalEnd className="size-4" />
          </div>

          <div className="grid flex-1 text-left text-sm leading-tight">
            <span className="truncate font-semibold">
              {selectedTenant?.name || "Select an tenant"}
            </span>
            <span className="truncate text-xs">Enterprise</span>
          </div>

          <ChevronsUpDown className="ml-auto" />
        </SidebarMenuButton>
      </DropdownMenuTrigger>

      <DropdownMenuContent
        className="w-[--radix-dropdown-menu-trigger-width] min-w-56 rounded-lg"
        align="start"
        side={isMobile ? "bottom" : "right"}
        sideOffset={4}
      >
        <DropdownMenuLabel className="text-xs text-muted-foreground">
          Tenants
        </DropdownMenuLabel>

        {isLoading && (
          <>
            <Skeleton className="h-[28px] w-[7rem]" />
            <Skeleton className="h-[28px] w-[5rem]" />
            <Skeleton className="h-[28px] w-[12rem]" />
          </>
        )}

        {data?.map((tenant, i) => (
          <DropdownMenuItem
            key={tenant.id}
            className="gap-2 p-2"
            onClick={handleSelect.bind(null, tenant)}
          >
            <div className="flex size-6 items-center justify-center rounded-sm border">
              <GalleryVerticalEnd className="size-4 shrink-0" />
            </div>
            {tenant.name}{" "}
            <DropdownMenuShortcut>⌘{i + 1}</DropdownMenuShortcut>
          </DropdownMenuItem>
        ))}

        <DropdownMenuSeparator />

        <DropdownMenuItem
          className="gap-2 p-2"
          onClick={() =>
            router.push("/dashboard/tenants/create-tenant")
          }
        >
          <div className="flex size-6 items-center justify-center rounded-md border bg-background">
            <Plus className="size-4" />
          </div>

          <div className="font-medium text-muted-foreground">
            Add tenant
          </div>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

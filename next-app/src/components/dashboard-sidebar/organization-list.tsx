"use client";

import { useParams, useRouter } from "next/navigation";
import { useEffect, useState } from "react";
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
  Organization,
  useOrganizationsSWR,
} from "@/services/dashboard/use-organizations-swr";

export function OrganizationList() {
  const { isMobile } = useSidebar();
  const router = useRouter();

  const selectedOrganizationId = useParams()?.organizationId; 
  
  const { data, isLoading } = useOrganizationsSWR({
    accessToken: "",
  });

  useEffect(() => {
    if (data && data.length > 0) {

      if (selectedOrganizationId) {
        const org = data.find(
          (organization) => organization.id === selectedOrganizationId
        );

        if (org) {
          setSelectedOrganization(org);
          cookieStore.set("organization", org.name);
        }
        
        return;
      }
      
      setSelectedOrganization(data[0]);
      cookieStore.set("organization", data[0].name);
    }
  }, [data, selectedOrganizationId]);

  const [selectedOrganization, setSelectedOrganization] =
    useState<Organization | null>(null);

  function handleSelect(organization: Organization) {
    setSelectedOrganization(organization);

    cookieStore.set("organization", organization.name);
    router.push(`/dashboard/${organization.id}/application`);
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
              {selectedOrganization?.name || "Select an organization"}
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
          Organizations
        </DropdownMenuLabel>

        {isLoading && (
          <>
            <Skeleton className="h-[28px] w-[7rem]" />
            <Skeleton className="h-[28px] w-[5rem]" />
            <Skeleton className="h-[28px] w-[12rem]" />
          </>
        )}

        {data?.map((organization, i) => (
          <DropdownMenuItem
            key={organization.id}
            className="gap-2 p-2"
            onClick={handleSelect.bind(null, organization)}
          >
            <div className="flex size-6 items-center justify-center rounded-sm border">
              <GalleryVerticalEnd className="size-4 shrink-0" />
            </div>
            {organization.name}{" "}
            <DropdownMenuShortcut>⌘{i + 1}</DropdownMenuShortcut>
          </DropdownMenuItem>
        ))}

        <DropdownMenuSeparator />

        <DropdownMenuItem
          className="gap-2 p-2"
          onClick={() => router.push("/settings/organizations")}
        >
          <div className="flex size-6 items-center justify-center rounded-md border bg-background">
            <Plus className="size-4" />
          </div>

          <div className="font-medium text-muted-foreground">
            Add organization
          </div>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

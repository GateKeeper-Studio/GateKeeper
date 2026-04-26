"use client";

import { useState } from "react";
import { PaginationState } from "@tanstack/react-table";

import { ErrorAlert } from "@/components/error-alert";
import { SectionTitle } from "@/components/section-title";

import { UsersTable, UserTableItem } from "./users-table";
import { useTenantUsersSWR } from "@/services/dashboard/use-tenant-users-swr";
import { useTenantsContext } from "@/app/dashboard/(contexts)/tenants-context-provider";

export function UsersTab() {
  const { selectedTenant } = useTenantsContext();

  const [pagination, setPagination] = useState<PaginationState>({
    pageIndex: 0,
    pageSize: 10,
  });

  const { data, error, isLoading, mutate } = useTenantUsersSWR(
    {
      tenantId: selectedTenant?.id || "",
      page: pagination.pageIndex + 1,
      pageSize: pagination.pageSize,
    },
    { accessToken: "" },
  );

  if (error) {
    return (
      <div className="flex m-4 w-full">
        <ErrorAlert
          message={
            error.response?.data.message || "Failed on trying to fetch users"
          }
          title={error.response?.data.title || "An Error Occurred"}
        />
      </div>
    );
  }

  return (
    <section className="flex flex-col gap-4 w-full">
      <SectionTitle>Users</SectionTitle>

      <span className="text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Users belong to the tenant and can be assigned roles per
        application. Manage all tenant users here.
      </span>

      <UsersTable
        items={data?.data || []}
        totalCount={data?.totalCount || 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        setItems={(items: UserTableItem[]) =>
          mutate(data ? { ...data, data: items } : undefined, {
            revalidate: false,
          })
        }
        isLoading={isLoading}
      />
    </section>
  );
}

"use client";

import { useState } from "react";
import { PaginationState } from "@tanstack/react-table";

import { ErrorAlert } from "@/components/error-alert";
import { SectionTitle } from "@/components/section-title";

import { RolesTable, RoleTableItem } from "./roles-table";
import { useApplicationRolesSWR } from "@/services/dashboard/use-application-roles-swr";
import { useApplicationContext } from "../../../(contexts)/application-context-provider";
import { useTenantsContext } from "@/app/dashboard/(contexts)/tenants-context-provider";

export function Roles() {
  const { selectedTenant } = useTenantsContext();
  const { application } = useApplicationContext();

  const [pagination, setPagination] = useState<PaginationState>({
    pageIndex: 0,
    pageSize: 10,
  });

  const { data, error, isLoading, mutate } = useApplicationRolesSWR(
    {
      tenantId: selectedTenant?.id || "",
      applicationId: application?.id || "",
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
            error.response?.data.message || "Failed on trying to fetch roles"
          }
          title={error.response?.data.title || "An Error Occurred"}
        />
      </div>
    );
  }

  return (
    <section className="flex flex-col gap-4 w-full">
      <SectionTitle>Roles</SectionTitle>

      <span className="text-sm tracking-tight text-gray-600 dark:text-gray-300">
        Roles define permissions and access control for users in this
        application. Assign roles to users to manage their access.
      </span>

      <RolesTable
        items={data?.data || []}
        totalCount={data?.totalCount || 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        setItems={(items: RoleTableItem[]) =>
          mutate(data ? { ...data, data: items } : undefined, {
            revalidate: false,
          })
        }
        addRole={(role: RoleTableItem) =>
          mutate(
            data
              ? {
                  ...data,
                  data: [role, ...data.data],
                  totalCount: data.totalCount + 1,
                }
              : undefined,
            { revalidate: false },
          )
        }
        isLoading={isLoading}
      />
    </section>
  );
}

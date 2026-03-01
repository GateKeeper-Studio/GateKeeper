"use client";

import { useState } from "react";
import { PaginationState } from "@tanstack/react-table";

import { ErrorAlert } from "@/components/error-alert";
import { SectionTitle } from "@/components/section-title";

import { UsersTable, UserTableItem } from "./users-table";
import { useApplicationUsersSWR } from "@/services/dashboard/use-application-users-swr";
import { useApplicationContext } from "../../../(contexts)/application-context-provider";
import { useOrganizationsContext } from "@/app/dashboard/(contexts)/organizations-context-provider";

export function UsersTab() {
  const { selectedOrganization } = useOrganizationsContext();
  const { application } = useApplicationContext();

  const [pagination, setPagination] = useState<PaginationState>({
    pageIndex: 0,
    pageSize: 10,
  });

  const { data, error, isLoading, mutate } = useApplicationUsersSWR(
    {
      organizationId: selectedOrganization?.id || "",
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
        Users are the individuals who have access to this application. They can
        be assigned roles and permissions to control their access.
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
